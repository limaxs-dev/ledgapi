use crate::core::id::Id;
use crate::domain::audit::{AuditAction, AuditEntry, AuditEvent, AuditFilter, AuditResource};
use crate::domain::errors::DomainError;
use crate::domain::ports::AuditRepo;
use crate::infra::db::Db;
use crate::infra::repos::project_repo_sqlite::parse_id;
use async_trait::async_trait;
use rusqlite::params;
use time::OffsetDateTime;

pub struct SqliteAuditRepo {
    pub(crate) db: Db,
}

fn parse_action(value: &str) -> Result<AuditAction, DomainError> {
    match value {
        "create" => Ok(AuditAction::Create),
        "update" => Ok(AuditAction::Update),
        "delete" => Ok(AuditAction::Delete),
        _ => Err(DomainError::Internal(format!("bad audit action: {value}"))),
    }
}

fn parse_resource(value: &str) -> Result<AuditResource, DomainError> {
    match value {
        "user" => Ok(AuditResource::User),
        "project" => Ok(AuditResource::Project),
        "group" => Ok(AuditResource::Group),
        "contract" => Ok(AuditResource::Contract),
        _ => Err(DomainError::Internal(format!("bad audit resource: {value}"))),
    }
}

fn map_entry(
    row: (String, Option<String>, Option<String>, String, String, Option<String>, String, i64),
) -> Result<AuditEntry, DomainError> {
    Ok(AuditEntry {
        id: parse_id(&row.0)?,
        actor_user_id: row.1.as_deref().map(parse_id).transpose()?,
        actor_username: row.2,
        action: parse_action(&row.3)?,
        resource: parse_resource(&row.4)?,
        resource_id: row.5.as_deref().map(parse_id).transpose()?,
        metadata: serde_json::from_str(&row.6)
            .map_err(|error| DomainError::Internal(error.to_string()))?,
        created_at: OffsetDateTime::from_unix_timestamp(row.7)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
    })
}

#[async_trait]
impl AuditRepo for SqliteAuditRepo {
    async fn append(&self, event: &AuditEvent) -> Result<AuditEntry, DomainError> {
        let db = self.db.clone();
        let event = event.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                let id = Id::new();
                let metadata = serde_json::to_string(&event.metadata)
                    .map_err(|error| DomainError::Internal(error.to_string()))?;
                conn.execute(
                    "INSERT INTO audit_log
                     (id, actor_user_id, action, resource_type, resource_id, metadata, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        id.to_string(),
                        event.actor_user_id.map(|value| value.to_string()),
                        event.action.as_str(),
                        event.resource.as_str(),
                        event.resource_id.map(|value| value.to_string()),
                        metadata,
                        event.created_at.unix_timestamp()
                    ],
                )
                .map_err(|error| DomainError::Internal(error.to_string()))?;
                Ok(AuditEntry {
                    id,
                    actor_user_id: event.actor_user_id,
                    actor_username: None,
                    action: event.action,
                    resource: event.resource,
                    resource_id: event.resource_id,
                    metadata: event.metadata,
                    created_at: event.created_at,
                })
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn list(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>, DomainError> {
        let db = self.db.clone();
        let filter = filter.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                let mut statement = conn
                    .prepare(
                        "SELECT a.id, a.actor_user_id, u.username, a.action, a.resource_type,
                                a.resource_id, a.metadata, a.created_at
                         FROM audit_log a LEFT JOIN users u ON u.id = a.actor_user_id
                         ORDER BY a.created_at DESC LIMIT ?1 OFFSET ?2",
                    )
                    .map_err(|error| DomainError::Internal(error.to_string()))?;
                let rows = statement
                    .query_map(params![filter.limit.max(1), filter.offset.max(0)], |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                            row.get(7)?,
                        ))
                    })
                    .map_err(|error| DomainError::Internal(error.to_string()))?;
                rows.map(|row| {
                    let entry = row.map_err(|error| DomainError::Internal(error.to_string()))?;
                    map_entry(entry)
                })
                .filter(|result| {
                    result.as_ref().is_ok_and(|entry| {
                        filter.actor_user_id.is_none_or(|id| entry.actor_user_id == Some(id))
                            && filter.action.is_none_or(|action| entry.action == action)
                            && filter.resource.is_none_or(|resource| entry.resource == resource)
                    })
                })
                .collect()
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn list_for_resource(
        &self,
        resource: AuditResource,
        resource_id: Id,
    ) -> Result<Vec<AuditEntry>, DomainError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                let mut statement = conn
                    .prepare(
                        "SELECT a.id, a.actor_user_id, u.username, a.action, a.resource_type,
                                a.resource_id, a.metadata, a.created_at
                         FROM audit_log a LEFT JOIN users u ON u.id = a.actor_user_id
                         WHERE a.resource_type = ?1 AND a.resource_id = ?2
                         ORDER BY a.created_at DESC",
                    )
                    .map_err(|error| DomainError::Internal(error.to_string()))?;
                let rows = statement
                    .query_map(params![resource.as_str(), resource_id.to_string()], |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                            row.get(7)?,
                        ))
                    })
                    .map_err(|error| DomainError::Internal(error.to_string()))?;
                rows.map(|row| {
                    let entry = row.map_err(|error| DomainError::Internal(error.to_string()))?;
                    map_entry(entry)
                })
                .collect()
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }
}
