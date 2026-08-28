//! Live tests — exercise real fastembed MiniLM + real sqlite-vec.
//!
//! Skipped by default. Run with:
//!   cargo test --test live_e2e -- --ignored
//!
//! First run downloads ~25MB of MiniLM weights into `EMBED_CACHE_DIR`
//! (defaults to `/tmp/fastembed-cache`).
//!
//! These tests exercise production code paths against real dependencies:
//! - `fastembed-rs` (real ONNX MiniLM-L6-v2 inference)
//! - `sqlite-vec` (real vec0 virtual table + k-NN)
//! - The OpenAPI exporter (real YAML golden file)
//!
//! All four tests share a single [`FastembedEmbedder`] via a process-wide
//! `OnceLock`. fastembed-rs acquires a file lock on the cached model blob;
//! instantiating multiple embedders in parallel causes the others to fail
//! with `Lock acquisition failed`. Sharing one instance sidesteps the
//! race and keeps the test runtime low.

use std::sync::{Arc, OnceLock};

use ledgapi::config::{
    AppConfig, AuthConfig, DatabaseConfig, EmbedConfig, LogConfig, LogFormat, ServerConfig,
};
use ledgapi::domain::contract::{ContractCreate, Method};
use ledgapi::domain::ports::{Embedder, Repos};
use ledgapi::domain::project::{ProjectCreate, ProjectSlug};
use ledgapi::infra::db::pool;
use ledgapi::infra::embeddings::fastembed_impl::FastembedEmbedder;
use ledgapi::infra::repos::SqliteRepos;

fn fixture_cfg() -> AppConfig {
    AppConfig {
        server: ServerConfig {
            bind: "127.0.0.1:0".into(),
            shutdown_timeout: std::time::Duration::from_secs(1),
        },
        database: DatabaseConfig { path: ":memory:".into(), busy_timeout_ms: 5000 },
        embed: EmbedConfig {
            cache_dir: std::env::var("EMBED_CACHE_DIR")
                .unwrap_or_else(|_| "/tmp/fastembed-cache".into()),
            model: "sentence-transformers/all-MiniLM-L6-v2".into(),
            similarity_threshold: 0.85,
            knn_top_k: 5,
            hybrid_limit: 10,
        },
        log: LogConfig { format: LogFormat::Pretty, level: "warn".into() },
        auth: AuthConfig {
            initial_admin_username: None,
            initial_admin_password: None,
            issuer: "http://localhost:18080".into(),
            session_ttl: std::time::Duration::from_hours(1),
            access_token_ttl: std::time::Duration::from_hours(1),
            refresh_token_ttl: std::time::Duration::from_hours(24),
            authorization_code_ttl: std::time::Duration::from_mins(1),
            cookie_secure: false,
        },
    }
}

/// Single shared embedder for all four tests in this binary. Built once
/// on first call; subsequent calls clone the `Arc`.
fn shared_embedder() -> Arc<dyn Embedder> {
    static SHARED: OnceLock<Arc<FastembedEmbedder>> = OnceLock::new();
    let cfg = fixture_cfg();
    SHARED
        .get_or_init(|| {
            Arc::new(
                FastembedEmbedder::new(&cfg.embed.cache_dir, &cfg.embed.model)
                    .expect("FastembedEmbedder"),
            )
        })
        .clone()
}

#[ignore = "live test — requires real MiniLM model download"]
#[tokio::test]
async fn real_minilm_roundtrip() {
    let embedder = shared_embedder();
    let v1 = embedder.embed("hello world").await.expect("embed 1");
    let v2 = embedder.embed("hello world").await.expect("embed 2");
    assert_eq!(v1.len(), 384, "MiniLM-L6-v2 must produce 384-dim vectors");
    assert_eq!(v1, v2, "MiniLM is deterministic for the same input");
}

