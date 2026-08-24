use crate::core::id::Id;
use crate::domain::auth::Session;
use crate::domain::errors::DomainError;
use crate::domain::ports::SessionRepo;
use crate::infra::db::Db;
use crate::infra::repos::project_repo_sqlite::parse_id;
use async_trait::async_trait;
use rusqlite::{OptionalExtension, params};
use time::OffsetDateTime;

pub struct SqliteSessionRepo {
    pub(crate) db: Db,
}

fn map_session(row: (String, String, String, i64, Option<i64>)) -> Result<Session, DomainError> {
    Ok(Session {
        token_hash: row.0,
        user_id: parse_id(&row.1)?,
        csrf_token_hash: row.2,
        expires_at: OffsetDateTime::from_unix_timestamp(row.3)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
        revoked_at: row.4.map(|value| {
            OffsetDateTime::from_unix_timestamp(value).unwrap_or(OffsetDateTime::UNIX_EPOCH)
        }),
    })
}

#[async_trait]
impl SessionRepo for SqliteSessionRepo {
    async fn create(&self, session: &Session) -> Result<(), DomainError> {
        let db = self.db.clone();
        let session = session.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.execute(
                    "INSERT INTO web_sessions
                     (token_hash, user_id, csrf_token_hash, expires_at, created_at, last_seen_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    params![
                        session.token_hash,
                        session.user_id.to_string(),
                        session.csrf_token_hash,
                        session.expires_at.unix_timestamp(),
                        OffsetDateTime::now_utc().unix_timestamp()
                    ],
                )
                .map_err(|error| DomainError::Internal(error.to_string()))?;
                Ok(())
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn find(
        &self,
        token_hash: &str,
        now: OffsetDateTime,
    ) -> Result<Option<Session>, DomainError> {
        let db = self.db.clone();
        let token_hash = token_hash.to_owned();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.query_row(
                    "SELECT token_hash, user_id, csrf_token_hash, expires_at, revoked_at
                     FROM web_sessions
                     WHERE token_hash = ?1 AND revoked_at IS NULL AND expires_at > ?2",
                    params![token_hash, now.unix_timestamp()],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )
                .optional()
                .map_err(|error| DomainError::Internal(error.to_string()))?
                .map(map_session)
                .transpose()
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn revoke(&self, token_hash: &str, now: OffsetDateTime) -> Result<(), DomainError> {
        let db = self.db.clone();
        let token_hash = token_hash.to_owned();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.execute(
                    "UPDATE web_sessions SET revoked_at = ?1 WHERE token_hash = ?2 AND revoked_at IS NULL",
                    params![now.unix_timestamp(), token_hash],
                )
                .map_err(|error| DomainError::Internal(error.to_string()))?;
                Ok(())
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }
}

#[allow(dead_code)]
fn _id_marker(_: Id) {}
