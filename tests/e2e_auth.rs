mod common;
use common::TestApp;

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
        .header("authorization", "Bearer not-a-token")
        .body(axum::body::Body::from("{}"))
        .unwrap();
    let resp = app.oneshot(req).await;
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn oauth_access_token_reaches_mcp() {
    let app = TestApp::new();
    let access_token = app.seed_admin_access_token().await;
    let req = TestApp::mcp_request("initialize", serde_json::json!({}));
    let req = with_bearer(req, &access_token);
    let resp = app.oneshot(req).await;
    assert_eq!(resp.status(), 200);
}

fn with_bearer(
    mut req: axum::http::Request<axum::body::Body>,
    token: &str,
) -> axum::http::Request<axum::body::Body> {
    req.headers_mut()
        .insert(axum::http::header::AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
    req
}
