//! Regression backfill for QA-hunter findings that predate automated
//! coverage. Each test maps 1:1 to a finding id recorded in
//! `.qa-hunter/data/findings.jsonl`:
//!
//! - API-002 — `list_contracts` with an unknown group must 404 without creating it
//! - API-003 — `search_contract` with an unknown group must 404 without creating it
//! - API-004 — `list_contracts` with an empty `group_name` means "no filter"
//! - API-006 — updating a concurrently-deleted contract returns NotFound (no silent success)
//! - API-008 — OpenAPI YAML export serves an `attachment` Content-Disposition
//! - UI-001 — project page and MCP listing surface the contract's group name
//! - UI-002 — semantic-only search hits are hydrated with real method/path/summary
//! - UI-003 — `.muted` and `.num` CSS classes referenced by templates are defined
//! - WEB-002 — fallback 404 responses are `text/html`
//! - WEB-003 — web search on an unknown project returns 404 JSON, not an empty 200 page
//!
//! Findings BUG-001, API-001 and API-005 are covered by the unit tests in
//! `src/domain/use_cases/update_contract.rs`; MCP-001/MCP-002 by
//! `tests/e2e_mcp.rs`; WEB-001 by `tests/e2e_web.rs`.

mod common;

use axum::body::Body;
use axum::http::{Request, header};
use common::TestApp;
use http_body_util::BodyExt;
use ledgapi::config::EmbedConfig;
use ledgapi::core::id::Id;
use ledgapi::domain::contract::{ContractCreate, ContractUpdate, Method};
use ledgapi::domain::ports::Repos;
use ledgapi::domain::project::{ProjectCreate, ProjectSlug};
use serde_json::{Value, json};

#[allow(unused_imports)]
use ledgapi::domain::ports::{ContractRepo, GroupRepo, ProjectRepo};

const CFG: EmbedConfig = EmbedConfig {
    cache_dir: String::new(),
    model: String::new(),
    similarity_threshold: 0.85,
    knn_top_k: 5,
    hybrid_limit: 10,
};

fn with_bearer(mut req: Request<Body>, plaintext: &str) -> Request<Body> {
    req.headers_mut().insert(header::AUTHORIZATION, format!("Bearer {plaintext}").parse().unwrap());
    req
}

fn with_session(mut req: Request<Body>, session: &str) -> Request<Body> {
    req.headers_mut().insert(header::COOKIE, format!("ledgapi_session={session}").parse().unwrap());
    req
}

async fn response_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn create_project(app: &TestApp, slug: &str) -> Id {
    app.state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse(slug).unwrap(),
            name: slug.to_owned(),
            description: None,
        })
        .await
        .unwrap()
        .id
}

async fn create_contract(app: &TestApp, project_id: Id, group_id: Option<Id>) -> Id {
    let input = ContractCreate {
        method: Method::Post,
        path: "/users".to_owned(),
        summary: "Create user account".to_owned(),
        description: None,
        request_headers: None,
        request_params: None,
        request_body_schema: None,
        request_example: None,
        response_schema: json!({"type": "object"}),
        response_example: None,
        examples: None,
        auth_type: None,
        status: None,
        tags: None,
        group_name: None,
        group_parent_id: None,
        force: true,
    };
    app.state.repos.contracts().create(project_id, group_id, &input).await.unwrap().id
}

/// Boot an app with an admin bearer token and a project at slug `api`.
async fn setup_app() -> (TestApp, String, Id) {
    let app = TestApp::new();
    let token = app.seed_admin_access_token().await;
    let pid = create_project(&app, "api").await;
    (app, token, pid)
}

async fn mcp_call(app: &TestApp, token: &str, tool: &str, args: Value) -> Value {
    let resp = app
        .oneshot(with_bearer(
            TestApp::mcp_request("tools/call", json!({"name": tool, "arguments": args})),
            token,
        ))
        .await;
    response_json(resp).await
}

/// API-002 regression: filtering `list_contracts` by an unknown group must
/// return a not_found tool error and must NOT create the group.
#[tokio::test]
async fn list_contracts_unknown_group_errors_without_creating() {
    let (app, token, pid) = setup_app().await;
    let body = mcp_call(
        &app,
        &token,
        "list_contracts",
        json!({"project_slug": "api", "group_name": "Ghost"}),
    )
    .await;
    assert_eq!(body["error"]["code"], -32602);
    assert_eq!(body["error"]["data"]["code"], "not_found");

    let groups = app.state.repos.groups().list_with_counts(pid).await.unwrap();
    assert!(groups.iter().all(|g| g.name != "Ghost"), "unknown group must not be created");
}

