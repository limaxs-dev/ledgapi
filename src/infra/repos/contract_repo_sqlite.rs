//! SQLite adapter for [`ContractRepo`].

use crate::core::id::Id;
use crate::domain::contract::{
    AuthType, Contract, ContractCreate, ContractSummary, ContractUpdate, Method, Status,
    normalize_path,
};
use crate::domain::errors::DomainError;
use crate::domain::ports::{ContractRepo, ListContractsFilter, SearchResult};
use crate::infra::db::Db;
use crate::infra::repos::project_repo_sqlite::parse_id;
use async_trait::async_trait;
use rusqlite::{OptionalExtension, params};
use time::OffsetDateTime;

/// `contracts` adapter. Implements [`ContractRepo`] against SQLite.
pub struct SqliteContractRepo {
    pub(crate) db: Db,
}

/// Helper: serialize `Option<serde_json::Value>` to `Option<String>` for storage.
fn json_to_text(v: Option<&serde_json::Value>) -> Option<String> {
    v.map(std::string::ToString::to_string)
}

fn parse_json_opt(s: Option<String>) -> Result<Option<serde_json::Value>, DomainError> {
    match s {
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => {
            serde_json::from_str(&s).map(Some).map_err(|e| DomainError::Internal(e.to_string()))
        }
        None => Ok(None),
    }
}

fn parse_json_required(s: String) -> Result<serde_json::Value, DomainError> {
    serde_json::from_str(&s).map_err(|e| DomainError::Internal(e.to_string()))
}

fn parse_method(s: &str) -> Result<Method, DomainError> {
    Method::parse(s).ok_or_else(|| DomainError::Internal(format!("bad method: {s}")))
}

fn parse_status(s: &str) -> Result<Status, DomainError> {
    Status::parse(s).ok_or_else(|| DomainError::Internal(format!("bad status: {s}")))
}

fn parse_auth_type_opt(s: Option<String>) -> Result<Option<AuthType>, DomainError> {
    s.map(|s| match s.as_str() {
        "none" => Ok(AuthType::None),
        "bearer" => Ok(AuthType::Bearer),
        "api_key" => Ok(AuthType::ApiKey),
        "basic" => Ok(AuthType::Basic),
        other => Err(DomainError::Internal(format!("bad auth_type: {other}"))),
    })
    .transpose()
}

fn parse_tags(s: String) -> Result<Vec<String>, DomainError> {
    serde_json::from_str(&s).map_err(|e| DomainError::Internal(e.to_string()))
}

#[async_trait]
impl ContractRepo for SqliteContractRepo {
    async fn create(
        &self,
        project_id: Id,
        group_id: Option<Id>,
        input: &ContractCreate,
    ) -> Result<Contract, DomainError> {
        let db = self.db.clone();
        let input = input.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let id = Id::new();
                let now = OffsetDateTime::now_utc().unix_timestamp();
                let path = normalize_path(&input.path);
                let status = match input.status.as_deref() {
                    None => Status::default(),
                    Some(s) => Status::parse(s)
                        .ok_or_else(|| DomainError::Internal(format!("bad status: {s}")))?,
                };
                let auth = input.auth_type.as_deref().map(AuthType::parse_or_default);
                let tags = input.tags.clone().unwrap_or_default();

                c.execute(
                    "INSERT INTO contracts (
                        id, project_id, group_id, method, path, summary, description,
                        request_headers, request_params, request_body_schema, request_example,
                        response_schema, response_example, auth_type, status, tags,
                        created_at, updated_at
                    ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
                    params![
                        id.to_string(),
                        project_id.to_string(),
                        group_id.map(|g| g.to_string()),
                        input.method.as_str(),
                        path,
                        input.summary,
                        input.description,
                        json_to_text(input.request_headers.as_ref()),
                        json_to_text(input.request_params.as_ref()),
                        json_to_text(input.request_body_schema.as_ref()),
                        json_to_text(input.request_example.as_ref()),
                        input.response_schema.to_string(),
                        json_to_text(input.response_example.as_ref()),
                        auth.map(AuthType::as_str),
                        status.as_str(),
                        serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_owned()),
                        now,
                        now,
                    ],
                )
                .map_err(|e| match e {
                    rusqlite::Error::SqliteFailure(err, _)
                        if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                    {
                        DomainError::DuplicateKey {
                            resource: "contract",
                            key: format!("{} {}", input.method, path),
                        }
                    }
                    _ => DomainError::Internal(e.to_string()),
                })?;

