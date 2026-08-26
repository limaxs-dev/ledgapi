//! Web route handlers — thin: parse, call use_case, render.

use std::collections::HashMap;

use crate::core::id::Id;
use crate::domain::group::GroupSummary;
use crate::domain::ports::ListContractsFilter;
use crate::domain::project::ProjectSlug;
use crate::state::AppState;
use crate::web::templates::{
    AuditRow, ContractExampleRow, ContractRow as CRow, ContractTpl, DashboardTpl, GroupNode,
    NotFoundTpl, ProjectRow as PRow, ProjectTpl, SearchRow, SearchTpl,
};
use askama::Template;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use time::OffsetDateTime;

/// `GET /` — list every project with its contract count.
pub async fn dashboard(Extension(state): Extension<AppState>) -> Response {
    let list =
        crate::domain::use_cases::manage_project::list(state.repos()).await.unwrap_or_default();
    let rows: Vec<PRow> = list
        .into_iter()
        .map(|p| PRow {
            slug: p.slug.as_str().to_owned(),
            name: p.name,
            contract_count: p.contract_count,
        })
        .collect();
    let tpl = DashboardTpl { title: "Projects", projects: rows };
    Html(tpl.render().unwrap_or_default()).into_response()
}

/// `GET /projects/{slug}` — project detail (contracts + groups + search box).
pub async fn project(Extension(state): Extension<AppState>, Path(slug): Path<String>) -> Response {
    let Ok(slug) = ProjectSlug::parse(&slug) else {
        return not_found().await;
    };
    let Some(project) = state.repos().projects().find_by_slug(&slug).await.ok().flatten() else {
        return not_found().await;
    };

    let contracts = crate::domain::use_cases::create_contract::list(
        state.repos(),
        project.slug.clone(),
        ListContractsFilter { limit: 100, ..Default::default() },
    )
    .await
    .unwrap_or_default();

    let groups = crate::domain::use_cases::manage_group::list(state.repos(), project.id)
        .await
        .unwrap_or_default();

    let total_contracts = contracts.len();
    let total_groups = groups.len();

    // Build the nested group tree. Group names are unique within a
    // project (UNIQUE(project_id, name) on `groups`), so we look up the
    // owning group's contracts by its name.
    let mut by_group_name: HashMap<String, Vec<CRow>> = HashMap::new();
    for c in &contracts {
        let row = CRow {
            id: c.id.to_string(),
            method: c.method.as_str().to_owned(),
            path: c.path.clone(),
            summary: c.summary.clone(),
            status: c.status.as_str().to_owned(),
            group: c.group_name.clone().unwrap_or_default(),
        };
        if !row.group.is_empty() {
            by_group_name.entry(row.group.clone()).or_default().push(row);
        }
    }
    // Stable ordering: by method, then path.
    for v in by_group_name.values_mut() {
        v.sort_by(|a, b| a.method.cmp(&b.method).then(a.path.cmp(&b.path)));
    }

    let mut by_parent: HashMap<Id, Vec<Id>> = HashMap::new();
    for g in &groups {
        if let Some(p) = g.parent_id {
            by_parent.entry(p).or_default().push(g.id);
        }
    }
    let name_of = |id: Id| -> &str {
        groups.iter().find(|g| g.id == id).map(|g| g.name.as_str()).unwrap_or("")
    };
    for v in by_parent.values_mut() {
        v.sort_by(|a, b| name_of(*a).cmp(name_of(*b)));
    }

    let group_tree = build_group_tree(&groups, &by_parent, &by_group_name, project.slug.as_str());
    let mut group_tree_html = String::new();
    for root in &group_tree {
        group_tree_html.push_str(&root.render().unwrap_or_default());
    }

    let tpl = ProjectTpl {
        title: project.name.as_str(),
        slug: project.slug.as_str(),
        name: project.name.as_str(),
        group_tree_html,
        total_contracts,
        total_groups,
    };
    Html(tpl.render().unwrap_or_default()).into_response()
}

