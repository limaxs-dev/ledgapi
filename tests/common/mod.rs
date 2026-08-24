//! Shared test helpers for integration tests.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, Response};
use std::sync::Arc;
use tower::ServiceExt;

use ledgapi::domain::auth::{OAuthClient, OAuthToken, Role, UserCreate};
use ledgapi::domain::ports::Repos;
use ledgapi::infra::auth::{password, token};
use ledgapi::infra::db::pool::open_memory;
use ledgapi::infra::embeddings::fastembed_impl::StubEmbedder;
use ledgapi::infra::repos::SqliteRepos;
use ledgapi::state::AppState;
use time::OffsetDateTime;

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

    #[allow(dead_code)]
    pub async fn seed_admin_access_token(&self) -> String {
        let user = self
            .state
            .repos
            .users()
            .create(&UserCreate {
                username: "admin".to_owned(),
                password_hash: password::hash_password("correct horse battery staple").unwrap(),
                role: Role::SuperAdmin,
            })
            .await
            .unwrap();
        let client = OAuthClient {
            client_id: "test-client".to_owned(),
            client_name: "Test client".to_owned(),
            redirect_uris: vec!["http://localhost/callback".to_owned()],
            created_at: OffsetDateTime::now_utc(),
        };
        self.state.repos.oauth().register_client(&client).await.unwrap();
        let raw = token::generate();
        self.state
            .repos
            .oauth()
            .create_access_token(&OAuthToken {
                token_hash: token::sha256_hex(&raw),
                client_id: client.client_id,
                user_id: user.id,
                scope: vec![
                    "ledgapi:read".to_owned(),
                    "ledgapi:write".to_owned(),
                    "ledgapi:admin".to_owned(),
                ],
                expires_at: OffsetDateTime::now_utc() + time::Duration::hours(1),
                revoked_at: None,
            })
            .await
            .unwrap();
        raw
    }

    #[allow(dead_code)]
    pub async fn seed_admin_session(&self) -> (String, String) {
        let user = self.state.repos.users().find_by_username("admin").await.unwrap().unwrap();
        let raw_session = token::generate();
        let raw_csrf = token::generate();
        let now = OffsetDateTime::now_utc();
        self.state
            .repos
            .sessions()
            .create(&ledgapi::domain::auth::Session {
                token_hash: token::sha256_hex(&raw_session),
                user_id: user.id,
                csrf_token_hash: token::sha256_hex(&raw_csrf),
                expires_at: now + time::Duration::hours(1),
                revoked_at: None,
            })
            .await
            .unwrap();
        (raw_session, raw_csrf)
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
