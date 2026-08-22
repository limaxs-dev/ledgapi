//! `search_contract` — hybrid (exact + semantic) with RRF merge.

use crate::config::EmbedConfig;
use crate::core::id::Id;
use crate::domain::errors::DomainError;
use crate::domain::ports::{Embedder, Repos, SearchMode, SearchResult};
use std::collections::HashMap;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub async fn execute(
    repos: &dyn Repos,
    embedder: Arc<dyn Embedder>,
    embed_cfg: &EmbedConfig,
    project_slug: crate::domain::project::ProjectSlug,
    query: &str,
    mode: SearchMode,
    group_name: Option<&str>,
    limit: Option<i64>,
) -> Result<Vec<SearchResult>, DomainError> {
    if query.trim().is_empty() {
        return Err(DomainError::Validation {
            field: "query".to_owned(),
            message: "must be non-empty".to_owned(),
        });
    }

    let project = repos
        .projects()
        .find_by_slug(&project_slug)
        .await?
        .ok_or(DomainError::NotFound { resource: "project" })?;

    let group_id = match group_name {
        Some(name) if !name.is_empty() => Some(
            repos
                .groups()
                .resolve(
                    project.id,
                    &crate::domain::group::GroupRef { name: name.to_owned(), description: None },
                )
                .await?
                .id,
        ),
        _ => None,
    };

    let limit = limit.unwrap_or(embed_cfg.hybrid_limit).clamp(1, 500);

    let exact = if matches!(mode, SearchMode::Exact | SearchMode::Hybrid) {
        repos.contracts().search_exact(project.id, group_id, query, 50).await?
    } else {
        vec![]
    };

    let semantic = if matches!(mode, SearchMode::Semantic | SearchMode::Hybrid) {
        let text = format!("{project_slug} {query}");
        let emb = embedder.embed(&text).await?;
        let k = 50;
        repos.contracts().search_semantic(project.id, group_id, &emb, k).await?
    } else {
        vec![]
    };

    Ok(rrf_merge(exact, semantic, limit))
}

fn rrf_merge(exact: Vec<SearchResult>, semantic: Vec<(Id, f32)>, limit: i64) -> Vec<SearchResult> {
    const K: f32 = 60.0;

    let mut scores: HashMap<Id, f32> = HashMap::new();
    let mut by_id: HashMap<Id, SearchResult> = HashMap::new();
    let mut sims: HashMap<Id, f32> = HashMap::new();

    for (rank, r) in exact.iter().enumerate() {
        *scores.entry(r.id).or_insert(0.0) += 1.0 / (K + rank as f32 + 1.0);
        by_id.entry(r.id).or_insert_with(|| r.clone());
    }

    for (rank, (id, sim)) in semantic.iter().enumerate() {
        *scores.entry(*id).or_insert(0.0) += 1.0 / (K + rank as f32 + 1.0);
        sims.insert(*id, *sim);
    }

    // Hydrate any semantic-only ids.
    for (id, sim) in &semantic {
        by_id.entry(*id).or_insert_with(|| SearchResult {
            id: *id,
            method: crate::domain::contract::Method::Get,
            path: String::new(),
            summary: String::new(),
            status: crate::domain::contract::Status::Draft,
            tags: vec![],
            similarity: Some(*sim),
        });
    }

    let mut all: Vec<(Id, f32)> = scores.into_iter().collect();
    all.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut out: Vec<SearchResult> = Vec::new();
    for (id, _score) in all.into_iter().take(limit as usize) {
        if let Some(mut r) = by_id.remove(&id) {
            r.similarity = sims.get(&id).copied();
            out.push(r);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contract::{Method, Status};

    #[test]
    fn rrf_merge_combines_lists() {
        let exact = vec![
            SearchResult {
                id: id(1),
                method: Method::Get,
                path: "/a".into(),
                summary: String::new(),
                status: Status::Draft,
                tags: vec![],
                similarity: None,
            },
            SearchResult {
                id: id(2),
                method: Method::Get,
                path: "/b".into(),
                summary: String::new(),
                status: Status::Draft,
                tags: vec![],
                similarity: None,
            },
        ];
        let semantic = vec![(id(2), 0.9), (id(3), 0.8)];
        let merged = rrf_merge(exact, semantic, 10);
        // id 2 appears in both → highest score.
        assert_eq!(merged[0].id, id(2));
        // id 3 is from semantic only.
        assert!(merged.iter().any(|r| r.id == id(3)));
    }

    #[test]
    fn rrf_respects_limit() {
        let exact: Vec<SearchResult> = (0..20)
            .map(|i| SearchResult {
                id: id(i),
                method: Method::Get,
                path: format!("/{i}"),
                summary: String::new(),
                status: Status::Draft,
                tags: vec![],
                similarity: None,
            })
            .collect();
        let merged = rrf_merge(exact, vec![], 5);
        assert_eq!(merged.len(), 5);
    }

    fn id(n: u8) -> Id {
        // v7 UUIDs from a fixed byte pattern (test-only).
        let mut bytes = [0_u8; 16];
        bytes[0] = n;
        bytes[6] = 0x70 | (n >> 4) & 0x0F; // version 7
        let u = uuid::Uuid::from_bytes(bytes);
        Id::new_v7(u)
    }
}
