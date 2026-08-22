mod common;
use common::TestApp;
use ledgapi::config::{AppConfig, DatabaseConfig, EmbedConfig, LogConfig, LogFormat, ServerConfig};
use ledgapi::infra::auth::token;
use ledgapi::state::{AppState, SetupState};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

/// Build an app where `setup_active=true` and a fresh plaintext is in memory.
fn boot_with_active_setup(plaintext: &str) -> TestApp {
    let mut app = TestApp::new();
    // Override setup state.
    let cfg = Arc::new(AppConfig {
        server: ServerConfig {
            bind: "127.0.0.1:0".into(),
            shutdown_timeout: Duration::from_secs(1),
        },
        database: DatabaseConfig { path: ":memory:".into(), busy_timeout_ms: 1000 },
        embed: EmbedConfig {
            cache_dir: String::new(),
            model: String::new(),
            similarity_threshold: 0.85,
            knn_top_k: 5,
            hybrid_limit: 10,
        },
        log: LogConfig { format: LogFormat::Pretty, level: "warn".into() },
    });
    app.state = AppState {
        repos: app.state.repos.clone(),
        embedder: app.state.embedder.clone(),
        mcp: app.state.mcp.clone(),
        cfg,
        setup_active: Arc::new(AtomicBool::new(true)),
        setup_state: Arc::new(SetupState {
            active: true,
            expires_at: Instant::now() + Duration::from_mins(5),
            plaintext: Some(plaintext.to_owned()),
        }),
        db: app.state.db.clone(),
    };
    app.router = ledgapi::web::router::router(app.state.clone());
    app
}

#[tokio::test]
async fn setup_page_shows_token_when_active() {
    let token_hex = token::generate();
    let app = boot_with_active_setup(&token_hex);
    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/setup")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await;
    assert_eq!(resp.status(), 200);
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let body = std::str::from_utf8(&bytes).unwrap();
    assert!(body.contains(&token_hex));
}

#[tokio::test]
async fn setup_page_returns_410_when_inactive() {
    let app = TestApp::new(); // setup_active defaults to false
    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/setup")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await;
    assert_eq!(resp.status(), 410);
}

#[tokio::test]
async fn setup_page_returns_410_after_ttl() {
    let mut app = TestApp::new();
    let cfg = Arc::new(AppConfig {
        server: ServerConfig {
            bind: "127.0.0.1:0".into(),
            shutdown_timeout: Duration::from_secs(1),
        },
        database: DatabaseConfig { path: ":memory:".into(), busy_timeout_ms: 1000 },
        embed: EmbedConfig {
            cache_dir: String::new(),
            model: String::new(),
            similarity_threshold: 0.85,
            knn_top_k: 5,
            hybrid_limit: 10,
        },
        log: LogConfig { format: LogFormat::Pretty, level: "warn".into() },
    });
    app.state = AppState {
        repos: app.state.repos.clone(),
        embedder: app.state.embedder.clone(),
        mcp: app.state.mcp.clone(),
        cfg,
        setup_active: Arc::new(AtomicBool::new(true)),
        setup_state: Arc::new(SetupState {
            active: true,
            expires_at: Instant::now().checked_sub(Duration::from_secs(1)).unwrap(), // expired
            plaintext: Some("anyway".into()),
        }),
        db: app.state.db.clone(),
    };
    app.router = ledgapi::web::router::router(app.state.clone());
    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/setup")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await;
    assert_eq!(resp.status(), 410);
}