                Ok(Contract {
                    id,
                    project_id,
                    group_id,
                    method: input.method,
                    path,
                    summary: input.summary,
                    description: input.description,
                    request_headers: input.request_headers,
                    request_params: input.request_params,
                    request_body_schema: input.request_body_schema,
                    request_example: input.request_example,
                    response_schema: input.response_schema,
                    response_example: input.response_example,
                    auth_type: auth,
                    status,
                    tags,
                    created_at: OffsetDateTime::from_unix_timestamp(now)
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                    updated_at: OffsetDateTime::from_unix_timestamp(now)
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                })
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn find_by_id(&self, project_id: Id, contract_id: Id) -> Result<Contract, DomainError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                load_contract(c, project_id, contract_id)?
                    .ok_or(DomainError::NotFound { resource: "contract" })
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn update(
        &self,
        project_id: Id,
        contract_id: Id,
        patch: &ContractUpdate,
        group_id: Option<Id>,
    ) -> Result<Contract, DomainError> {
        let db = self.db.clone();
        let patch = patch.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let current = load_contract(c, project_id, contract_id)?
                    .ok_or(DomainError::NotFound { resource: "contract" })?;

                let merged = Contract {
                    id: current.id,
                    project_id: current.project_id,
                    group_id,
                    method: patch.method.unwrap_or(current.method),
                    path: patch.path.as_deref().map(normalize_path).unwrap_or(current.path),
                    summary: patch.summary.unwrap_or(current.summary),
                    description: patch.description.or(current.description),
                    request_headers: patch.request_headers.or(current.request_headers),
                    request_params: patch.request_params.or(current.request_params),
                    request_body_schema: patch.request_body_schema.or(current.request_body_schema),
                    request_example: patch.request_example.or(current.request_example),
                    response_schema: patch.response_schema.unwrap_or(current.response_schema),
                    response_example: patch.response_example.or(current.response_example),
                    auth_type: patch.auth_type.as_deref().map(AuthType::parse_or_default).or(current.auth_type),
                    status: match patch.status.as_deref() {
                        Some(s) => Status::parse(s)
                            .ok_or_else(|| DomainError::Internal(format!("bad status: {s}")))?,
                        None => current.status,
                    },
                    tags: patch.tags.unwrap_or(current.tags),
                    created_at: current.created_at,
                    updated_at: OffsetDateTime::now_utc(),
                };

                c.execute(
                    "UPDATE contracts SET
                        group_id=?1, method=?2, path=?3, summary=?4, description=?5,
                        request_headers=?6, request_params=?7, request_body_schema=?8, request_example=?9,
                        response_schema=?10, response_example=?11, auth_type=?12, status=?13, tags=?14,
                        updated_at=?15
                     WHERE id=?16 AND project_id=?17",
                    params![
                        merged.group_id.map(|g| g.to_string()),
                        merged.method.as_str(),
                        merged.path,
                        merged.summary,
                        merged.description,
                        json_to_text(merged.request_headers.as_ref()),
                        json_to_text(merged.request_params.as_ref()),
                        json_to_text(merged.request_body_schema.as_ref()),
                        json_to_text(merged.request_example.as_ref()),
                        merged.response_schema.to_string(),
                        json_to_text(merged.response_example.as_ref()),
                        merged.auth_type.map(AuthType::as_str),
                        merged.status.as_str(),
                        serde_json::to_string(&merged.tags).unwrap_or_else(|_| "[]".to_owned()),
                        merged.updated_at.unix_timestamp(),
                        merged.id.to_string(),
                        project_id.to_string(),
                    ],
                )
                .map_err(|e| DomainError::Internal(e.to_string()))?;
                // API-006: detect the race where the contract was deleted
                // between load_contract and UPDATE. Returning Ok here would
                // let the use case upsert an orphan embedding row.
                let n = c
                    .query_row(
                        "SELECT COUNT(*) FROM contracts WHERE id = ?1",
                        [merged.id.to_string()],
                        |r| r.get::<_, i64>(0),
                    )
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                if n == 0 {
                    return Err(DomainError::NotFound { resource: "contract" });
                }

                Ok(merged)
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn delete(&self, project_id: Id, contract_id: Id) -> Result<(), DomainError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let n = c
                    .execute(
                        "DELETE FROM contracts WHERE id=?1 AND project_id=?2",
                        params![contract_id.to_string(), project_id.to_string()],
                    )
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                if n == 0 {
                    return Err(DomainError::NotFound { resource: "contract" });
                }
                Ok(())
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn list(
        &self,
        project_id: Id,
        filter: &ListContractsFilter,
    ) -> Result<Vec<ContractSummary>, DomainError> {
        let db = self.db.clone();
        let filter = filter.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let mut sql = String::from(
                    "SELECT c.id, c.method, c.path, c.summary, c.status, c.tags, g.name
                     FROM contracts c
                     LEFT JOIN groups g ON g.id = c.group_id
                     WHERE c.project_id = ?1",
                );
                if filter.group_id.is_some() {
                    sql.push_str(" AND group_id = ?2");
                }
                let status_idx = if filter.group_id.is_some() { 3 } else { 2 };
                if filter.status.is_some() {
                    use std::fmt::Write as _;
                    let _ = write!(sql, " AND status = ?{status_idx}");
                }
                sql.push_str(" ORDER BY updated_at DESC");
                if filter.limit > 0 {
                    use std::fmt::Write as _;
                    let _ = write!(sql, " LIMIT {}", filter.limit);
                }

                let mut stmt = c.prepare(&sql).map_err(|e| DomainError::Internal(e.to_string()))?;
                let rows = if let Some(g) = filter.group_id {
                    if let Some(s) = filter.status {
                        stmt.query_map(
                            params![project_id.to_string(), g.to_string(), s.as_str()],
                            row_to_summary,
                        )
                    } else {
                        stmt.query_map(
                            params![project_id.to_string(), g.to_string()],
                            row_to_summary,
                        )
                    }
                } else if let Some(s) = filter.status {
                    stmt.query_map(params![project_id.to_string(), s.as_str()], row_to_summary)
                } else {
                    stmt.query_map(params![project_id.to_string()], row_to_summary)
                }
                .map_err(|e| DomainError::Internal(e.to_string()))?;

                let mut out = Vec::new();
                for r in rows {
                    out.push(r.map_err(|e| DomainError::Internal(e.to_string()))?);
                }
                Ok::<_, DomainError>(out)
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn search_exact(
        &self,
        project_id: Id,
        group_id: Option<Id>,
        query: &str,
        limit: i64,
    ) -> Result<Vec<SearchResult>, DomainError> {
        let db = self.db.clone();
        let query = query.to_owned();
        let q = format!("%{query}%");
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let sql = format!(
                    "SELECT id, method, path, summary, status, tags FROM contracts
                     WHERE project_id = ?1
                       AND (?2 IS NULL OR group_id = ?2)
                       AND (path LIKE ?3 OR summary LIKE ?3)
                     ORDER BY CASE WHEN path = ?4 THEN 0 ELSE 1 END ASC, updated_at DESC
                     LIMIT {limit}",
                );
                let mut stmt = c.prepare(&sql).map_err(|e| DomainError::Internal(e.to_string()))?;
                let rows = stmt
                    .query_map(
                        params![project_id.to_string(), group_id.map(|g| g.to_string()), q, query],
                        |r| -> rusqlite::Result<SearchResult> {
                            let id: String = r.get(0)?;
                            let method: String = r.get(1)?;
                            let path: String = r.get(2)?;
                            let summary: String = r.get(3)?;
                            let status: String = r.get(4)?;
                            let tags: String = r.get(5)?;
                            Ok(SearchResult {
                                id: parse_id(&id).map_err(|_| rusqlite::Error::InvalidQuery)?,
                                method: parse_method(&method)
                                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                                path,
                                summary,
                                status: parse_status(&status)
                                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                                tags: parse_tags(tags)
                                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                                group_name: None,
                                similarity: None,
                            })
                        },
                    )
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r.map_err(|e| DomainError::Internal(e.to_string()))?);
                }
                Ok::<_, DomainError>(out)
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn search_semantic(
        &self,
        project_id: Id,
        group_id: Option<Id>,
        query_embedding: &[f32],
        k: i64,
    ) -> Result<Vec<(Id, f32)>, DomainError> {
        let bytes: Vec<u8> = query_embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                // JOIN with contracts to filter by project_id and group_id.
                // Note: sqlite-vec KNN requires the `k = ?` constraint in
                // the WHERE clause (or a literal LIMIT). Using a bound
                // `LIMIT ?` is rejected with "A LIMIT or 'k = ?' constraint
                // is required on vec0 knn queries."
                let mut stmt = c
                    .prepare(
                        "SELECT ce.contract_id, ce.distance
                       FROM contract_embeddings ce
                       JOIN contracts c ON c.id = ce.contract_id
                      WHERE ce.embedding MATCH ?1
                        AND k = ?4
                        AND c.project_id = ?2
                        AND (?3 IS NULL OR c.group_id = ?3)
                      ORDER BY ce.distance",
                    )
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                let rows = stmt
                    .query_map(
                        params![bytes, project_id.to_string(), group_id.map(|g| g.to_string()), k],
                        |r| -> rusqlite::Result<(Id, f32)> {
                            let id: String = r.get(0)?;
                            let distance: f32 = r.get(1)?;
                            Ok((
                                parse_id(&id).map_err(|_| rusqlite::Error::InvalidQuery)?,
                                1.0 - distance,
                            ))
                        },
                    )
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r.map_err(|e| DomainError::Internal(e.to_string()))?);
                }
                Ok::<_, DomainError>(out)
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn top_k_similar(
        &self,
        project_id: Id,
        query_embedding: &[f32],
        k: i64,
    ) -> Result<Vec<(Id, f32)>, DomainError> {
        // No group filter on dup-check — we want to find any near-match in
        // the project.
        self.search_semantic(project_id, None, query_embedding, k).await
    }
}