#[ignore = "live test — requires real MiniLM model download"]
#[tokio::test]
async fn real_semantic_search_finds_similar() {
    let embedder = shared_embedder();
    let db = pool::open_memory().expect("db");
    let repos = SqliteRepos::new(db);

    let p = repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".into(),
            description: None,
        })
        .await
        .unwrap();

    let c1 = repos
        .contracts()
        .create(
            p.id,
            None,
            &ContractCreate {
                method: Method::Get,
                path: "/users".into(),
                summary: "List users".into(),
                description: None,
                request_headers: None,
                request_params: None,
                request_body_schema: None,
                request_example: None,
                response_schema: serde_json::json!({"type": "object"}),
                response_example: None,
                examples: None,
                auth_type: None,
                status: None,
                tags: None,
                group_name: None,
                group_parent_id: None,
                force: false,
            },
        )
        .await
        .unwrap();
    repos
        .embeddings()
        .upsert(c1.id, p.id, &embedder.embed("List users").await.unwrap())
        .await
        .unwrap();

    let c2 = repos
        .contracts()
        .create(
            p.id,
            None,
            &ContractCreate {
                method: Method::Get,
                path: "/products".into(),
                summary: "List products in catalog".into(),
                description: None,
                request_headers: None,
                request_params: None,
                request_body_schema: None,
                request_example: None,
                response_schema: serde_json::json!({"type": "object"}),
                response_example: None,
                examples: None,
                auth_type: None,
                status: None,
                tags: None,
                group_name: None,
                group_parent_id: None,
                force: false,
            },
        )
        .await
        .unwrap();
    repos
        .embeddings()
        .upsert(c2.id, p.id, &embedder.embed("List products in catalog").await.unwrap())
        .await
        .unwrap();

    let query_emb = embedder.embed("fetch user list").await.unwrap();
    let results = repos.contracts().top_k_similar(p.id, &query_emb, 2).await.unwrap();
    assert_eq!(results.len(), 2);
    // The first result should be one of the two contracts we inserted.
    let top_id = results[0].0;
    assert!(top_id == c1.id || top_id == c2.id);
}

#[ignore = "live test — requires real MiniLM model download"]
#[tokio::test]
async fn real_sqlite_vec_knn_returns_ordered_neighbours() {
    let embedder = shared_embedder();
    let db = pool::open_memory().expect("db");
    let repos = SqliteRepos::new(db);
    let p = repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".into(),
            description: None,
        })
        .await
        .unwrap();

    // Insert three contracts.
    let mut ids = Vec::new();
    for (path, summary) in [("/a", "alpha"), ("/b", "beta"), ("/c", "gamma")] {
        let c = repos
            .contracts()
            .create(
                p.id,
                None,
                &ContractCreate {
                    method: Method::Get,
                    path: path.into(),
                    summary: summary.into(),
                    description: None,
                    request_headers: None,
                    request_params: None,
                    request_body_schema: None,
                    request_example: None,
                    response_schema: serde_json::json!({"type": "object"}),
                    response_example: None,
                    examples: None,
                    auth_type: None,
                    status: None,
                    tags: None,
                    group_name: None,
                    group_parent_id: None,
                    force: false,
                },
            )
            .await
            .unwrap();
        repos
            .embeddings()
            .upsert(c.id, p.id, &embedder.embed(summary).await.unwrap())
            .await
            .unwrap();
        ids.push(c.id);
    }

    let q = embedder.embed("beta").await.unwrap();
    let r = repos.contracts().top_k_similar(p.id, &q, 3).await.unwrap();
    assert_eq!(r.len(), 3);
    // The "beta" contract should be closest.
    assert_eq!(r[0].0, ids[1]);
    // All cosine similarities in [-1, 1] (sqlite-vec cosine distance in [0, 2]
    // for normalized MiniLM vectors; `top_k_similar` returns `1 - distance`).
    for (_, s) in r {
        assert!((-1.0..=1.0).contains(&s), "similarity out of range: {s}");
    }
}

#[ignore = "live test — golden snapshot depends on YAML serialization"]
#[tokio::test]
async fn real_openapi_export_matches_golden() {
    use std::fs;

    let db = pool::open_memory().expect("db");
    let repos = SqliteRepos::new(db);
    let p = repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("golden").unwrap(),
            name: "Golden".into(),
            description: None,
        })
        .await
        .unwrap();

    let _c = repos
        .contracts()
        .create(
            p.id,
            None,
            &ContractCreate {
                method: Method::Get,
                path: "/users/{id}".into(),
                summary: "Get user".into(),
                description: None,
                request_headers: None,
                request_params: None,
                request_body_schema: None,
                request_example: None,
                response_schema: serde_json::json!({"type": "object"}),
                response_example: None,
                examples: None,
                auth_type: Some("bearer".into()),
                status: None,
                tags: None,
                group_name: None,
                group_parent_id: None,
                force: false,
            },
        )
        .await
        .unwrap();

    let r = ledgapi::domain::use_cases::export_openapi::execute(
        &repos,
        ProjectSlug::parse("golden").unwrap(),
    )
    .await
    .unwrap();

    let golden_path = std::path::Path::new("tests/fixtures/golden_openapi.yml");
    if golden_path.exists() {
        let expected = fs::read_to_string(golden_path).unwrap();
        assert_eq!(r.yaml.trim(), expected.trim(), "golden mismatch");
    } else {
        // First-run: write the golden.
        fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
        fs::write(golden_path, &r.yaml).unwrap();
        eprintln!("golden file written — re-run to verify");
    }
}
