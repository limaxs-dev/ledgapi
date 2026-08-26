//! MCP corner-case suite: JSON-RPC envelope edge cases, per-tool error
//! paths, scope enforcement, and cross-project isolation — all through
//! the real router with the in-memory test app.

mod common;

use common::TestApp;
use http_body_util::BodyExt;
use ledgapi::domain::auth::{OAuthClient, OAuthToken, Role, UserCreate};
use ledgapi::domain::ports::Repos;
use ledgapi::domain::project::{ProjectCreate, ProjectSlug};
use ledgapi::infra::auth::{password, token};
use serde_json::{Value, json};
use time::OffsetDateTime;

fn with_bearer(
    mut req: axum::http::Request<axum::body::Body>,
    plaintext: &str,
) -> axum::http::Request<axum::body::Body> {
    use axum::http::header::AUTHORIZATION;
    req.headers_mut().insert(AUTHORIZATION, format!("Bearer {plaintext}").parse().unwrap());
    req
}

async fn response_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Send a `tools/call` and return the full JSON-RPC frame.
async fn call(app: &TestApp, token: &str, name: &str, arguments: Value) -> Value {
    let resp = app
        .oneshot(with_bearer(
            TestApp::mcp_request("tools/call", json!({"name": name, "arguments": arguments})),
            token,
        ))
        .await;
    assert_eq!(resp.status(), 200, "JSON-RPC errors ride on HTTP 200");
    response_json(resp).await
}

/// Seed app + super-admin token + one project (`slug = "api"`).
async fn setup() -> (TestApp, String) {
    let app = TestApp::new();
    let token = app.seed_admin_access_token().await;
    (app, token)
}

const BASE_ARGS: &str = r#"{"project_slug":"api","method":"GET","path":"/users","summary":"List users","response_schema":{"type":"object"},"force":true}"#;

// ---------------------------------------------------------------------------
// Envelope / transport corner cases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn malformed_json_body_returns_parse_error_frame() {
    let (app, token) = setup().await;
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(axum::body::Body::from("{not json"))
        .unwrap();
    let resp = app.oneshot(with_bearer(req, &token)).await;
    assert_eq!(resp.status(), 200);
    let v = response_json(resp).await;
    assert_eq!(v["error"]["code"], -32700);
    assert!(v["result"].is_null());
}

#[tokio::test]
async fn unknown_method_returns_method_not_found() {
    let (app, token) = setup().await;
    let resp =
        app.oneshot(with_bearer(TestApp::mcp_request("resources/list", json!({})), &token)).await;
    let v = response_json(resp).await;
    assert_eq!(v["error"]["code"], -32601);
    assert!(v["error"]["message"].as_str().unwrap().contains("resources/list"));
}

#[tokio::test]
async fn notification_without_id_gets_204_no_content() {
    let (app, token) = setup().await;
    let body = serde_json::to_vec(&json!({
        "jsonrpc": "2.0", "method": "notifications/initialized"
    }))
    .unwrap();
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body))
        .unwrap();
    let resp = app.oneshot(with_bearer(req, &token)).await;
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn id_null_treated_as_notification_gets_204() {
    let (app, token) = setup().await;
    let body = serde_json::to_vec(&json!({
        "jsonrpc": "2.0", "id": null, "method": "notifications/initialized"
    }))
    .unwrap();
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body))
        .unwrap();
    let resp = app.oneshot(with_bearer(req, &token)).await;
    // Per handler: no id → no JSON-RPC response → HTTP 204.
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn missing_method_field_is_invalid_request() {
    let (app, token) = setup().await;
    let body = serde_json::to_vec(&json!({"jsonrpc": "2.0", "id": 7})).unwrap();
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body))
        .unwrap();
    let resp = app.oneshot(with_bearer(req, &token)).await;
    let v = response_json(resp).await;
    assert_eq!(v["error"]["code"], -32600);
}