fn row_to_summary(r: &rusqlite::Row<'_>) -> rusqlite::Result<ContractSummary> {
    let id: String = r.get(0)?;
    let method: String = r.get(1)?;
    let path: String = r.get(2)?;
    let summary: String = r.get(3)?;
    let status: String = r.get(4)?;
    let tags: String = r.get(5)?;
    let group_name: Option<String> = r.get(6)?;
    Ok(ContractSummary {
        id: Id::parse(&id).ok_or(rusqlite::Error::InvalidQuery)?,
        method: Method::parse(&method).ok_or(rusqlite::Error::InvalidQuery)?,
        path,
        summary,
        status: Status::parse(&status).ok_or(rusqlite::Error::InvalidQuery)?,
        tags: serde_json::from_str(&tags).unwrap_or_default(),
        group_name,
        similarity: None,
    })
}

fn load_contract(
    c: &rusqlite::Connection,
    project_id: Id,
    contract_id: Id,
) -> Result<Option<Contract>, DomainError> {
    let row = c
        .query_row(
            "SELECT id, project_id, group_id, method, path, summary, description,
                request_headers, request_params, request_body_schema, request_example,
                response_schema, response_example, auth_type, status, tags,
                created_at, updated_at
         FROM contracts WHERE id = ?1 AND project_id = ?2",
            params![contract_id.to_string(), project_id.to_string()],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, Option<String>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, Option<String>>(9)?,
                    r.get::<_, Option<String>>(10)?,
                    r.get::<_, String>(11)?,
                    r.get::<_, Option<String>>(12)?,
                    r.get::<_, Option<String>>(13)?,
                    r.get::<_, String>(14)?,
                    r.get::<_, String>(15)?,
                    r.get::<_, i64>(16)?,
                    r.get::<_, i64>(17)?,
                ))
            },
        )
        .optional()
        .map_err(|e| DomainError::Internal(e.to_string()))?;

    let Some(row) = row else {
        return Ok(None);
    };

    Ok(Some(Contract {
        id: parse_id(&row.0)?,
        project_id: parse_id(&row.1)?,
        group_id: match row.2.as_deref() {
            Some(s) => Some(parse_id(s)?),
            None => None,
        },
        method: parse_method(&row.3)?,
        path: row.4,
        summary: row.5,
        description: row.6,
        request_headers: parse_json_opt(row.7)?,
        request_params: parse_json_opt(row.8)?,
        request_body_schema: parse_json_opt(row.9)?,
        request_example: parse_json_opt(row.10)?,
        response_schema: parse_json_required(row.11)?,
        response_example: parse_json_opt(row.12)?,
        auth_type: parse_auth_type_opt(row.13)?,
        status: parse_status(&row.14)?,
        tags: parse_tags(row.15)?,
        created_at: OffsetDateTime::from_unix_timestamp(row.16)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
        updated_at: OffsetDateTime::from_unix_timestamp(row.17)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contract::Method;
    use crate::domain::ports::{ContractRepo, ProjectRepo};
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

    fn make_create(path: &str, summary: &str) -> ContractCreate {
        ContractCreate {
            method: Method::Get,
            path: path.to_owned(),
            summary: summary.to_owned(),
            description: None,
            request_headers: None,
            request_params: None,
            request_body_schema: None,
            request_example: None,
            response_schema: serde_json::json!({"type": "object"}),
            response_example: None,
            auth_type: None,
            status: None,
            tags: None,
            group_name: None,
            force: false,
        }
    }

    #[tokio::test]
    async fn create_then_find() {
        let (db, pid) = setup().await;
        let repo = SqliteContractRepo { db };
        let c = repo.create(pid, None, &make_create("/api/users", "List users")).await.unwrap();
        let found = repo.find_by_id(pid, c.id).await.unwrap();
        assert_eq!(found.path, "/api/users");
    }

    #[tokio::test]
    async fn duplicate_method_path_errors() {
        let (db, pid) = setup().await;
        let repo = SqliteContractRepo { db };
        repo.create(pid, None, &make_create("/api/users", "A")).await.unwrap();
        let err = repo.create(pid, None, &make_create("/api/users", "B")).await.unwrap_err();
        assert!(matches!(err, DomainError::DuplicateKey { .. }));
    }

    #[tokio::test]
    async fn normalize_path_strips_trailing() {
        let (db, pid) = setup().await;
        let repo = SqliteContractRepo { db };
        let c = repo.create(pid, None, &make_create("/api/users/", "A")).await.unwrap();
        assert_eq!(c.path, "/api/users");
    }

    #[tokio::test]
    async fn update_changes_summary_and_touches_updated_at() {
        let (db, pid) = setup().await;
        let repo = SqliteContractRepo { db };
        let c = repo.create(pid, None, &make_create("/api/users", "Old")).await.unwrap();
        let before = c.updated_at;
        std::thread::sleep(std::time::Duration::from_secs(1));
        let patch = ContractUpdate { summary: Some("New".to_owned()), ..Default::default() };
        let updated = repo.update(pid, c.id, &patch, None).await.unwrap();
        assert_eq!(updated.summary, "New");
        assert!(updated.updated_at > before);
    }

    #[tokio::test]
    async fn delete_removes() {
        let (db, pid) = setup().await;
        let repo = SqliteContractRepo { db };
        let c = repo.create(pid, None, &make_create("/api/users", "A")).await.unwrap();
        repo.delete(pid, c.id).await.unwrap();
        let err = repo.find_by_id(pid, c.id).await.unwrap_err();
        assert!(matches!(err, DomainError::NotFound { .. }));
    }

    #[tokio::test]
    async fn list_filters_by_status() {
        let (db, pid) = setup().await;
        let repo = SqliteContractRepo { db };
        repo.create(pid, None, &make_create("/a", "A")).await.unwrap();
        let mut c2 = make_create("/b", "B");
        c2.status = Some("stable".to_owned());
        repo.create(pid, None, &c2).await.unwrap();
        let list = repo
            .list(
                pid,
                &ListContractsFilter {
                    status: Some(Status::Stable),
                    limit: 100,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, "/b");
    }
}
