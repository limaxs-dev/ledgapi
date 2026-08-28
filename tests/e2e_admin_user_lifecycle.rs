//! Tests for the user-management lifecycle routes that fill the gap
//! QA-hunter BUG-000008 surfaced in iteration 14:
//!
//! - `POST /admin/users/{id}/update` — change a user's role and active flag
//! - `POST /admin/users/{id}/password` — reset a user's password
//! - The self-admin guard (a super-admin cannot demote or deactivate
//!   themselves)
//!
//! These cover the previously-missing controls on the /admin/users
//! page; until this iteration, the page only had create + list, with
//! no way to revoke a compromised viewer's access from the UI.

mod common;
use axum::body::Body;
use axum::http::Request;
use common::TestApp;
use ledgapi::domain::auth::{Role, UserCreate};
use ledgapi::domain::ports::Repos;
use ledgapi::infra::auth::password;

async fn post_user_route(
    app: &TestApp,
    session: &str,
    csrf: &str,
    route: &str,
    body: &str,
) -> (u16, Option<String>) {
    let req = Request::builder()
        .method("POST")
        .uri(route)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("cookie", format!("ledgapi_session={session}; ledgapi_csrf={csrf}"))
        .body(Body::from(body.to_owned()))
        .unwrap();
    let resp = app.oneshot(req).await;
    let status = resp.status().as_u16();
    let location = resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    (status, location)
}

async fn seed_viewer(app: &TestApp, username: &str) -> String {
    let user = app
        .state
        .repos
        .users()
        .create(&UserCreate {
            username: username.to_owned(),
            password_hash: password::hash_password("viewerpassword123").unwrap(),
            role: Role::Viewer,
        })
        .await
        .unwrap();
    user.id.to_string()
}

#[tokio::test]
async fn update_user_changes_role_and_active() {
    let app = TestApp::new();
    let (session, csrf) = app.seed_admin_session().await;
    let viewer_id = seed_viewer(&app, "um-update-1").await;

    let body = format!("role=editor&active=false&csrf={csrf}");
    let (status, location) = post_user_route(
        &app,
        &session,
        &csrf,
        &format!("/admin/users/{viewer_id}/update"),
        &body,
    )
    .await;
    assert_eq!(status, 303, "update must redirect, got {status}");
    assert_eq!(location.as_deref(), Some("/admin/users?flash=updated"));

    let updated = app
        .state
        .repos
        .users()
        .find_by_username("um-update-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.role, Role::Editor);
    assert!(!updated.active, "active flag should be flipped to false");
}

#[tokio::test]
async fn update_user_unknown_role_redirects_with_flash_invalid() {
    let app = TestApp::new();
    let (session, csrf) = app.seed_admin_session().await;
    let viewer_id = seed_viewer(&app, "um-update-badrole").await;
    let body = format!("role=ghost&active=true&csrf={csrf}");
    let (status, location) = post_user_route(
        &app,
        &session,
        &csrf,
        &format!("/admin/users/{viewer_id}/update"),
        &body,
    )
    .await;
    assert_eq!(status, 303);
    assert_eq!(location.as_deref(), Some("/admin/users?flash=invalid"));
}

#[tokio::test]
async fn update_user_unknown_id_redirects_with_flash_notfound() {
    let app = TestApp::new();
    let (session, csrf) = app.seed_admin_session().await;
    let body = format!("role=editor&active=true&csrf={csrf}");
    let (status, location) = post_user_route(
        &app,
        &session,
        &csrf,
        "/admin/users/00000000-0000-0000-0000-000000000000/update",
        &body,
    )
    .await;
    assert_eq!(status, 303);
    assert_eq!(location.as_deref(), Some("/admin/users?flash=notfound"));
}

#[tokio::test]
async fn update_self_admin_cannot_be_demoted() {
    // Domain guard: a super-admin cannot demote or deactivate themselves.
    // The handler maps that DomainError::Forbidden to flash=invalid.
    let app = TestApp::new();
    let (session, csrf) = app.seed_admin_session().await;
    let admin_id = app
        .state
        .repos
        .users()
        .find_by_username("admin")
        .await
        .unwrap()
        .unwrap()
        .id
        .to_string();
    let body = format!("role=viewer&active=true&csrf={csrf}");
    let (status, location) = post_user_route(
        &app,
        &session,
        &csrf,
        &format!("/admin/users/{admin_id}/update"),
        &body,
    )
    .await;
    assert_eq!(status, 303);
    assert_eq!(location.as_deref(), Some("/admin/users?flash=invalid"));
    // And the admin's role must still be SuperAdmin
    let admin = app
        .state
        .repos
        .users()
        .find_by_username("admin")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(admin.role, Role::SuperAdmin);
    assert!(admin.active);
}

#[tokio::test]
async fn reset_password_changes_user_password() {
    let app = TestApp::new();
    let (session, csrf) = app.seed_admin_session().await;
    let viewer_id = seed_viewer(&app, "um-pw-1").await;
    let body = format!("password=newpassphrase12345&csrf={csrf}");
    let (status, location) = post_user_route(
        &app,
        &session,
        &csrf,
        &format!("/admin/users/{viewer_id}/password"),
        &body,
    )
    .await;
    assert_eq!(status, 303);
    assert_eq!(location.as_deref(), Some("/admin/users?flash=updated"));
    // The new password should now be valid; the old one shouldn't.
    let user = app
        .state
        .repos
        .users()
        .find_by_username("um-pw-1")
        .await
        .unwrap()
        .unwrap();
    assert!(
        password::verify_password("newpassphrase12345", &user.password_hash).unwrap_or(false),
        "new password should be accepted"
    );
    assert!(
        !password::verify_password("viewerpassword123", &user.password_hash).unwrap_or(true),
        "old password should no longer be accepted"
    );
}

#[tokio::test]
async fn reset_password_short_password_redirects_with_flash_invalid() {
    let app = TestApp::new();
    let (session, csrf) = app.seed_admin_session().await;
    let viewer_id = seed_viewer(&app, "um-pw-short").await;
    let body = format!("password=short&csrf={csrf}");
    let (status, location) = post_user_route(
        &app,
        &session,
        &csrf,
        &format!("/admin/users/{viewer_id}/password"),
        &body,
    )
    .await;
    assert_eq!(status, 303);
    assert_eq!(location.as_deref(), Some("/admin/users?flash=invalid"));
}

#[tokio::test]
async fn update_route_without_csrf_is_rejected() {
    // The session cookie alone isn't enough — CSRF must also be sent
    // (verified via constant-time compare of the SHA-256 hash of the
    // form value vs the cookie).
    let app = TestApp::new();
    let (session, _csrf) = app.seed_admin_session().await;
    let viewer_id = seed_viewer(&app, "um-nocsrf-update").await;
    let req = Request::builder()
        .method("POST")
        .uri(format!("/admin/users/{viewer_id}/update"))
        .header("content-type", "application/x-www-form-urlencoded")
        .header("cookie", format!("ledgapi_session={session}"))
        .body(Body::from("role=editor&active=true&csrf="))
        .unwrap();
    let resp = app.oneshot(req).await;
    let status = resp.status().as_u16();
    // The handler returns 403 when CSRF validation fails.
    assert_eq!(status, 403, "missing CSRF should be 403, got {status}");
}
