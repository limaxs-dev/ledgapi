//! Askama template structs. One per `.html` file.

use askama::Template;
use serde::Serialize;

/// 404 page. Receives the requested path so it can echo it back.
#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFoundTpl<'a> {
    pub path: &'a str,
}

/// Base layout — child templates `{% extends "base.html" %}`.
#[derive(Template)]
#[template(path = "base.html")]
pub struct BaseTpl<'a> {
    pub title: &'a str,
    pub body: &'a str,
}

/// Dashboard — list of all projects with contract counts.
#[derive(Template, Serialize)]
#[template(path = "dashboard.html")]
pub struct DashboardTpl<'a> {
    pub title: &'a str,
    pub projects: Vec<ProjectRow>,
}

/// One row of the project list.
#[derive(Serialize)]
pub struct ProjectRow {
    pub slug: String,
    pub name: String,
    pub contract_count: i64,
}

/// Project detail — groups + contracts + search box.
#[derive(Template, Serialize)]
#[template(path = "project.html")]
pub struct ProjectTpl<'a> {
    pub title: &'a str,
    pub slug: &'a str,
    pub name: &'a str,
    pub groups: Vec<GroupRow>,
    pub contracts: Vec<ContractRow>,
}

/// One row in the groups table.
#[derive(Serialize)]
pub struct GroupRow {
    pub name: String,
    pub contract_count: i64,
}

/// One row in the contracts table on the project page.
#[derive(Serialize)]
pub struct ContractRow {
    pub id: String,
    pub method: String,
    pub path: String,
    pub summary: String,
    pub status: String,
    pub group: String,
}

/// One rendered request/response example on a contract detail page.
#[derive(Serialize)]
pub struct ContractExampleRow {
    pub name: String,
    pub kind: String,
    pub status_code: u16,
    pub request: String,
    pub response: String,
}

/// Contract detail page.
#[derive(Template, Serialize)]
#[template(path = "contract.html")]
pub struct ContractTpl {
    pub title: String,
    pub id: String,
    pub method: String,
    pub path: String,
    pub summary: String,
    pub status: String,
    pub description: Option<String>,
    pub auth_type: Option<String>,
    pub tags: Vec<String>,
    pub request_headers: Option<String>,
    pub request_params: Option<String>,
    pub request_body_schema: Option<String>,
    pub request_example: Option<String>,
    pub response_schema: String,
    pub response_example: Option<String>,
    pub examples: Vec<ContractExampleRow>,
    pub audit: Vec<AuditRow>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct AuditRow {
    pub actor: String,
    pub action: String,
    pub created_at: String,
}

/// Search results page.
#[derive(Template, Serialize)]
#[template(path = "search.html")]
pub struct SearchTpl<'a> {
    pub title: &'a str,
    pub slug: &'a str,
    pub query: &'a str,
    pub mode: &'a str,
    pub results: Vec<SearchRow>,
}

/// One row in the search results list.
#[derive(Serialize)]
pub struct SearchRow {
    pub id: String,
    pub method: String,
    pub path: String,
    pub summary: String,
    pub status: String,
    pub similarity: Option<f32>,
}

#[derive(Template, Serialize)]
#[template(path = "login.html")]
pub struct LoginTpl<'a> {
    pub next: &'a str,
    pub error: Option<&'a str>,
}

#[derive(Template, Serialize)]
#[template(path = "oauth_consent.html")]
pub struct OAuthConsentTpl<'a> {
    pub client_name: &'a str,
    pub username: &'a str,
    pub client_id: &'a str,
    pub redirect_uri: &'a str,
    pub code_challenge: &'a str,
    pub code_challenge_method: &'a str,
    pub scope: &'a str,
    pub state: &'a str,
    pub csrf: &'a str,
}

#[derive(Template, Serialize)]
#[template(path = "admin_users.html")]
pub struct AdminUsersTpl<'a> {
    pub users: Vec<AdminUserRow>,
    pub csrf: &'a str,
    pub error: Option<&'a str>,
}

#[derive(Serialize)]
pub struct AdminUserRow {
    pub username: String,
    pub role: String,
    pub status: String,
}

#[derive(Template, Serialize)]
#[template(path = "audit.html")]
pub struct AuditTpl {
    pub entries: Vec<AuditPageRow>,
}

#[derive(Serialize)]
pub struct AuditPageRow {
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub created_at: String,
}