#[tokio::test]
async fn non_object_params_are_accepted_for_methods_that_ignore_them() {
    let (app, token) = setup().await;
    // params as an array — dispatch ignores params for initialize/tools/list.
    let resp =
        app.oneshot(with_bearer(TestApp::mcp_request("initialize", json!([1, 2])), &token)).await;
    assert_eq!(resp.status(), 200);
    let v = response_json(resp).await;
    assert_eq!(v["result"]["serverInfo"]["name"], "ledgapi");
}

#[tokio::test]
async fn string_and_numeric_ids_echo_back() {
    let (app, token) = setup().await;
    for id in [json!("req-abc"), json!(42), json!(-1), json!(1.5)] {
        let body = serde_json::to_vec(&json!({
            "jsonrpc": "2.0", "id": id, "method": "initialize", "params": {}
        }))
        .unwrap();
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body))
            .unwrap();
        let resp = app.oneshot(with_bearer(req, &token)).await;
        let v = response_json(resp).await;
        assert_eq!(v["id"], id);
    }
}

#[tokio::test]
async fn oversized_body_over_limit_is_rejected_with_400() {
    let (app, token) = setup().await;
    // MAX_BODY_BYTES is 4 MiB; send a payload larger than that.
    let big = vec![b'a'; 4 * 1024 * 1024 + 1];
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(big))
        .unwrap();
    let resp = app.oneshot(with_bearer(req, &token)).await;
    assert_eq!(resp.status(), 400);
}

// ---------------------------------------------------------------------------
// tools/call argument corner cases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_call_missing_name_is_invalid_params() {
    let (app, token) = setup().await;
    let v = call(&app, &token, "", json!({})).await; // empty name → unknown tool path? No: missing name
    // Actually send params without "name" at all:
    let resp = app
        .oneshot(with_bearer(TestApp::mcp_request("tools/call", json!({"arguments": {}})), &token))
        .await;
    let v2 = response_json(resp).await;
    assert_eq!(v2["error"]["code"], -32602);
    drop(v); // the first call returns unknown tool for empty-string name
}

#[tokio::test]
async fn tools_call_unknown_tool_is_invalid_params_with_data() {
    let (app, token) = setup().await;
    let resp = app
        .oneshot(with_bearer(
            TestApp::mcp_request("tools/call", json!({"name": "no_such_tool", "arguments": {}})),
            &token,
        ))
        .await;
    let v = response_json(resp).await;
    assert_eq!(v["error"]["code"], -32602);
    assert_eq!(v["error"]["data"]["code"], "validation_failed");
    assert!(v["error"]["message"].as_str().unwrap().contains("unknown tool"));
}

#[tokio::test]
async fn invalid_project_slug_chars_rejected_before_lookup() {
    let (app, token) = setup().await;
    let args: Value = serde_json::from_str(
        BASE_ARGS.replace("\"project_slug\":\"api\"", "\"project_slug\":\"Bad Slug!\"").as_str(),
    )
    .unwrap();
    let v = call(&app, &token, "create_contract", args).await;
    assert_eq!(v["error"]["code"], -32602);
    assert_eq!(v["error"]["data"]["field"], "project_slug");
}

#[tokio::test]
async fn nonexistent_project_returns_not_found_data_code() {
    let (app, token) = setup().await;
    let v = call(&app, &token, "list_contracts", json!({"project_slug": "ghost"})).await;
    assert_eq!(v["error"]["code"], -32602);
    assert_eq!(v["error"]["data"]["code"], "not_found");
}

#[tokio::test]
async fn create_contract_rejects_bad_method() {
    let (app, token) = setup().await;
    app.state.repos.projects(); // touch to silence unused warnings
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();

    let args: Value = serde_json::from_str(
        BASE_ARGS.replace("\"method\":\"GET\"", "\"method\":\"BREW\"").as_str(),
    )
    .unwrap();
    let v = call(&app, &token, "create_contract", args).await;
    assert_eq!(v["error"]["code"], -32602);
    assert_eq!(v["error"]["data"]["field"], "method");
}

