mod common;
use common::TestApp;

#[tokio::test]
async fn protected_html_routes_redirect_to_login() {
    let app = TestApp::new();

    for uri in [
        "/",
        "/projects/missing-project",
        "/projects/missing-project/contracts/not-an-id",
        "/projects/missing-project/search?q=test",
    ] {
        let request =
            axum::http::Request::builder().uri(uri).body(axum::body::Body::empty()).unwrap();
        let response = app.oneshot(request).await;
        assert_eq!(response.status(), 303, "expected login redirect for {uri}");
        assert!(response.headers().contains_key(axum::http::header::LOCATION));
    }
}

#[tokio::test]
async fn login_page_is_html_and_no_store() {
    let app = TestApp::new();
    let request =
        axum::http::Request::builder().uri("/login").body(axum::body::Body::empty()).unwrap();
    let response = app.oneshot(request).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()[axum::http::header::CONTENT_TYPE], "text/html; charset=utf-8");
    assert_eq!(response.headers()[axum::http::header::CACHE_CONTROL], "no-store");
}
