//! SQLite adapter for [`GroupRepo`].

use crate::core::id::Id;
use crate::domain::errors::DomainError;
use crate::domain::group::{Group, GroupRef, GroupSummary};
use crate::domain::ports::{GroupRepo, GroupResolution};
use crate::infra::db::Db;
use crate::infra::repos::project_repo_sqlite::parse_id;
use async_trait::async_trait;
use rusqlite::OptionalExtension;
use rusqlite::params;

/// `groups` adapter. Implements [`GroupRepo`] against SQLite.
pub struct SqliteGroupRepo {
    pub(crate) db: Db,
}

#[async_trait]
impl GroupRepo for SqliteGroupRepo {
    async fn resolve(&self, project_id: Id, input: &GroupRef) -> Result<Group, DomainError> {
        let db = self.db.clone();
        let input = input.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                // Try insert; on UNIQUE conflict, return the existing row.
                let id = Id::new();
                let parent_str = input.parent_id.map(|p| p.to_string());
                let inserted = c.execute(
                    "INSERT OR IGNORE INTO groups (id, project_id, name, description, parent_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id.to_string(), project_id.to_string(), input.name, input.description, parent_str],
                ).map_err(|e| DomainError::Internal(e.to_string()))?;

                if inserted == 0 {
                    let existing = c.query_row(
                        "SELECT id, name, description, parent_id FROM groups WHERE project_id = ?1 AND name = ?2",
                        params![project_id.to_string(), input.name],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, Option<String>>(3)?)),
                    ).map_err(|e| DomainError::Internal(e.to_string()))?;
                    return Ok(Group {
                        id: parse_id(&existing.0)?,
                        project_id,
                        name: existing.1,
                        description: existing.2,
                        parent_id: existing.3.as_deref().and_then(Id::parse),
                    });
                }

                Ok(Group {
                    id,
                    project_id,
                    name: input.name,
                    description: input.description,
                    parent_id: input.parent_id,
                })
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn resolve_with_created(
        &self,
        project_id: Id,
        input: &GroupRef,
    ) -> Result<GroupResolution, DomainError> {
        let db = self.db.clone();
        let input = input.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let id = Id::new();
                let parent_str = input.parent_id.map(|p| p.to_string());
                let inserted = c
                    .execute(
                        "INSERT OR IGNORE INTO groups (id, project_id, name, description, parent_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![id.to_string(), project_id.to_string(), input.name, input.description, parent_str],
                    )
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                if inserted != 0 {
                    return Ok(GroupResolution {
                        group: Group {
                            id,
                            project_id,
                            name: input.name,
                            description: input.description,
                            parent_id: input.parent_id,
                        },
                        created: true,
                    });
                }
                let existing = c
                    .query_row(
                        "SELECT id, name, description, parent_id FROM groups WHERE project_id = ?1 AND name = ?2",
                        params![project_id.to_string(), input.name],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, Option<String>>(3)?)),
                    )
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                Ok(GroupResolution {
                    group: Group {
                        id: parse_id(&existing.0)?,
                        project_id,
                        name: existing.1,
                        description: existing.2,
                        parent_id: existing.3.as_deref().and_then(Id::parse),
                    },
                    created: false,
                })
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn find_by_name(&self, project_id: Id, name: &str) -> Result<Option<Group>, DomainError> {
        let db = self.db.clone();
        let name = name.to_owned();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| -> Result<Option<Group>, DomainError> {
                let row = c
                    .query_row(
                        "SELECT id, name, description, parent_id FROM groups
                         WHERE project_id = ?1 AND name = ?2",
                        params![project_id.to_string(), name],
                        |r| {
                            Ok((
                                r.get::<_, String>(0)?,
                                r.get::<_, String>(1)?,
                                r.get::<_, Option<String>>(2)?,
                                r.get::<_, Option<String>>(3)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                let group = match row {
                    Some((id, n, desc, parent)) => Some(Group {
                        id: parse_id(&id)?,
                        project_id,
                        name: n,
                        description: desc,
                        parent_id: parent.as_deref().and_then(Id::parse),
                    }),
                    None => None,
                };
                Ok(group)
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn list_with_counts(&self, project_id: Id) -> Result<Vec<GroupSummary>, DomainError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let mut stmt = c
                    .prepare(
                        "SELECT g.id, g.name, g.parent_id, COUNT(c.id)
                     FROM groups g
                     LEFT JOIN contracts c ON c.group_id = g.id
                     WHERE g.project_id = ?1
                     GROUP BY g.id
                     ORDER BY g.name ASC",
                    )
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                let rows = stmt
                    .query_map([project_id.to_string()], |r| {
                        let id: String = r.get(0)?;
                        let name: String = r.get(1)?;
                        let parent: Option<String> = r.get(2)?;
                        let count: i64 = r.get(3)?;
                        Ok((id, name, parent, count))
                    })
                    .map_err(|e| DomainError::Internal(e.to_string()))?;

                let mut out = Vec::new();
                for row in rows {
                    let (id, name, parent, count) =
                        row.map_err(|e| DomainError::Internal(e.to_string()))?;
                    let parent_id = parent.as_deref().and_then(Id::parse);
                    out.push(GroupSummary {
                        id: parse_id(&id)?,
                        name,
                        contract_count: count,
                        parent_id,
                    });
                }
                Ok::<_, DomainError>(out)
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::ProjectRepo;
    use crate::domain::project::{ProjectCreate, ProjectSlug};
    use crate::infra::db::pool::open_memory;
    use crate::infra::repos::project_repo_sqlite::SqliteProjectRepo;

    async fn setup() -> (crate::infra::db::Db, Id) {
        let db = open_memory().unwrap();
        let proj = SqliteProjectRepo { db: db.clone() };
        let p = proj
            .create(&ProjectCreate {
                slug: ProjectSlug::parse("api").unwrap(),
                name: "API".to_owned(),
                description: None,
            })
            .await
            .unwrap();
        (db, p.id)
    }

    fn ref0(name: &str) -> GroupRef {
        GroupRef { name: name.to_owned(), description: None, parent_id: None }
    }

    fn refd(name: &str, desc: &str) -> GroupRef {
        GroupRef { name: name.to_owned(), description: Some(desc.to_owned()), parent_id: None }
    }

    #[tokio::test]
    async fn resolve_creates_then_finds() {
        let (db, pid) = setup().await;
        let repo = SqliteGroupRepo { db };
        let g1 = repo.resolve(pid, &refd("Auth", "d")).await.unwrap();
        let g2 = repo.resolve(pid, &ref0("Auth")).await.unwrap();
        assert_eq!(g1.id, g2.id);
        assert_eq!(g1.name, "Auth");
    }

    #[tokio::test]
    async fn list_with_counts_includes_zero() {
        let (db, pid) = setup().await;
        let repo = SqliteGroupRepo { db };
        repo.resolve(pid, &ref0("A")).await.unwrap();
        repo.resolve(pid, &ref0("B")).await.unwrap();
        let list = repo.list_with_counts(pid).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn list_with_counts_returns_parent_id() {
        let (db, pid) = setup().await;
        let repo = SqliteGroupRepo { db };
        let parent = repo.resolve(pid, &ref0("parent")).await.unwrap();
        let child = repo
            .resolve(
                pid,
                &GroupRef {
                    name: "child".to_owned(),
                    description: None,
                    parent_id: Some(parent.id),
                },
            )
            .await
            .unwrap();
        let list = repo.list_with_counts(pid).await.unwrap();
        let child_summary = list.iter().find(|g| g.id == child.id).unwrap();
        assert_eq!(child_summary.parent_id, Some(parent.id));
    }
}