#[tokio::test]
async fn create_contract_rejects_missing_response_schema() {
    let (app, token) = setup().await;
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();
    let args = json!({
        "project_slug": "api",
        "method": "GET",
        "path": "/users",
        "summary": "List users",
        "force": true
    });
    let v = call(&app, &token, "create_contract", args).await;
    assert_eq!(v["error"]["code"], -32602);
    assert_eq!(v["error"]["data"]["field"], "args");
}

#[tokio::test]
async fn create_contract_summary_too_long_is_validation_error() {
    let (app, token) = setup().await;
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();
    let args = json!({
        "project_slug": "api",
        "method": "GET",
        "path": "/users",
        "summary": "x".repeat(301),
        "response_schema": {"type": "object"},
        "force": true
    });
    let v = call(&app, &token, "create_contract", args).await;
    assert_eq!(v["error"]["code"], -32602);
    let field = v["error"]["data"]["field"].as_str().unwrap_or_default();
    assert!(field == "args" || field == "summary" || field == "path", "unexpected field: {field}");
}

#[tokio::test]
async fn get_contract_by_id_non_uuid_returns_not_found() {
    let (app, token) = setup().await;
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();
    let v = call(
        &app,
        &token,
        "get_contract_by_id",
        json!({"project_slug": "api", "contract_id": "not-a-uuid"}),
    )
    .await;
    assert_eq!(v["error"]["code"], -32602);
    assert_eq!(v["error"]["data"]["code"], "not_found");
}

#[tokio::test]
async fn get_contract_by_id_uuid_v4_rejected_not_just_v7() {
    let (app, token) = setup().await;
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();
    // A valid UUID but NOT v7 → Id::parse rejects it.
    let v4 = "550e8400-e29b-41d4-a716-446655440000";
    let v =
        call(&app, &token, "get_contract_by_id", json!({"project_slug": "api", "contract_id": v4}))
            .await;
    assert_eq!(v["error"]["code"], -32602);
    assert_eq!(v["error"]["data"]["code"], "not_found");
}

#[tokio::test]
async fn delete_contract_unknown_but_valid_id_returns_not_found() {
    let (app, token) = setup().await;
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();
    // A syntactically valid UUIDv7 that doesn't exist.
    let v7ish = "01900000-0000-7000-8000-000000000000";
    let v =
        call(&app, &token, "delete_contract", json!({"project_slug": "api", "contract_id": v7ish}))
            .await;
    assert_eq!(v["error"]["code"], -32602);
    assert_eq!(v["error"]["data"]["code"], "not_found");
}

#[tokio::test]
async fn list_contracts_status_filter_case_sensitive_rejects_uppercase() {
    let (app, token) = setup().await;
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();
    let v = call(&app, &token, "list_contracts", json!({"project_slug": "api", "status": "DRAFT"}))
        .await;
    assert_eq!(v["error"]["code"], -32602);
    assert_eq!(v["error"]["data"]["field"], "status");
}

#[tokio::test]
async fn search_contract_invalid_mode_is_validation_error() {
    let (app, token) = setup().await;
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();
    let v = call(
        &app,
        &token,
        "search_contract",
        json!({"project_slug": "api", "query": "users", "search_mode": "fuzzy"}),
    )
    .await;
    assert_eq!(v["error"]["code"], -32602);
    assert_eq!(v["error"]["data"]["field"], "search_mode");
}

#[tokio::test]
async fn export_openapi_empty_project_still_succeeds_with_valid_yaml() {
    let (app, token) = setup().await;
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();
    let v = call(&app, &token, "export_openapi", json!({"project_slug": "api"})).await;
    let out = &v["result"]["content"][0]["json"];
    assert_eq!(v["result"]["isError"], false);
    assert!(out["yaml"].as_str().unwrap().contains("openapi"));
    assert!(out["download_url"].as_str().unwrap().contains(".yml"));
}

