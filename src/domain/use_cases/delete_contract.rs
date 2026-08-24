//! `delete_contract` — also removes the embedding row.

use crate::core::id::Id;
use crate::domain::audit::{AuditAction, AuditResource};
use crate::domain::auth::Principal;
use crate::domain::errors::DomainError;
use crate::domain::ports::Repos;

pub async fn execute(
    repos: &dyn Repos,
    project_slug: crate::domain::project::ProjectSlug,
    contract_id: Id,
) -> Result<(), DomainError> {
    let project = repos
        .projects()
        .find_by_slug(&project_slug)
        .await?
        .ok_or(DomainError::NotFound { resource: "project" })?;

    repos.contracts().delete(project.id, contract_id).await?;
    if let Err(e) = repos.embeddings().delete(contract_id).await {
        tracing::warn!(error = %e, contract_id = %contract_id, "embedding delete failed");
    }
    Ok(())
}

pub async fn execute_with_actor(
    repos: &dyn Repos,
    principal: &Principal,
    project_slug: crate::domain::project::ProjectSlug,
    contract_id: Id,
) -> Result<(), DomainError> {
    principal.require_scope("ledgapi:write")?;
    execute(repos, project_slug, contract_id).await?;
    crate::domain::use_cases::audit::record(
        repos,
        principal,
        AuditAction::Delete,
        AuditResource::Contract,
        Some(contract_id),
        serde_json::json!({"contract_id": contract_id}),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::project::ProjectSlug;
    use crate::infra::db::pool::open_memory;
    use crate::infra::repos::SqliteRepos;

    #[tokio::test]
    async fn delete_unknown_project_errors() {
        let db = open_memory().unwrap();
        let repos = SqliteRepos::new(db);
        let err =
            execute(&repos, ProjectSlug::parse("missing").unwrap(), Id::new()).await.unwrap_err();
        assert!(matches!(err, DomainError::NotFound { .. }));
    }
}
