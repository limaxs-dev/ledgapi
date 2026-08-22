//! `update_contract` — silent overwrite; regenerate embedding when the
//! patch changes method/path/summary/description.

use crate::config::EmbedConfig;
use crate::core::id::Id;
use crate::domain::contract::{Contract, ContractUpdate};
use crate::domain::errors::DomainError;
use crate::domain::ports::{Embedder, Repos};
use std::sync::Arc;

pub async fn execute(
    repos: &dyn Repos,
    embedder: Arc<dyn Embedder>,
    _embed_cfg: &EmbedConfig,
    project_slug: crate::domain::project::ProjectSlug,
    contract_id: Id,
    mut patch: ContractUpdate,
) -> Result<Contract, DomainError> {
    if patch.is_empty() {
        return Err(DomainError::Validation {
            field: "patch".to_owned(),
            message: "at least one field must be set".to_owned(),
        });
    }
    if let Some(ref mut path) = patch.path {
        *path = crate::domain::contract::normalize_path(path);
    }

    let group_id = match patch.group_name.as_deref() {
        Some(name) if !name.is_empty() => Some(
            repos
                .groups()
                .resolve(
                    find_project_id(repos, &project_slug).await?,
                    &crate::domain::group::GroupRef { name: name.to_owned(), description: None },
                )
                .await?
                .id,
        ),
        _ => None,
    };

    let project_id = find_project_id(repos, &project_slug).await?;

    let updated = repos.contracts().update(project_id, contract_id, &patch, group_id).await?;

    if patch.affects_embedding() {
        let text = format!(
            "{} {} {} {}",
            updated.method.as_str(),
            updated.path,
            updated.summary,
            updated.description.as_deref().unwrap_or(""),
        );
        match embedder.embed(&text).await {
            Ok(emb) => {
                if let Err(e) =
                    repos.embeddings().upsert(updated.id, updated.project_id, &emb).await
                {
                    tracing::warn!(error = %e, contract_id = %updated.id, "failed to upsert embedding after update");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "embedding regeneration failed; semantic search may be stale");
            }
        }
    }

    Ok(updated)
}

async fn find_project_id(
    repos: &dyn Repos,
    slug: &crate::domain::project::ProjectSlug,
) -> Result<Id, DomainError> {
    repos
        .projects()
        .find_by_slug(slug)
        .await?
        .map(|p| p.id)
        .ok_or(DomainError::NotFound { resource: "project" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contract::normalize_path;
    use crate::domain::project::{ProjectCreate, ProjectSlug};
    use crate::infra::db::pool::open_memory;
    use crate::infra::embeddings::fastembed_impl::StubEmbedder;
    use crate::infra::repos::SqliteRepos;

    const CFG: EmbedConfig = EmbedConfig {
        cache_dir: String::new(),
        model: String::new(),
        similarity_threshold: 0.85,
        knn_top_k: 5,
        hybrid_limit: 10,
    };

    async fn boot() -> (SqliteRepos, Arc<dyn Embedder>, ProjectSlug, Id) {
        let db = open_memory().unwrap();
        let repos = SqliteRepos::new(db);
        let p = repos
            .projects()
            .create(&ProjectCreate {
                slug: ProjectSlug::parse("api").unwrap(),
                name: "API".to_owned(),
                description: None,
            })
            .await
            .unwrap();
        let created = crate::domain::use_cases::create_contract::execute(
            &repos,
            Arc::new(StubEmbedder::new()),
            &CFG,
            p.slug.clone(),
            crate::domain::use_cases::create_contract::ContractCreate_for_tests(),
        )
        .await
        .unwrap();
        (repos, Arc::new(StubEmbedder::new()), p.slug, created.contract_id)
    }

    // Helper is defined as a free function in `create_contract` because
    // you cannot `impl` a module. (Brief defect — original used
    // `impl crate::domain::use_cases::create_contract { ... }` which is
    // not valid Rust syntax.)

    #[tokio::test]
    async fn update_empty_patch_errors() {
        let (repos, emb, slug, cid) = boot().await;
        let err =
            execute(&repos, emb, &CFG, slug, cid, ContractUpdate::default()).await.unwrap_err();
        assert!(matches!(err, DomainError::Validation { .. }));
    }

    #[tokio::test]
    async fn update_summary_succeeds() {
        let (repos, emb, slug, cid) = boot().await;
        let patch = ContractUpdate { summary: Some("New".to_owned()), ..Default::default() };
        let updated = execute(&repos, emb, &CFG, slug, cid, patch).await.unwrap();
        assert_eq!(updated.summary, "New");
    }

    #[tokio::test]
    async fn update_path_normalizes() {
        let (repos, emb, slug, cid) = boot().await;
        let patch = ContractUpdate { path: Some("/api/users/".to_owned()), ..Default::default() };
        let updated = execute(&repos, emb, &CFG, slug, cid, patch).await.unwrap();
        assert_eq!(updated.path, normalize_path("/api/users/"));
    }
}