// ---------------------------------------------------------------------------
// Scope enforcement (viewer vs editor capabilities)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn viewer_scope_cannot_call_write_tools() {
    let (app, _) = setup().await;
    // Viewer user with read-only token.
    let user = app
        .state
        .repos
        .users()
        .create(&UserCreate {
            username: "viewer".to_owned(),
            password_hash: password::hash_password("correct horse battery staple").unwrap(),
            role: Role::Viewer,
        })
        .await
        .unwrap();
    let client = OAuthClient {
        client_id: "viewer-client".to_owned(),
        client_name: "Viewer client".to_owned(),
        redirect_uris: vec!["http://localhost/cb".to_owned()],
        created_at: OffsetDateTime::now_utc(),
    };
    app.state.repos.oauth().register_client(&client).await.unwrap();
    let raw = token::generate();
    app.state
        .repos
        .oauth()
        .create_access_token(&OAuthToken {
            token_hash: token::sha256_hex(&raw),
            client_id: client.client_id.clone(),
            user_id: user.id,
            scope: vec![
                "ledgapi:read".to_owned(),
                "ledgapi:write".to_owned(), // over-asked at token level...
            ],
            expires_at: OffsetDateTime::now_utc() + time::Duration::hours(1),
            revoked_at: None,
        })
        .await
        .unwrap();
    // Middleware intersects token scopes with role scopes, so the
    // effective principal scopes must NOT contain ledgapi:write.

    // list_projects is read-only → allowed.
    let v = call(&app, &raw, "list_projects", json!({})).await;
    assert_eq!(v["result"]["isError"], false);

    // create_project requires write → forbidden even though token asked for it.
    let resp = app
        .oneshot(with_bearer(
            TestApp::mcp_request(
                "tools/call",
                json!({"name": "create_project", "arguments": {"slug": "vproj", "name": "V"}}),
            ),
            &raw,
        ))
        .await;
    let v = response_json(resp).await;
    assert_eq!(v["error"]["code"], -32603);
    assert_eq!(v["error"]["data"]["code"], "forbidden");

    // delete_contract on a nonexistent project: the dispatcher resolves
    // project_slug BEFORE the tool's scope check runs, so this surfaces as
    // not_found rather than forbidden (recorded as a QA observation — the
    // scope check should ideally run first).
    let resp = app
        .oneshot(with_bearer(
            TestApp::mcp_request(
                "tools/call",
                json!({"name": "delete_contract", "arguments": {"project_slug": "x", "contract_id": "01900000-0000-7000-8000-000000000000"}}),
            ),
            &raw,
        ))
        .await;
    let v = response_json(resp).await;
    assert_eq!(v["error"]["data"]["code"], "not_found");

    // On an EXISTING project the scope check fires → forbidden.
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("real").unwrap(),
            name: "Real".to_owned(),
            description: None,
        })
        .await
        .unwrap();
    let resp = app
        .oneshot(with_bearer(
            TestApp::mcp_request(
                "tools/call",
                json!({"name": "delete_contract", "arguments": {"project_slug": "real", "contract_id": "01900000-0000-7000-8000-000000000000"}}),
            ),
            &raw,
        ))
        .await;
    let v = response_json(resp).await;
    assert_eq!(v["error"]["data"]["code"], "forbidden");
}

