//! SQLite adapter for [`GroupRepo`].

use crate::core::id::Id;
use crate::domain::errors::DomainError;
use crate::domain::group::{Group, GroupRef, GroupSummary};
use crate::domain::ports::GroupRepo;
use crate::infra::db::Db;
use crate::infra::repos::project_repo_sqlite::parse_id;
use async_trait::async_trait;
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
                let inserted = c.execute(
                    "INSERT OR IGNORE INTO groups (id, project_id, name, description) VALUES (?1, ?2, ?3, ?4)",
                    params![id.to_string(), project_id.to_string(), input.name, input.description],
                ).map_err(|e| DomainError::Internal(e.to_string()))?;

                if inserted == 0 {
                    let existing = c.query_row(
                        "SELECT id, name, description FROM groups WHERE project_id = ?1 AND name = ?2",
                        params![project_id.to_string(), input.name],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?)),
                    ).map_err(|e| DomainError::Internal(e.to_string()))?;
                    return Ok(Group {
                        id: parse_id(&existing.0)?,
                        project_id,
                        name: existing.1,
                        description: existing.2,
                    });
                }

                Ok(Group {
                    id,
                    project_id,
                    name: input.name,
                    description: input.description,
                })
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
                        "SELECT g.id, g.name, COUNT(c.id)
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
                        let count: i64 = r.get(2)?;
                        Ok((id, name, count))
                    })
                    .map_err(|e| DomainError::Internal(e.to_string()))?;

                let mut out = Vec::new();
                for row in rows {
                    let (id, name, count) =
                        row.map_err(|e| DomainError::Internal(e.to_string()))?;
                    out.push(GroupSummary { id: parse_id(&id)?, name, contract_count: count });
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

    #[tokio::test]
    async fn resolve_creates_then_finds() {
        let (db, pid) = setup().await;
        let repo = SqliteGroupRepo { db };
        let g1 = repo
            .resolve(pid, &GroupRef { name: "Auth".to_owned(), description: Some("d".to_owned()) })
            .await
            .unwrap();
        let g2 = repo
            .resolve(pid, &GroupRef { name: "Auth".to_owned(), description: None })
            .await
            .unwrap();
        assert_eq!(g1.id, g2.id);
        assert_eq!(g1.name, "Auth");
    }

    #[tokio::test]
    async fn list_with_counts_includes_zero() {
        let (db, pid) = setup().await;
        let repo = SqliteGroupRepo { db };
        repo.resolve(pid, &GroupRef { name: "A".to_owned(), description: None }).await.unwrap();
        repo.resolve(pid, &GroupRef { name: "B".to_owned(), description: None }).await.unwrap();
        let list = repo.list_with_counts(pid).await.unwrap();
        assert_eq!(list.len(), 2);
    }
}
