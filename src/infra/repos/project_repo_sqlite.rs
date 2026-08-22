//! SQLite adapter for [`ProjectRepo`].

use crate::core::id::Id;
use crate::domain::errors::DomainError;
use crate::domain::ports::ProjectRepo;
use crate::domain::project::{Project, ProjectCreate, ProjectSlug, ProjectSummary};
use crate::infra::db::Db;
use async_trait::async_trait;
use rusqlite::{OptionalExtension, params};
use time::OffsetDateTime;

/// `projects` adapter. Implements [`ProjectRepo`] against SQLite.
pub struct SqliteProjectRepo {
    pub(crate) db: Db,
}

#[async_trait]
impl ProjectRepo for SqliteProjectRepo {
    async fn create(&self, input: &ProjectCreate) -> Result<Project, DomainError> {
        let db = self.db.clone();
        let input = input.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let id = Id::new();
                let now = OffsetDateTime::now_utc().unix_timestamp();
                c.execute(
                    "INSERT INTO projects (id, slug, name, description, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        id.to_string(),
                        input.slug.as_str(),
                        input.name,
                        input.description,
                        now
                    ],
                )
                .map_err(|e| match e {
                    rusqlite::Error::SqliteFailure(err, _)
                        if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                    {
                        DomainError::DuplicateKey {
                            resource: "project",
                            key: input.slug.to_string(),
                        }
                    }
                    _ => DomainError::Internal(e.to_string()),
                })?;
                Ok(Project {
                    id,
                    slug: input.slug,
                    name: input.name,
                    description: input.description,
                    created_at: OffsetDateTime::from_unix_timestamp(now)
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                })
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn find_by_slug(&self, slug: &ProjectSlug) -> Result<Option<Project>, DomainError> {
        let db = self.db.clone();
        let slug = slug.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let row = c
                    .query_row(
                        "SELECT id, slug, name, description, created_at
                         FROM projects WHERE slug = ?1",
                        [slug.as_str()],
                        |r| {
                            Ok((
                                r.get::<_, String>(0)?,
                                r.get::<_, String>(1)?,
                                r.get::<_, String>(2)?,
                                r.get::<_, Option<String>>(3)?,
                                r.get::<_, i64>(4)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                row.map(|(id, slug, name, description, ts)| -> Result<Project, DomainError> {
                    Ok(Project {
                        id: parse_id(&id)?,
                        slug: ProjectSlug::parse(&slug)
                            .map_err(|e| DomainError::Internal(e.to_string()))?,
                        name,
                        description,
                        created_at: OffsetDateTime::from_unix_timestamp(ts)
                            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                    })
                })
                .transpose()
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn find_by_id(&self, id: Id) -> Result<Option<Project>, DomainError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let row = c
                    .query_row(
                        "SELECT id, slug, name, description, created_at
                         FROM projects WHERE id = ?1",
                        [id.to_string()],
                        |r| {
                            Ok((
                                r.get::<_, String>(0)?,
                                r.get::<_, String>(1)?,
                                r.get::<_, String>(2)?,
                                r.get::<_, Option<String>>(3)?,
                                r.get::<_, i64>(4)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                row.map(|(id, slug, name, description, ts)| -> Result<Project, DomainError> {
                    Ok(Project {
                        id: parse_id(&id)?,
                        slug: ProjectSlug::parse(&slug)
                            .map_err(|e| DomainError::Internal(e.to_string()))?,
                        name,
                        description,
                        created_at: OffsetDateTime::from_unix_timestamp(ts)
                            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                    })
                })
                .transpose()
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn list_with_counts(&self) -> Result<Vec<ProjectSummary>, DomainError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let mut stmt = c
                    .prepare(
                        "SELECT p.slug, p.name, COUNT(c.id)
                         FROM projects p
                         LEFT JOIN contracts c ON c.project_id = p.id
                         GROUP BY p.id
                         ORDER BY p.created_at DESC",
                    )
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                let rows = stmt
                    .query_map([], |r| {
                        let slug: String = r.get(0)?;
                        let name: String = r.get(1)?;
                        let count: i64 = r.get(2)?;
                        Ok((slug, name, count))
                    })
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                let mut out = Vec::new();
                for row in rows {
                    let (slug, name, count) =
                        row.map_err(|e| DomainError::Internal(e.to_string()))?;
                    out.push(ProjectSummary {
                        slug: ProjectSlug::parse(&slug)
                            .map_err(|e| DomainError::Internal(e.to_string()))?,
                        name,
                        contract_count: count,
                    });
                }
                Ok::<_, DomainError>(out)
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }
}

/// Parse a string into an [`Id`], wrapping internal failures as
/// [`DomainError::Internal`].
pub(crate) fn parse_id(s: &str) -> Result<Id, DomainError> {
    Id::parse(s).ok_or_else(|| DomainError::Internal(format!("invalid UUID: {s}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::db::pool::open_memory;

    fn pc(slug: &str, name: &str) -> ProjectCreate {
        ProjectCreate {
            slug: ProjectSlug::parse(slug).unwrap(),
            name: name.to_owned(),
            description: None,
        }
    }

    #[tokio::test]
    async fn create_then_find_by_slug() {
        let db = open_memory().unwrap();
        let repo = SqliteProjectRepo { db };
        let p = repo.create(&pc("api", "My API")).await.unwrap();
        let found = repo.find_by_slug(&p.slug).await.unwrap().unwrap();
        assert_eq!(found.id, p.id);
        assert_eq!(found.name, "My API");
    }

    #[tokio::test]
    async fn duplicate_slug_errors() {
        let db = open_memory().unwrap();
        let repo = SqliteProjectRepo { db };
        repo.create(&pc("api", "My API")).await.unwrap();
        let err = repo.create(&pc("api", "Other")).await.unwrap_err();
        assert!(matches!(err, DomainError::DuplicateKey { .. }));
    }

    #[tokio::test]
    async fn list_with_counts_includes_zero() {
        let db = open_memory().unwrap();
        let repo = SqliteProjectRepo { db };
        repo.create(&pc("a", "A")).await.unwrap();
        repo.create(&pc("b", "B")).await.unwrap();
        let list = repo.list_with_counts().await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|p| p.contract_count == 0));
    }

    #[tokio::test]
    async fn find_by_id_returns_none_for_missing() {
        let db = open_memory().unwrap();
        let repo = SqliteProjectRepo { db };
        let found = repo.find_by_id(Id::new()).await.unwrap();
        assert!(found.is_none());
    }
}