#[tokio::test]
async fn expired_token_rejected_as_invalid() {
    let app = TestApp::new();
    let user = app
        .state
        .repos
        .users()
        .create(&UserCreate {
            username: "expired".to_owned(),
            password_hash: password::hash_password("correct horse battery staple").unwrap(),
            role: Role::SuperAdmin,
        })
        .await
        .unwrap();
    let client = OAuthClient {
        client_id: "exp-client".to_owned(),
        client_name: "Exp client".to_owned(),
        redirect_uris: vec!["http://localhost/cb".to_owned()],
        created_at: OffsetDateTime::now_utc(),
    };
    app.state.repos.oauth().register_client(&client).await.unwrap();
    let raw = token::generate();
    app.state
        .repos
        .oauth()
        .create_access_token(&OAuthToken {
            token_hash: token::sha256_hex(&raw),
            client_id: client.client_id,
            user_id: user.id,
            scope: vec!["ledgapi:read".to_owned(), "ledgapi:write".to_owned()],
            expires_at: OffsetDateTime::now_utc() - time::Duration::hours(1), // expired
            revoked_at: None,
        })
        .await
        .unwrap();
    // Middleware maps expired tokens to AuthInvalid → HTTP 403
    // (observation: RFC 6750 would suggest 401; recorded as QA note).
    let resp = app.oneshot(with_bearer(TestApp::mcp_request("initialize", json!({})), &raw)).await;
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn inactive_user_token_rejected_as_invalid() {
    let app = TestApp::new();
    let user = app
        .state
        .repos
        .users()
        .create(&UserCreate {
            username: "gone".to_owned(),
            password_hash: password::hash_password("correct horse battery staple").unwrap(),
            role: Role::SuperAdmin,
        })
        .await
        .unwrap();
    // Deactivate via repo update (no dedicated deactivate port).
    let mut inactive = user.clone();
    inactive.active = false;
    app.state.repos.users().update(&inactive).await.unwrap();
    let client = OAuthClient {
        client_id: "inact-client".to_owned(),
        client_name: "Inact".to_owned(),
        redirect_uris: vec!["http://localhost/cb".to_owned()],
        created_at: OffsetDateTime::now_utc(),
    };
    app.state.repos.oauth().register_client(&client).await.unwrap();
    let raw = token::generate();
    app.state
        .repos
        .oauth()
        .create_access_token(&OAuthToken {
            token_hash: token::sha256_hex(&raw),
            client_id: client.client_id,
            user_id: user.id,
            scope: vec!["ledgapi:read".to_owned()],
            expires_at: OffsetDateTime::now_utc() + time::Duration::hours(1),
            revoked_at: None,
        })
        .await
        .unwrap();
    let resp = app.oneshot(with_bearer(TestApp::mcp_request("initialize", json!({})), &raw)).await;
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn revoked_token_rejected_as_invalid() {
    let app = TestApp::new();
    let user = app
        .state
        .repos
        .users()
        .create(&UserCreate {
            username: "revoked".to_owned(),
            password_hash: password::hash_password("correct horse battery staple").unwrap(),
            role: Role::SuperAdmin,
        })
        .await
        .unwrap();
    let client = OAuthClient {
        client_id: "revo-client".to_owned(),
        client_name: "Revo".to_owned(),
        redirect_uris: vec!["http://localhost/cb".to_owned()],
        created_at: OffsetDateTime::now_utc(),
    };
    app.state.repos.oauth().register_client(&client).await.unwrap();
    let raw = token::generate();
    let tok = OAuthToken {
        token_hash: token::sha256_hex(&raw),
        client_id: client.client_id,
        user_id: user.id,
        scope: vec!["ledgapi:read".to_owned()],
        expires_at: OffsetDateTime::now_utc() + time::Duration::hours(1),
        revoked_at: None,
    };
    app.state.repos.oauth().create_access_token(&tok).await.unwrap();
    app.state
        .repos
        .oauth()
        .revoke_access_token(&tok.token_hash, OffsetDateTime::now_utc())
        .await
        .unwrap();
    let resp = app.oneshot(with_bearer(TestApp::mcp_request("initialize", json!({})), &raw)).await;
    assert_eq!(resp.status(), 403);
}

// ---------------------------------------------------------------------------
// Cross-project isolation + happy-path sanity through MCP
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_of_other_project_is_invisible_through_mcp() {
    let (app, token) = setup().await;
    let pa = app
        .state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("other").unwrap(),
            name: "Other".to_owned(),
            description: None,
        })
        .await
        .unwrap();

    // Create a contract in project "api".
    let c = app
        .state
        .repos
        .contracts()
        .create(
            pa.id,
            None,
            &serde_json::from_str::<ledgapi::domain::contract::ContractCreate>(
                r#"{
                    "method":"GET","path":"/secret","summary":"s",
                    "response_schema":{"type":"object"}
                }"#,
            )
            .unwrap(),
        )
        .await
        .unwrap();

    // Lookup via the other project must 404 even though the id exists globally.
    let v = call(
        &app,
        &token,
        "get_contract_by_id",
        json!({"project_slug": "other", "contract_id": c.id.to_string()}),
    )
    .await;
    assert_eq!(v["error"]["data"]["code"], "not_found");
}