/// Build a forest of `GroupNode` from a project's flat group list.
/// `by_parent` maps parent id → child ids (already sorted by name).
/// `by_group_name` maps group name → contracts belonging to that group.
///
/// Children are pre-rendered to HTML bottom-up because Askama 0.12
/// cannot recurse via `child.render()?` and apply `| safe` together
/// (the `?` strips the `Result` wrapping before `| safe` sees it, but
/// Askama re-escapes the resulting `String`).
fn build_group_tree(
    groups: &[GroupSummary],
    by_parent: &HashMap<Id, Vec<Id>>,
    by_group_name: &HashMap<String, Vec<CRow>>,
    slug: &str,
) -> Vec<GroupNode> {
    fn build(
        id: Id,
        depth: usize,
        groups: &[GroupSummary],
        by_parent: &HashMap<Id, Vec<Id>>,
        by_group_name: &HashMap<String, Vec<CRow>>,
        slug: &str,
    ) -> GroupNode {
        let summary = groups.iter().find(|g| g.id == id).expect("group missing");
        // Render children bottom-up.
        let mut children_html = String::new();
        if let Some(child_ids) = by_parent.get(&id) {
            for cid in child_ids {
                let child = build(*cid, depth + 1, groups, by_parent, by_group_name, slug);
                children_html.push_str(&child.render().unwrap_or_default());
            }
        }
        let contracts = by_group_name.get(&summary.name).cloned().unwrap_or_default();
        GroupNode {
            id: summary.id.to_string(),
            name: summary.name.clone(),
            depth,
            slug: slug.to_owned(),
            contracts,
            children_html,
        }
    }

    let mut roots: Vec<GroupNode> = groups
        .iter()
        .filter(|g| g.parent_id.is_none())
        .map(|g| build(g.id, 0, groups, by_parent, by_group_name, slug))
        .collect();
    roots.sort_by(|a, b| a.name.cmp(&b.name));
    roots
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

    let Ok(c) = crate::domain::use_cases::create_contract::get(state.repos(), slug, id).await
    else {
        return not_found().await;
    };

    let audit = state
        .repos()
        .audit()
        .list_for_resource(crate::domain::audit::AuditResource::Contract, c.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|entry| AuditRow {
            actor: entry.actor_username.unwrap_or_else(|| "system".to_owned()),
            action: entry.action.as_str().to_owned(),
            created_at: format_dt(entry.created_at),
        })
        .collect();

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
        examples: c
            .examples
            .into_iter()
            .map(|example| ContractExampleRow {
                name: example.name,
                kind: example.kind.as_str().to_owned(),
                status_code: example.status_code,
                request: json_pretty(example.request),
                response: json_pretty(example.response),
            })
            .collect(),
        audit,
        created_at: format_dt(c.created_at),
        updated_at: format_dt(c.updated_at),
    };
    Html(tpl.render().unwrap_or_default()).into_response()
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
    let results = match crate::domain::use_cases::search_contract::execute(
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
    {
        Ok(results) => results,
        Err(error) => return crate::errors::AppError::from(error).into_response(),
    };

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
    Html(tpl.render().unwrap_or_default()).into_response()
}

/// Fallback 404 handler.
pub async fn not_found() -> Response {
    let tpl = NotFoundTpl { path: "" };
    (StatusCode::NOT_FOUND, Html(tpl.render().unwrap_or_default())).into_response()
}

/// Pretty-print a JSON value, falling back to a placeholder on error.
fn json_pretty(v: serde_json::Value) -> String {
    serde_json::to_string_pretty(&v).unwrap_or_else(|_| "<invalid json>".to_owned())
}

/// RFC3339 string for a datetime, falling back to `Display` on format error.
pub(crate) fn format_dt(dt: OffsetDateTime) -> String {
    dt.format(&time::format_description::well_known::Rfc3339).unwrap_or_else(|_| dt.to_string())
}