/// API-003 regression: same read-side rule for `search_contract`.
#[tokio::test]
async fn search_contract_unknown_group_errors_without_creating() {
    let (app, token, pid) = setup_app().await;
    let body = mcp_call(
        &app,
        &token,
        "search_contract",
        json!({"project_slug": "api", "query": "user", "search_mode": "exact", "group_name": "Ghost"}),
    )
    .await;
    assert_eq!(body["error"]["code"], -32602);
    assert_eq!(body["error"]["data"]["code"], "not_found");

    let groups = app.state.repos.groups().list_with_counts(pid).await.unwrap();
    assert!(groups.iter().all(|g| g.name != "Ghost"), "unknown group must not be created");
}

/// API-004 regression: an empty `group_name` on `list_contracts` is treated
/// as "no filter", mirroring create_contract's empty-string drop — it must
/// list contracts, not 404.
#[tokio::test]
async fn list_contracts_empty_group_name_means_no_filter() {
    let (app, token, pid) = setup_app().await;
    create_contract(&app, pid, None).await;
    let body =
        mcp_call(&app, &token, "list_contracts", json!({"project_slug": "api", "group_name": ""}))
            .await;
    let contracts = body["result"]["content"][0]["json"]["contracts"].as_array().unwrap();
    assert_eq!(contracts.len(), 1);
}

/// API-006 regression: updating a contract whose row disappeared (the
/// load/update race) must surface NotFound instead of a silent success.
#[tokio::test]
async fn update_of_deleted_contract_returns_not_found() {
    let (app, _token, pid) = setup_app().await;
    let cid = create_contract(&app, pid, None).await;
    app.state.repos.contracts().delete(pid, cid).await.unwrap();
    let err = app
        .state
        .repos
        .contracts()
        .update(
            pid,
            cid,
            &ContractUpdate { summary: Some("X".to_owned()), ..Default::default() },
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ledgapi::domain::errors::DomainError::NotFound { .. }));
}

/// API-008 regression: the OpenAPI YAML export downloads as an attachment
/// with a per-project filename; invalid slugs still 404.
#[tokio::test]
async fn openapi_yaml_served_as_attachment_download() {
    let (app, _token, pid) = setup_app().await;
    create_contract(&app, pid, None).await;
    let (session, _) = app.seed_admin_session().await;

    let resp = app
        .oneshot(with_session(
            Request::builder().uri("/projects/api/openapi.yml").body(Body::empty()).unwrap(),
            &session,
        ))
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()[header::CONTENT_TYPE], "application/yaml");
    let disposition = resp.headers()[header::CONTENT_DISPOSITION].to_str().unwrap();
    assert_eq!(disposition, "attachment; filename=\"api-openapi.yml\"");

    let bad = app
        .oneshot(with_session(
            Request::builder().uri("/projects/NOT_A_SLUG/openapi.yml").body(Body::empty()).unwrap(),
            &session,
        ))
        .await;
    assert_eq!(bad.status(), 404);
}

/// UI-001 regression: the project page Group column and the MCP contract
/// payloads must carry the contract's group name.
#[tokio::test]
async fn project_page_and_mcp_outputs_include_group_name() {
    let (app, token, pid) = setup_app().await;
    let group = app
        .state
        .repos
        .groups()
        .resolve(
            pid,
            &ledgapi::domain::group::GroupRef {
                name: "Auth".to_owned(),
                description: None,
                parent_id: None,
            },
        )
        .await
        .unwrap();
    create_contract(&app, pid, Some(group.id)).await;
    let (session, _) = app.seed_admin_session().await;

    // Web page: group cell populated.
    let resp = app
        .oneshot(with_session(
            Request::builder().uri("/projects/api").body(Body::empty()).unwrap(),
            &session,
        ))
        .await;
    assert_eq!(resp.status(), 200);
    let html = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&html).into_owned();
    assert!(
        html.contains("class=\"group-name\">Auth"),
        "project page must render the Auth group in the nested tree"
    );

    // MCP list output: group field populated.
    let body = mcp_call(&app, &token, "list_contracts", json!({"project_slug": "api"})).await;
    let contracts = body["result"]["content"][0]["json"]["contracts"].as_array().unwrap();
    assert_eq!(contracts[0]["group"], "Auth");
}

