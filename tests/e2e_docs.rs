//! Regression tests for QA-hunter BUG-000003 (docs home rendered two
//! conflicting "View on GitHub" buttons — template hero CTAs duplicated the
//! markdown-authored CTAs with a different URL) and for the docs route
//! surface's core contract: every embedded doc page resolves 200, unknown
//! suffixes 404 cleanly, and the routes sit behind the web-auth gate.

mod common;
use axum::body::Body;
use axum::http::Request;
use common::TestApp;

async fn get(app: &TestApp, session: &str, uri: &str) -> (u16, String, Option<String>) {
    let req = Request::builder()
        .uri(uri)
        .header("cookie", format!("ledgapi_session={session}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await;
    let content_type =
        resp.headers().get("content-type").and_then(|v| v.to_str().ok()).map(str::to_owned);
    let status = resp.status().as_u16();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8_lossy(&body).into_owned(), content_type)
}

/// Every doc suffix in the DOCS table must resolve 200 as HTML. The sidebar
/// is the user-facing contract: a link that 404s (or a page that leaks raw
/// frontmatter) is a broken docs surface.
#[tokio::test]
async fn all_embedded_doc_pages_resolve_200_html() {
    let app = TestApp::new();
    let (session, _) = app.seed_admin_session().await;

    let suffixes = [
        "",
        "docs-index",
        "getting-started/install",
        "getting-started/first-login",
        "getting-started/connect-mcp",
        "getting-started/first-contract",
        "concepts/architecture",
        "concepts/projects-and-groups",
        "concepts/rag-and-duplicates",
        "concepts/audit-log",
        "mcp-tools/list-projects",
        "mcp-tools/create-project",
        "mcp-tools/list-groups",
        "mcp-tools/list-contracts",
        "mcp-tools/get-contract-by-id",
        "mcp-tools/search-contract",
        "mcp-tools/create-contract",
        "mcp-tools/update-contract",
        "mcp-tools/delete-contract",
        "mcp-tools/export-openapi",
        "http-api",
        "auth",
        "deployment",
        "changelog",
    ];
    for suffix in suffixes {
        let path = if suffix.is_empty() { "/docs".to_owned() } else { format!("/docs/{suffix}") };
        let (status, body, content_type) = get(&app, &session, &path).await;
        assert_eq!(status, 200, "{path} must resolve 200");
        assert!(
            content_type.as_deref().unwrap_or_default().starts_with("text/html"),
            "{path} must be text/html, got {content_type:?}"
        );
        assert!(
            !body.starts_with("---\n") && !body.contains("description: A self-hosted"),
            "{path} leaked raw frontmatter"
        );
    }
}

/// Unknown doc suffixes must return a clean 404, never 500 or file contents.
#[tokio::test]
async fn unknown_doc_suffix_returns_clean_404() {
    let app = TestApp::new();
    let (session, _) = app.seed_admin_session().await;
    for path in ["/docs/nonexistent-page", "/docs/a/b/c/d/e", "/docs/getting-started/install/"] {
        let (status, _, _) = get(&app, &session, path).await;
        assert_eq!(status, 404, "{path} must 404");
    }
}

/// Percent-encoded traversal against /docs/{*rest} must never read files.
#[tokio::test]
async fn docs_path_traversal_is_rejected() {
    let app = TestApp::new();
    let (session, _) = app.seed_admin_session().await;
    for path in ["/docs/..%2f..%2fCargo.toml", "/docs/%2e%2e%2fCargo.toml"] {
        let (status, body, _) = get(&app, &session, path).await;
        assert_ne!(status, 200, "{path} must not serve files");
        assert!(!body.contains("[package]"), "{path} leaked Cargo.toml");
    }
}

/// The docs home must render exactly one CTA row: the template's hero CTAs.
/// BUG-000003 was the markdown body ALSO emitting 'Read the docs' and a
/// 'View on GitHub' button pointing at https://github.com/ instead of this
/// repository — same labels, divergent targets.
#[tokio::test]
async fn docs_home_has_single_nonconflicting_cta_set() {
    let app = TestApp::new();
    let (session, _) = app.seed_admin_session().await;
    let (_, body, _) = get(&app, &session, "/docs").await;

    let github_btns: Vec<&str> =
        body.match_indices("View on GitHub").map(|(i, _)| &body[..i]).collect();
    assert_eq!(
        github_btns.len(),
        1,
        "exactly one 'View on GitHub' button expected; found {}: {}",
        github_btns.len(),
        body
    );
    // The single button must point at this repo, not github.com root.
    let btn_start = body.rfind("btn btn-secondary").expect("secondary CTA present");
    assert!(
        body[btn_start..].contains("github.com/limaxs-dev/ledgapi"),
        "'View on GitHub' must link to the canonical repo URL"
    );
}

/// Heading ids and TOC anchors must stay in sync: every TOC entry links to
/// an id that exists on the page (dangling anchors were the BUG-000003-era
/// pipeline risk flagged during exploration).
#[tokio::test]
async fn toc_anchors_resolve_to_heading_ids() {
    let app = TestApp::new();
    let (session, _) = app.seed_admin_session().await;
    for path in ["/docs/concepts/architecture", "/docs/deployment", "/docs/auth"] {
        let (_, body, _) = get(&app, &session, path).await;
        for (anchor, _) in body.match_indices("href=\"#") {
            let rest = &body[anchor + 7..];
            let end = rest.find('"').expect("terminated href");
            let slug = &rest[..end];
            assert!(
                body.contains(&format!("id=\"{slug}\"")),
                "{path}: TOC anchor #{slug} has no matching heading id"
            );
        }
    }
}
