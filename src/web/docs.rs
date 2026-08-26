//! Docs site — server-rendered from the Markdown content under
//! `docs/content/`. Same Axum + Askama stack as the product UI, no client JS.
//!
//! Routes:
//!   GET /docs                       home (value prop + quickstart)
//!   GET /docs/getting-started/{id}  install, first-login, connect-mcp, first-contract
//!   GET /docs/concepts/{id}         architecture, projects-and-groups, ...
//!   GET /docs/mcp-tools/{id}        one page per MCP tool
//!   GET /docs/{ref}                 http-api, auth, deployment, changelog
//!
//! All routes are mounted behind the session middleware in `router.rs`. The
//! handler extracts the `Principal` so the top nav can show the username.

use crate::domain::auth::Principal;
use crate::web::templates::{SidebarFile, TocEntry};
use askama::Template;
use axum::Extension;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use comrak::{ComrakOptions, markdown_to_html};
use regex::Regex;

/// One entry in the docs map: the URL suffix is what the user types in the
/// path bar, the body is the raw Markdown at compile time.
macro_rules! doc {
    ($url:expr, $path:expr) => {
        ($url, include_str!(concat!("../../docs/content/", $path)))
    };
}

/// Embedded doc table. Order is irrelevant; lookup is by URL suffix.
const DOCS: &[(&str, &str)] = &[
    doc!("", "home.md"),
    doc!("docs-index", "docs-index.md"),
    doc!("getting-started/install", "getting-started/install.md"),
    doc!("getting-started/first-login", "getting-started/first-login.md"),
    doc!("getting-started/connect-mcp", "getting-started/connect-mcp.md"),
    doc!("getting-started/first-contract", "getting-started/first-contract.md"),
    doc!("concepts/architecture", "concepts/architecture.md"),
    doc!("concepts/projects-and-groups", "concepts/projects-and-groups.md"),
    doc!("concepts/rag-and-duplicates", "concepts/rag-and-duplicates.md"),
    doc!("concepts/audit-log", "concepts/audit-log.md"),
    doc!("mcp-tools/list-projects", "mcp-tools/list-projects.md"),
    doc!("mcp-tools/create-project", "mcp-tools/create-project.md"),
    doc!("mcp-tools/list-groups", "mcp-tools/list-groups.md"),
    doc!("mcp-tools/list-contracts", "mcp-tools/list-contracts.md"),
    doc!("mcp-tools/get-contract-by-id", "mcp-tools/get-contract-by-id.md"),
    doc!("mcp-tools/search-contract", "mcp-tools/search-contract.md"),
    doc!("mcp-tools/create-contract", "mcp-tools/create-contract.md"),
    doc!("mcp-tools/update-contract", "mcp-tools/update-contract.md"),
    doc!("mcp-tools/delete-contract", "mcp-tools/delete-contract.md"),
    doc!("mcp-tools/export-openapi", "mcp-tools/export-openapi.md"),
    doc!("http-api", "http-api.md"),
    doc!("auth", "auth.md"),
    doc!("deployment", "deployment.md"),
    doc!("changelog", "changelog.md"),
];

/// Sidebar nav tree, embedded at compile time.
const SIDEBAR_JSON: &str = include_str!("../../docs/content/sidebar.json");

// -- Frontmatter -----------------------------------------------------------

/// YAML frontmatter at the top of every doc file.
#[derive(Debug, Default, serde::Deserialize)]
struct Frontmatter {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    method: Option<String>,
}

/// Split frontmatter from the body. Returns the parsed frontmatter and the
/// remaining Markdown. If the file has no frontmatter, returns defaults and
/// the original string.
fn split_frontmatter(md: &str) -> (Frontmatter, &str) {
    let Some(after_open) = md.strip_prefix("---\n") else {
        return (Frontmatter::default(), md);
    };
    let Some((fm_raw, body)) = after_open.split_once("\n---\n") else {
        return (Frontmatter::default(), md);
    };
    let fm = serde_yaml::from_str::<Frontmatter>(fm_raw).unwrap_or_default();
    (fm, body)
}

// -- TOC -------------------------------------------------------------------

/// Walk the raw Markdown for H2/H3 lines and produce a TOC with slugs.
fn build_toc(md: &str) -> Vec<TocEntry> {
    let mut out = Vec::new();
    for line in md.lines() {
        if let Some(text) = line.strip_prefix("### ") {
            out.push(TocEntry { level: 3, text: text.to_owned(), id: slugify(text) });
        } else if let Some(text) = line.strip_prefix("## ") {
            out.push(TocEntry { level: 2, text: text.to_owned(), id: slugify(text) });
        }
    }
    out
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_owned()
}

// -- Sidebar ---------------------------------------------------------------

/// Parse the embedded sidebar.json and patch each item with its URL path
/// derived from the DOCS table.
fn build_sidebar() -> SidebarFile {
    let mut sb: SidebarFile =
        serde_json::from_str(SIDEBAR_JSON).expect("sidebar.json must be valid");
    for group in &mut sb.groups {
        for item in &mut group.items {
            let suffix = DOCS
                .iter()
                .find(|(k, _)| k.rsplit('/').next() == Some(item.id.as_str()))
                .map(|(k, _)| *k)
                .unwrap_or("");
            item.path = format!("/docs/{suffix}");
        }
    }
    sb
}

// -- Markdown pre-processing ----------------------------------------------

