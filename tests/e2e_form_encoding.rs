//! Regression tests for QA-hunter BUG-000002:
//! form parsers treated '+' as a literal plus instead of a space, so the
//! consent page's hidden scope field (browser-encoded with '+') never
//! matched an allowed scope and tokens were issued with an empty scope.

mod common;
use common::TestApp;

/// A web-login password containing a space must be accepted when submitted
/// with HTML form encoding ('+' for space), as every real browser does.
#[tokio::test]
async fn login_accepts_browser_encoded_password_with_space() {
    let app = TestApp::new();
    app.seed_user_with_password("spacey", "pass word+here").await;

    let body = "username=spacey&password=pass+word%2Bhere&next=%2F";
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(body))
        .unwrap();
    let resp = app.oneshot(req).await;
    assert_eq!(resp.status(), 303, "login with space-containing password must succeed");
}

/// The OAuth consent endpoint receives its own hidden scope field
/// browser-encoded ('+' for spaces); the resulting authorization code must
/// carry the requested scopes, not an empty set.
#[tokio::test]
async fn consent_preserves_multi_scope_from_browser_encoding() {
    let app = TestApp::new();
    app.register_qa_client().await;
    let (session, csrf) = app.seed_admin_session().await;

    // Authorize to render the consent page (also validates the request).
    let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
    let authorize = axum::http::Request::builder()
        .uri(format!(
            "/oauth/authorize?response_type=code&client_id=test-client\
             &redirect_uri=http://127.0.0.1:9999/cb&scope=ledgapi:read%20ledgapi:write\
             &state=s1&code_challenge={challenge}&code_challenge_method=S256"
        ))
        .header("cookie", format!("ledgapi_session={session}"))
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(authorize).await;
    assert_eq!(resp.status(), 200, "consent page should render");

    // Approve consent with '+'-encoded scope, exactly as a browser submits it.
    let body = "client_id=test-client&redirect_uri=http%3A%2F%2F127.0.0.1%3A9999%2Fcb\
                &code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM\
                &code_challenge_method=S256&scope=ledgapi%3Aread+ledgapi%3Awrite\
                &state=s1&decision=approve"
        .replace('\n', "")
        + "&csrf=";
    let body = body + &csrf;
    let consent = axum::http::Request::builder()
        .method("POST")
        .uri("/oauth/consent")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("cookie", format!("ledgapi_session={session}; ledgapi_csrf={csrf}"))
        .body(axum::body::Body::from(body))
        .unwrap();
    let resp = app.oneshot(consent).await;
    assert_eq!(resp.status(), 303, "consent must succeed with matching csrf cookie");
    let location = resp
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .expect("consent redirect");
    let code = location
        .split("code=")
        .nth(1)
        .and_then(|rest| rest.split('&').next())
        .expect("authorization code in redirect");

    // Exchange the code; the token response must echo non-empty scopes.
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let token_body = format!(
        "grant_type=authorization_code&code={code}\
         &redirect_uri=http%3A%2F%2F127.0.0.1%3A9999%2Fcb&client_id=test-client\
         &code_verifier={verifier}"
    );
    let token_req = axum::http::Request::builder()
        .method("POST")
        .uri("/oauth/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(token_body))
        .unwrap();
    let resp = app.oneshot(token_req).await;
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(status, 200, "token exchange failed: {}", String::from_utf8_lossy(&bytes));
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let scope = json["scope"].as_str().unwrap_or_default();
    assert!(
        scope.split_whitespace().all(|s| s == "ledgapi:read" || s == "ledgapi:write"),
        "unexpected scopes: {scope}"
    );
    assert!(
        scope.contains("ledgapi:read") && scope.contains("ledgapi:write"),
        "'+'-encoded consent scope was lost; got {scope:?}"
    );
}

/// decode_form_component unit behavior is exercised through the e2e flows
/// above; this pins the exact contract directly.
#[test]
fn decode_form_component_plus_becomes_space_and_percent_decodes() {
    use ledgapi::web::auth::decode_form_component;
    assert_eq!(decode_form_component("pass+word%2Bhere").unwrap(), "pass word+here");
    assert_eq!(
        decode_form_component("ledgapi%3Aread+ledgapi%3Awrite").unwrap(),
        "ledgapi:read ledgapi:write"
    );
}