/// UI-002 regression: a hit found only by the semantic branch (no exact
/// match) must be hydrated with its real method/path/summary, not blank
/// placeholder fields.
#[tokio::test]
async fn semantic_only_search_results_are_hydrated() {
    let (app, token, _pid) = setup_app().await;
    // Create through the use case so the embedding is stored and the
    // semantic branch can rank the contract.
    ledgapi::domain::use_cases::create_contract::execute(
        app.state.repos.as_ref(),
        app.state.embedder.clone(),
        &CFG,
        ProjectSlug::parse("api").unwrap(),
        ContractCreate {
            method: Method::Post,
            path: "/users".to_owned(),
            summary: "Create user account".to_owned(),
            response_schema: json!({"type": "object"}),
            force: true,
            description: None,
            request_headers: None,
            request_params: None,
            request_body_schema: None,
            request_example: None,
            response_example: None,
            examples: None,
            auth_type: None,
            status: None,
            tags: None,
            group_name: None,
            group_parent_id: None,
        },
    )
    .await
    .unwrap();

    // The query shares no tokens with the stored contract, so the exact
    // branch misses and the hit arrives via the semantic branch only.
    let body = mcp_call(
        &app,
        &token,
        "search_contract",
        json!({
            "project_slug": "api",
            "query": "zz-qwerty-unrelated-probe",
            "search_mode": "semantic"
        }),
    )
    .await;
    let results = body["result"]["content"][0]["json"]["results"].as_array().unwrap();
    assert!(!results.is_empty(), "stub embedder should still rank the lone contract");
    for r in results {
        assert_eq!(r["path"], "/users", "semantic-only hit must be hydrated");
        assert_eq!(r["method"], "POST");
        assert_eq!(r["summary"], "Create user account");
    }
}

/// UI-003 regression: CSS classes referenced by templates must actually be
/// defined in the stylesheet.
#[test]
fn style_css_defines_muted_and_num_classes() {
    let css = include_str!("../templates/style.css");
    assert!(css.contains(".muted {"), ".muted class must be defined");
    assert!(css.contains(".num {"), ".num class must be defined");
}

/// WEB-001 regression: template-rendered pages (dashboard) are served as
/// text/html with charset, never plain text.
#[tokio::test]
async fn dashboard_is_html() {
    let app = TestApp::new();
    let (session, _) = app.seed_admin_session().await;
    let resp = app
        .oneshot(with_session(Request::builder().uri("/").body(Body::empty()).unwrap(), &session))
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()[header::CONTENT_TYPE], "text/html; charset=utf-8");
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).to_lowercase().starts_with("<!doctype html>"));
}

/// WEB-002 regression: the router fallback renders the HTML 404 page with a
/// text/html content type, never plain text.
#[tokio::test]
async fn fallback_not_found_is_html() {
    let app = TestApp::new();
    let resp = app
        .oneshot(Request::builder().uri("/definitely-not-a-route").body(Body::empty()).unwrap())
        .await;
    assert_eq!(resp.status(), 404);
    assert_eq!(resp.headers()[header::CONTENT_TYPE], "text/html; charset=utf-8");
}

/// WEB-003 regression: searching inside an unknown project returns the JSON
/// 404 error envelope, not a misleading 200 empty results page.
#[tokio::test]
async fn web_search_unknown_project_returns_json_404() {
    let app = TestApp::new();
    let (session, _) = app.seed_admin_session().await;
    let resp = app
        .oneshot(with_session(
            Request::builder()
                .uri("/projects/nope/search?q=user&mode=exact")
                .body(Body::empty())
                .unwrap(),
            &session,
        ))
        .await;
    assert_eq!(resp.status(), 404);
    assert_eq!(resp.headers()[header::CONTENT_TYPE], "application/json");
    let body = response_json(resp).await;
    assert_eq!(body["errors"][0]["code"], "not_found");

    // Sanity: a known project still searches fine through the same route.
    create_project(&app, "known").await;
    let ok = app
        .oneshot(with_session(
            Request::builder()
                .uri("/projects/known/search?q=user&mode=exact")
                .body(Body::empty())
                .unwrap(),
            &session,
        ))
        .await;
    assert_eq!(ok.status(), 200);
}

