mod common;
use common::TestApp;
use ledgapi::domain::ports::TokenRepo;
use ledgapi::infra::auth::token;

#[tokio::test]
async fn bearer_missing_returns_401() {
    let app = TestApp::new();
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(axum::body::Body::from("{}"))
        .unwrap();
    let resp = app.oneshot(req).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn bearer_invalid_returns_403() {
    let app = TestApp::new();
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header(
            "authorization",
            "Bearer 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdff",
        )
        .body(axum::body::Body::from("{}"))
        .unwrap();
    let resp = app.oneshot(req).await;
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn bearer_valid_reaches_mcp() {
    let app = TestApp::new();
    // Issue a token.
    let plaintext = token::generate();
    let hash = token::sha256_hex(&plaintext);
    app.state.repos.tokens.insert(&hash, Some("test")).await.unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {plaintext}"))
        .body(axum::body::Body::from(
            serde_json::to_vec(&serde_json::json!({
                "jsonrpc":"2.0","id":1,"method":"initialize","params":{}
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await;
    assert_eq!(resp.status(), 200);
}
