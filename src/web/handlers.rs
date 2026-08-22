//! Web route handlers — thin: parse, call use_case, render.

use crate::core::id::Id;
use crate::domain::ports::ListContractsFilter;
use crate::domain::project::ProjectSlug;
use crate::state::AppState;
use crate::web::templates::{
    ContractRow as CRow, ContractTpl, DashboardTpl, GroupRow, NotFoundTpl, ProjectRow as PRow,
    ProjectTpl, SearchRow, SearchTpl,
};
use askama::Template;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use time::OffsetDateTime;

/// `GET /` — list every project with its contract count.
pub async fn dashboard(Extension(state): Extension<AppState>) -> Response {
    let list = crate::domain::use_cases::manage_project::list(state.repos())
        .await
        .unwrap_or_default();
    let rows: Vec<PRow> = list
        .into_iter()
        .map(|p| PRow {
            slug: p.slug.as_str().to_owned(),
            name: p.name,
            contract_count: p.contract_count,
        })
        .collect();
    let tpl = DashboardTpl {
        title: "Projects",
        projects: rows,
    };
    tpl.render().unwrap_or_default().into_response()
}

/// `GET /projects/{slug}` — project detail (contracts + groups + search box).
pub async fn project(
    Extension(state): Extension<AppState>,
    Path(slug): Path<String>,
) -> Response {
    let Ok(slug) = ProjectSlug::parse(&slug) else {
        return not_found().await;
    };
    let Some(project) = state.repos().projects().find_by_slug(&slug).await.ok().flatten() else {
        return not_found().await;
    };

    let contracts = crate::domain::use_cases::create_contract::list(
        state.repos(),
        project.slug.clone(),
        ListContractsFilter {
            limit: 100,
            ..Default::default()
        },
    )
    .await
    .unwrap_or_default();

    let groups = crate::domain::use_cases::manage_group::list(state.repos(), project.id)
        .await
        .unwrap_or_default();

    let contract_rows: Vec<CRow> = contracts
        .into_iter()
        .map(|c| CRow {
            id: c.id.to_string(),
            method: c.method.as_str().to_owned(),
            path: c.path,
            summary: c.summary,
            status: c.status.as_str().to_owned(),
            group: String::new(),
        })
        .collect();
    let group_rows: Vec<GroupRow> = groups
        .into_iter()
        .map(|g| GroupRow {
            name: g.name,
            contract_count: g.contract_count,
        })
        .collect();

    let tpl = ProjectTpl {
        title: project.name.as_str(),
        slug: project.slug.as_str(),
        name: project.name.as_str(),
        groups: group_rows,
        contracts: contract_rows,
    };
    tpl.render().unwrap_or_default().into_response()
}

/// `GET /projects/{slug}/contracts/{id}` — contract detail.
pub async fn contract(
    Extension(state): Extension<AppState>,
    Path((slug, id)): Path<(String, String)>,
) -> Response {
    let Ok(slug) = ProjectSlug::parse(&slug) else {
        return not_found().await;
    };
    let Some(id) = Id::parse(&id) else {
        return not_found().await;
    };

    let Ok(c) = crate::domain::use_cases::create_contract::get(state.repos(), slug, id).await else {
        return not_found().await;
    };

    let tpl = ContractTpl {
        title: c.summary.clone(),
        id: c.id.to_string(),
        method: c.method.as_str().to_owned(),
        path: c.path,
        summary: c.summary,
        status: c.status.as_str().to_owned(),
        description: c.description,
        auth_type: c.auth_type.map(|a| a.as_str().to_owned()),
        tags: c.tags,
        request_headers: c.request_headers.map(json_pretty),
        request_params: c.request_params.map(json_pretty),
        request_body_schema: c.request_body_schema.map(json_pretty),
        request_example: c.request_example.map(json_pretty),
        response_schema: json_pretty(c.response_schema),
        response_example: c.response_example.map(json_pretty),
        created_at: format_dt(c.created_at),
        updated_at: format_dt(c.updated_at),
    };
    tpl.render().unwrap_or_default().into_response()
}

/// Query string for `GET /projects/{slug}/search`.
#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "hybrid".to_owned()
}

/// `GET /projects/{slug}/search` — search a project's contracts.
pub async fn search(
    Extension(state): Extension<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<SearchQuery>,
) -> Response {
    let Ok(slug_parsed) = ProjectSlug::parse(&slug) else {
        return not_found().await;
    };
    let Ok(mode) = crate::domain::ports::SearchMode::parse(&q.mode) else {
        return not_found().await;
    };
    let results = crate::domain::use_cases::search_contract::execute(
        state.repos(),
        state.embedder(),
        state.embed_cfg(),
        slug_parsed.clone(),
        &q.q,
        mode,
        None,
        Some(20),
    )
    .await
    .unwrap_or_default();

    let rows: Vec<SearchRow> = results
        .into_iter()
        .map(|r| SearchRow {
            id: r.id.to_string(),
            method: r.method.as_str().to_owned(),
            path: r.path,
            summary: r.summary,
            status: r.status.as_str().to_owned(),
            similarity: r.similarity,
        })
        .collect();

    let tpl = SearchTpl {
        title: "Search",
        slug: slug_parsed.as_str(),
        query: &q.q,
        mode: &q.mode,
        results: rows,
    };
    tpl.render().unwrap_or_default().into_response()
}

/// Fallback 404 handler.
pub async fn not_found() -> Response {
    let tpl = NotFoundTpl { path: "" };
    (StatusCode::NOT_FOUND, tpl.render().unwrap_or_default()).into_response()
}

/// Pretty-print a JSON value, falling back to a placeholder on error.
fn json_pretty(v: serde_json::Value) -> String {
    serde_json::to_string_pretty(&v).unwrap_or_else(|_| "<invalid json>".to_owned())
}

/// RFC3339 string for a datetime, falling back to `Display` on format error.
fn format_dt(dt: OffsetDateTime) -> String {
    dt.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| dt.to_string())
}