/// BUG-000005 regression: the project page group tree was silently
/// dropping contracts that had no group, even though the `total_contracts`
/// count in the header still reflected them. A new contract with no group
/// would make the page show "Contracts (1)" and "No contracts yet."
/// simultaneously, hiding the contract from the user. The fix introduces
/// an "Ungrouped" virtual group at the top of the tree.
#[tokio::test]
async fn project_page_shows_ungrouped_contracts() {
    let (app, _token, pid) = setup_app().await;
    // No group: contract will be ungrouped.
    create_contract(&app, pid, None).await;
    let (session, _) = app.seed_admin_session().await;
    let resp = app
        .oneshot(with_session(
            Request::builder().uri("/projects/api").body(Body::empty()).unwrap(),
            &session,
        ))
        .await;
    assert_eq!(resp.status(), 200);
    let html = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&html).into_owned();
    assert!(
        html.contains("Contracts (1)"),
        "header must show the ungrouped contract count: {html}"
    );
    assert!(
        !html.contains("No contracts yet"),
        "page must not say 'No contracts yet' when an ungrouped contract exists: {html}"
    );
    assert!(
        html.contains(">Ungrouped<"),
        "page must render an 'Ungrouped' virtual group for contracts without a group: {html}"
    );
    assert!(
        html.contains("class=\"method-badge method-POST\""),
        "ungrouped contract method badge must render: {html}"
    );
}

/// BUG-000006 regression: every authenticated page (base layout and docs
/// base) must expose a working sign-out form, otherwise users have no way
/// to end their session through the UI. The fix added a `data-logout` form
/// to both `templates/base.html` and `templates/docs/base_docs.html`, and
/// made the CSRF cookie readable from JavaScript (it was HttpOnly, which
/// was correct for security but prevented the form from reading the token).
#[tokio::test]
async fn logout_form_present_on_every_page() {
    let (app, _token, _pid) = setup_app().await;
    let (session, _) = app.seed_admin_session().await;
    for path in ["/", "/admin/users", "/docs", "/admin/audit"] {
        let resp = app
            .oneshot(with_session(
                Request::builder().uri(path).body(Body::empty()).unwrap(),
                &session,
            ))
            .await;
        assert_eq!(resp.status(), 200, "GET {path} must be 200");
        let html = resp.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8_lossy(&html).into_owned();
        assert!(
            html.contains("data-logout"),
            "{path} must render a logout form with data-logout attribute: {html}"
        );
        assert!(
            html.contains("action=\"/logout\""),
            "{path} logout form must POST to /logout: {html}"
        );
        assert!(
            html.contains("Sign out"),
            "{path} logout form must have a 'Sign out' button: {html}"
        );
    }
}

/// BUG-000006 regression: submitting the logout form with a valid CSRF
/// must invalidate the session, so a subsequent /admin/users navigation
/// is redirected to /login.
#[tokio::test]
async fn logout_form_submission_invalidates_session() {
    let (app, _token, _pid) = setup_app().await;
    let (session, csrf) = app.seed_admin_session().await;
    // First confirm /admin/users is accessible.
    let resp = app
        .oneshot(with_session(
            Request::builder().uri("/admin/users").body(Body::empty()).unwrap(),
            &session,
        ))
        .await;
    assert_eq!(resp.status(), 200);

    // Submit logout. We need a session AND csrf cookie; build a Cookie
    // header with both.
    let body = format!("csrf={csrf}");
    let logout = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/logout")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", format!("ledgapi_session={session}; ledgapi_csrf={csrf}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await;
    assert!(
        logout.status() == 303 || logout.status() == 302 || logout.status() == 200,
        "logout must redirect or return OK, got {}",
        logout.status()
    );

    // A second request with the same session should now be unauthenticated
    // (303 to /login).
    let after = app
        .oneshot(with_session(
            Request::builder().uri("/admin/users").body(Body::empty()).unwrap(),
            &session,
        ))
        .await;
    assert_eq!(
        after.status(),
        303,
        "session must be invalidated after logout; got {}",
        after.status()
    );
}