/// Convert the three doc-specific Markdown extensions into raw HTML so
/// comrak passes them through unchanged.
///
/// 1. `::: warning|info|note|success|danger\n...\n:::` becomes a callout div.
/// 2. `[text](url){.btn-X}` becomes a styled anchor.
/// 3. `::: bento\n...\n:::` becomes a bento wrapper.
fn pre_process(md: &str) -> String {
    let mut out = md.to_owned();

    let re_callout =
        Regex::new(r"(?ms)^::: (warning|info|note|success|danger)\n(.+?)\n:::").unwrap();
    out = re_callout
        .replace_all(&out, |caps: &regex::Captures<'_>| {
            let kind = &caps[1];
            let body = &caps[2];
            format!("<div class=\"callout callout-{kind}\">\n\n{body}\n\n</div>")
        })
        .into_owned();

    let re_bento = Regex::new(r"(?ms)^::: bento\n(.+?)\n:::").unwrap();
    out = re_bento
        .replace_all(&out, |caps: &regex::Captures<'_>| {
            let body = &caps[1];
            format!("<div class=\"bento\">\n\n{body}\n\n</div>")
        })
        .into_owned();

    let re_btn = Regex::new(r"\[([^\]]+)\]\(([^\)]+)\)\{\.btn-([a-z]+)\}").unwrap();
    out = re_btn
        .replace_all(&out, |caps: &regex::Captures<'_>| {
            format!("<a class=\"btn btn-{}\" href=\"{}\">{}</a>", &caps[3], &caps[2], &caps[1])
        })
        .into_owned();

    out
}

// -- Markdown rendering ----------------------------------------------------

/// Render Markdown to HTML with the right extensions, then inject heading
/// ids so the in-page TOC can anchor-link to them.
///
/// Syntax highlighting is intentionally not enabled. comrak's `syntect`
/// feature pulls in a transitive `yaml-rust` advisory that `cargo deny`
/// flags. Plain styled code blocks (no highlighting) ship instead. The
/// `language-xxx` class from the fence info string is preserved on the
/// rendered `<code>` element so a future highlighter (e.g. static JS) can
/// re-style without re-parsing.
fn render_markdown(md: &str) -> String {
    let pre = pre_process(md);

    let mut options = ComrakOptions::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.autolink = true;
    options.render.unsafe_ = true; // trusted content
    // github_pre_lang defaults to true: the language hint from the fence
    // info string becomes `language-xxx` on the <code> element.

    let html = markdown_to_html(&pre, &options);
    inject_heading_ids(&html)
}

/// Add `id="..."` to h2 and h3 tags. Cheaper than walking comrak's AST.
fn inject_heading_ids(html: &str) -> String {
    let h2 = Regex::new(r"<h2>([^<]+)</h2>").unwrap();
    let h3 = Regex::new(r"<h3>([^<]+)</h3>").unwrap();
    let html = h2.replace_all(html, |c: &regex::Captures<'_>| {
        format!("<h2 id=\"{}\">{}</h2>", slugify(&c[1]), &c[1])
    });
    h3.replace_all(&html, |c: &regex::Captures<'_>| {
        format!("<h3 id=\"{}\">{}</h3>", slugify(&c[1]), &c[1])
    })
    .into_owned()
}

// -- Handlers --------------------------------------------------------------

/// `GET /docs` — the docs home page.
pub async fn home(Extension(principal): Extension<Principal>) -> Response {
    let Some((_, body)) = DOCS.iter().find(|(k, _)| k.is_empty()) else {
        return not_found();
    };
    let (fm, body_md) = split_frontmatter(body);
    let html = render_markdown(body_md);
    let sidebar = build_sidebar();
    let tpl = crate::web::templates::DocsHomeTpl {
        title: "ledgapi docs",
        page_title: if fm.title.is_empty() { "ledgapi" } else { &fm.title },
        page_description: &fm.description,
        username: &principal.username,
        sidebar: &sidebar,
        current_id: "home",
        body_html: &html,
    };
    match tpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => render_error(e),
    }
}

/// `GET /docs/{*rest}` — any docs page. `rest` is the URL suffix.
pub async fn page(
    Extension(principal): Extension<Principal>,
    Path(rest): Path<String>,
) -> Response {
    let Some((_, body)) = DOCS.iter().find(|(k, _)| *k == rest.as_str()) else {
        return not_found();
    };
    let (fm, body_md) = split_frontmatter(body);
    let html = render_markdown(body_md);
    let toc = build_toc(body_md);
    let sidebar = build_sidebar();
    let page_id = page_id_from_suffix(&rest);
    let group_label = sidebar_position(&sidebar, &page_id).unwrap_or_default();
    let tpl = crate::web::templates::DocsPageTpl {
        title: if fm.title.is_empty() { &rest } else { &fm.title },
        page_title: if fm.title.is_empty() { &rest } else { &fm.title },
        page_description: &fm.description,
        method: fm.method.as_deref(),
        username: &principal.username,
        sidebar: &sidebar,
        current_id: &page_id,
        group_label: &group_label,
        toc: &toc,
        body_html: &html,
    };
    match tpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => render_error(e),
    }
}

fn sidebar_position(sb: &SidebarFile, id: &str) -> Option<String> {
    for group in &sb.groups {
        for item in &group.items {
            if item.id == id {
                return Some(group.label.clone());
            }
        }
    }
    None
}

fn page_id_from_suffix(suffix: &str) -> String {
    suffix.rsplit('/').next().unwrap_or(suffix).to_owned()
}

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "doc not found").into_response()
}

fn render_error(e: askama::Error) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("template error: {e}")).into_response()
}
