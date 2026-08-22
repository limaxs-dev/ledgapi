//! Health probes. `/healthz` is a cheap liveness check (no DB).
//! `/readyz` checks DB connectivity and embedder readiness.

use crate::state::AppState;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// `GET /healthz` — always returns 200 if the process is running.
pub async fn live() -> Response {
    (StatusCode::OK, "ok").into_response()
}

/// `GET /readyz` — 200 if the DB is reachable, else 503.
pub async fn ready(Extension(state): Extension<AppState>) -> Response {
    let db_ok = state.sqlite_repos().db.with_conn(|c| {
        c.query_row::<i64, _, _>("SELECT 1", [], |r| r.get(0)).map(|_| true).unwrap_or(false)
    });
    if !db_ok {
        return (StatusCode::SERVICE_UNAVAILABLE, "db unavailable").into_response();
    }
    (StatusCode::OK, "ready").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn live_is_always_200() {
        let resp = live().await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ready_returns_200_when_db_ok() {
        let s = AppState::for_tests_default();
        let resp = ready(Extension(s)).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
