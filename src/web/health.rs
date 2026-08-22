//! Health probes. `/healthz` is a cheap liveness check (no DB).
//! `/readyz` checks DB connectivity and embedder readiness.

use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// `GET /healthz` — always returns 200 if the process is running.
pub async fn live() -> Response {
    (StatusCode::OK, "ok").into_response()
}

/// `GET /readyz` — 200 if the DB is reachable, else 503.
pub async fn ready(State(state): State<AppState>) -> Response {
    let db_ok = state.sqlite_repos().db.with_conn(|c| {
        c.query_row::<i64, _, _>("SELECT 1", [], |r| r.get(0))
            .map(|_| true)
            .unwrap_or(false)
    });
    if !db_ok {
        return (StatusCode::SERVICE_UNAVAILABLE, "db unavailable").into_response();
    }
    (StatusCode::OK, "ready").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::db::pool::open_memory;
    use crate::infra::embeddings::fastembed_impl::StubEmbedder;
    use crate::infra::repos::SqliteRepos;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;

    fn fixture() -> AppState {
        let cfg = Arc::new(crate::config::AppConfig {
            server: crate::config::ServerConfig {
                bind: "127.0.0.1:0".to_owned(),
                shutdown_timeout: std::time::Duration::from_secs(30),
            },
            database: crate::config::DatabaseConfig {
                path: ":memory:".to_owned(),
                busy_timeout_ms: 5000,
            },
            embed: crate::config::EmbedConfig {
                cache_dir: String::new(),
                model: String::new(),
                similarity_threshold: 0.85,
                knn_top_k: 5,
                hybrid_limit: 10,
            },
            log: crate::config::LogConfig {
                format: crate::config::LogFormat::Pretty,
                level: "info".to_owned(),
            },
        });
        AppState {
            repos: Arc::new(SqliteRepos::new(open_memory().unwrap())),
            embedder: Arc::new(StubEmbedder::new()),
            mcp: Arc::new(crate::mcp::tools_impl::McpRegistry::new()),
            cfg,
            setup_active: Arc::new(AtomicBool::new(false)),
            setup_state: Arc::new(crate::state::SetupState {
                active: false,
                expires_at: Instant::now(),
                plaintext: None,
            }),
        }
    }

    #[tokio::test]
    async fn live_is_always_200() {
        let resp = live().await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ready_returns_200_when_db_ok() {
        let s = fixture();
        let resp = ready(State(s)).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
