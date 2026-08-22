mod common;
use common::TestApp;
use ledgapi::domain::ports::TokenRepo;
use ledgapi::infra::auth::token;
use serde_json::json;

#[tokio::test]
async fn initialize_advertises_protocol_and_capabilities() {
    let app = TestApp::new();
    // Insert a token so the bearer-auth middleware lets the request through.
    let plaintext = token::generate();
    let hash = token::sha256_hex(&plaintext);
    app.state.repos.tokens.insert(&hash, Some("test")).await.unwrap();
    let req = TestApp::mcp_request("initialize", json!({}));
    let req = with_bearer(req, &plaintext);
    let resp = app.oneshot(req).await;
    assert_eq!(resp.status(), 200);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["result"]["serverInfo"]["name"], "ledgapi");
    assert!(v["result"]["capabilities"]["tools"].is_object());
}

#[tokio::test]
async fn tools_list_advertises_all_10_tools() {
    let app = TestApp::new();
    let plaintext = token::generate();
    let hash = token::sha256_hex(&plaintext);
    app.state.repos.tokens.insert(&hash, Some("test")).await.unwrap();
    let req = TestApp::mcp_request("tools/list", json!({}));
    let req = with_bearer(req, &plaintext);
    let resp = app.oneshot(req).await;
    assert_eq!(resp.status(), 200);
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let tools = v["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for expected in [
        "create_contract",
        "create_project",
        "delete_contract",
        "export_openapi",
        "get_contract_by_id",
        "list_contracts",
        "list_groups",
        "list_projects",
        "search_contract",
        "update_contract",
    ] {
        assert!(names.contains(&expected), "missing tool: {expected}");
    }
}

/// Add an `Authorization: Bearer <plaintext>` header to the request.
fn with_bearer(
    req: axum::http::Request<axum::body::Body>,
    plaintext: &str,
) -> axum::http::Request<axum::body::Body> {
    use axum::http::header::AUTHORIZATION;
    let mut req = req;
    req.headers_mut().insert(AUTHORIZATION, format!("Bearer {plaintext}").parse().unwrap());
    req
}

#[tokio::test]
async fn mcp_without_bearer_returns_401() {
    let app = TestApp::new();
    let body = serde_json::to_vec(&json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{}
    }))
    .unwrap();
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body))
        .unwrap();
    let resp = app.oneshot(req).await;
    assert_eq!(resp.status(), 401);
}
