//! Project create + list use cases.

use crate::domain::errors::DomainError;
use crate::domain::ports::Repos;
use crate::domain::project::{Project, ProjectCreate, ProjectSummary};

/// Create a new project. Validates the input, then delegates.
pub async fn create(repos: &dyn Repos, input: ProjectCreate) -> Result<Project, DomainError> {
    input.validate()?;
    repos.projects().create(&input).await
}

/// List all projects with contract counts.
pub async fn list(repos: &dyn Repos) -> Result<Vec<ProjectSummary>, DomainError> {
    repos.projects().list_with_counts().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::project::ProjectSlug;
    use crate::infra::db::pool::open_memory;
    use crate::infra::repos::SqliteRepos;

    #[tokio::test]
    async fn create_validates_then_persists() {
        let db = open_memory().unwrap();
        let repos = SqliteRepos::new(db);
        let p = create(
            &repos,
            ProjectCreate {
                slug: ProjectSlug::parse("api").unwrap(),
                name: "API".to_owned(),
                description: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(p.slug.as_str(), "api");
    }

    #[tokio::test]
    async fn create_rejects_empty_name() {
        let db = open_memory().unwrap();
        let repos = SqliteRepos::new(db);
        let err = create(
            &repos,
            ProjectCreate {
                slug: ProjectSlug::parse("api").unwrap(),
                name: String::new(),
                description: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DomainError::Validation { .. }));
    }

    #[tokio::test]
    async fn list_returns_in_created_at_desc_order() {
        let db = open_memory().unwrap();
        let repos = SqliteRepos::new(db);
        create(
            &repos,
            ProjectCreate {
                slug: ProjectSlug::parse("a").unwrap(),
                name: "A".to_owned(),
                description: None,
            },
        )
        .await
        .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        create(
            &repos,
            ProjectCreate {
                slug: ProjectSlug::parse("b").unwrap(),
                name: "B".to_owned(),
                description: None,
            },
        )
        .await
        .unwrap();
        let list = list(&repos).await.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].slug.as_str(), "b");
    }
}
