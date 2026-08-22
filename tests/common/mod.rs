//! Shared test helpers for integration tests.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, Response};
use std::sync::Arc;
use tower::ServiceExt;

use ledgapi::infra::db::pool::open_memory;
use ledgapi::infra::embeddings::fastembed_impl::StubEmbedder;
use ledgapi::infra::repos::SqliteRepos;
use ledgapi::state::AppState;

/// In-process application backed by in-memory repos and a stub embedder.
#[derive(Clone)]
#[allow(dead_code)]
pub struct TestApp {
    pub state: AppState,
    pub router: Router,
}

impl TestApp {
    /// Build a fresh app.
    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn new() -> Self {
        let db = open_memory().expect("open memory");
        let repos = SqliteRepos::new(db);
        let embedder: Arc<dyn ledgapi::domain::ports::Embedder> = Arc::new(StubEmbedder::new());
        let state = AppState::for_tests(repos, embedder);
        let router = ledgapi::web::router::router(state.clone());
        Self { state, router }
    }

    /// Send a request through the router.
    pub async fn oneshot(&self, request: Request<Body>) -> Response<Body> {
        self.router.clone().oneshot(request).await.expect("test_app oneshot failed")
    }

    /// Helper: JSON-RPC request body.
    #[must_use]
    #[allow(dead_code)]
    pub fn mcp_request(method: &str, params: serde_json::Value) -> Request<Body> {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": method, "params": params,
        });
        Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    }
}

impl Default for TestApp {
    fn default() -> Self {
        Self::new()
    }
}
