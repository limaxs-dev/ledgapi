//! Regression tests for QA-hunter BUG-000004: `POST /admin/users` returned
//! a bare HTTP 400 (empty body) for a too-short password or unknown role
//! instead of redirecting to `/admin/users?flash=invalid` like every other
//! validation failure does. The friendly flash message the new admin
//! template advertises ("Check the username and password (minimum 12
//! characters)") was therefore unreachable from the user-facing flow.

mod common;
use axum::body::Body;
use axum::http::Request;
use common::TestApp;

async fn post_form(app: &TestApp, session: &str, csrf: &str, body: &str) -> (u16, Option<String>) {
    let req = Request::builder()
        .method("POST")
        .uri("/admin/users")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("cookie", format!("ledgapi_session={session}; ledgapi_csrf={csrf}"))
        .body(Body::from(body.to_owned()))
        .unwrap();
    let resp = app.oneshot(req).await;
    let status = resp.status().as_u16();
    let location = resp.headers().get("location").and_then(|v| v.to_str().ok()).map(str::to_owned);
    (status, location)
}

#[tokio::test]
async fn create_user_short_password_redirects_with_flash_invalid() {
    let app = TestApp::new();
    let (session, csrf) = app.seed_admin_session().await;
    let body = format!("username=foo123&password=short&role=viewer&csrf={csrf}");
    let (status, location) = post_form(&app, &session, &csrf, &body).await;
    assert_eq!(status, 303, "validation failure must redirect, got {status}");
    assert_eq!(location.as_deref(), Some("/admin/users?flash=invalid"));
}

#[tokio::test]
async fn create_user_unknown_role_redirects_with_flash_invalid() {
    let app = TestApp::new();
    let (session, csrf) = app.seed_admin_session().await;
    let body = format!("username=foo124&password=validpassword123&role=bogus&csrf={csrf}");
    let (status, location) = post_form(&app, &session, &csrf, &body).await;
    assert_eq!(status, 303, "validation failure must redirect, got {status}");
    assert_eq!(location.as_deref(), Some("/admin/users?flash=invalid"));
}