#[tokio::test]
async fn similar_found_warning_comes_back_as_success_result() {
    let (app, token) = setup().await;
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();

    let mk_args = |force: bool| {
        json!({
            "project_slug": "api",
            "method": "GET",
            "path": "/users",
            "summary": "List users",
            "response_schema": {"type": "object"},
            "force": force
        })
    };

    let first = call(&app, &token, "create_contract", mk_args(true)).await;
    assert_eq!(first["result"]["content"][0]["json"]["status"], "created");

    // Identical method+path+summary without force → the stub embedder gives
    // an identical vector, so SimilarFound fires as a SUCCESSFUL result
    // (isError=false) before the UNIQUE constraint can.
    let second = call(&app, &token, "create_contract", mk_args(false)).await;
    assert_eq!(second["result"]["isError"], false);
    let payload = &second["result"]["content"][0]["json"];
    assert_eq!(payload["status"], "warning_similar_found");
    assert!(!payload["similar_contracts"].as_array().unwrap().is_empty());

    // force=true with a DIFFERENT path but same summary creates anyway.
    let mut third_args = mk_args(true);
    third_args["path"] = json!("/users-v2");
    let third = call(&app, &token, "create_contract", third_args).await;
    assert_eq!(third["result"]["content"][0]["json"]["status"], "created");
}

#[tokio::test]
async fn full_crud_cycle_via_mcp_tools() {
    let (app, token) = setup().await;
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();

    // create
    let created = call(
        &app,
        &token,
        "create_contract",
        json!({
            "project_slug": "api", "method": "post", "path": "/orders",
            "summary": "Create order", "response_schema": {"type":"object"},
            "group_name": "orders-group", "tags": ["billing"],
            "status": "draft", "force": true
        }),
    )
    .await;
    let cid = created["result"]["content"][0]["json"]["contract_id"].as_str().unwrap().to_owned();

    // group was implicitly created and shows up in list_groups
    let groups = call(&app, &token, "list_groups", json!({"project_slug": "api"})).await;
    let gnames: Vec<&str> = groups["result"]["content"][0]["json"]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|g| g["name"].as_str())
        .collect();
    assert!(gnames.contains(&"orders-group"));

    // lowercase method was normalized; get_contract_by_id returns the raw
    // contract (no group_name field — that's only hydrated for list/search).
    let got = call(
        &app,
        &token,
        "get_contract_by_id",
        json!({"project_slug": "api", "contract_id": cid}),
    )
    .await;
    let contract = &got["result"]["content"][0]["json"];
    assert_eq!(contract["method"], "POST");
    assert_eq!(contract["status"], "draft");

    // update
    let upd = call(
        &app,
        &token,
        "update_contract",
        json!({"project_slug": "api", "contract_id": cid, "summary": "Create order v2", "status": "stable"}),
    )
    .await;
    assert_eq!(upd["result"]["content"][0]["json"]["status"], "updated");

    // filter by status finds it
    let listed =
        call(&app, &token, "list_contracts", json!({"project_slug": "api", "status": "stable"}))
            .await;
    let ids: Vec<&str> = listed["result"]["content"][0]["json"]["contracts"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["id"].as_str())
        .collect();
    assert!(ids.contains(&cid.as_str()));

    // search exact finds it
    let found = call(
        &app,
        &token,
        "search_contract",
        json!({"project_slug": "api", "query": "/orders", "search_mode": "exact"}),
    )
    .await;
    let sids: Vec<&str> = found["result"]["content"][0]["json"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["id"].as_str())
        .collect();
    assert!(sids.contains(&cid.as_str()));

    // delete
    let del =
        call(&app, &token, "delete_contract", json!({"project_slug": "api", "contract_id": cid}))
            .await;
    assert_eq!(del["result"]["content"][0]["json"]["status"], "deleted");

    // second delete of same id → not found
    let del2 =
        call(&app, &token, "delete_contract", json!({"project_slug": "api", "contract_id": cid}))
            .await;
    assert_eq!(del2["error"]["data"]["code"], "not_found");
}
