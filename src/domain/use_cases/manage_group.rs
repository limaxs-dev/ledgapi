//! Group resolution + listing.

use crate::domain::errors::DomainError;
use crate::domain::group::{Group, GroupRef, GroupSummary};
use crate::domain::ports::Repos;

pub async fn resolve(
    repos: &dyn Repos,
    project_id: crate::core::id::Id,
    input: GroupRef,
) -> Result<Group, DomainError> {
    input.validate()?;
    repos.groups().resolve(project_id, &input).await
}

pub async fn list(
    repos: &dyn Repos,
    project_id: crate::core::id::Id,
) -> Result<Vec<GroupSummary>, DomainError> {
    repos.groups().list_with_counts(project_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::project::{ProjectCreate, ProjectSlug};
    use crate::domain::use_cases::manage_project;
    use crate::infra::db::pool::open_memory;
    use crate::infra::repos::SqliteRepos;

    #[tokio::test]
    async fn resolve_creates_group() {
        let db = open_memory().unwrap();
        let repos = SqliteRepos::new(db);
        let p = manage_project::create(
            &repos,
            ProjectCreate {
                slug: ProjectSlug::parse("api").unwrap(),
                name: "API".to_owned(),
                description: None,
            },
        )
        .await
        .unwrap();
        let g = resolve(
            &repos,
            p.id,
            GroupRef { name: "Auth".to_owned(), description: None, parent_id: None },
        )
        .await
        .unwrap();
        assert_eq!(g.name, "Auth");
    }

    #[tokio::test]
    async fn resolve_rejects_empty_name() {
        let db = open_memory().unwrap();
        let repos = SqliteRepos::new(db);
        let p = manage_project::create(
            &repos,
            ProjectCreate {
                slug: ProjectSlug::parse("api").unwrap(),
                name: "API".to_owned(),
                description: None,
            },
        )
        .await
        .unwrap();
        let err = resolve(
            &repos,
            p.id,
            GroupRef { name: String::new(), description: None, parent_id: None },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DomainError::Validation { .. }));
    }
}
