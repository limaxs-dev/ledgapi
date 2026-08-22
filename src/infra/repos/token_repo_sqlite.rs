//! SQLite adapter for [`TokenRepo`].

use crate::domain::errors::DomainError;
use crate::domain::ports::TokenRepo;
use crate::infra::db::Db;
use async_trait::async_trait;
use time::OffsetDateTime;

/// `auth_tokens` adapter. Implements [`TokenRepo`] against SQLite.
pub struct SqliteTokenRepo {
    pub(crate) db: Db,
}

#[async_trait]
impl TokenRepo for SqliteTokenRepo {
    async fn exists(&self, hash_hex: &str) -> Result<bool, DomainError> {
        let db = self.db.clone();
        let h = hash_hex.to_owned();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let n: i64 = c
                    .query_row(
                        "SELECT COUNT(*) FROM auth_tokens WHERE token_hash = ?1",
                        [&h],
                        |r| r.get(0),
                    )
                    .map_err(|e| DomainError::Internal(e.to_string()))?;
                Ok::<_, DomainError>(n > 0)
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn insert(&self, hash_hex: &str, label: Option<&str>) -> Result<(), DomainError> {
        let db = self.db.clone();
        let h = hash_hex.to_owned();
        let l = label.map(str::to_owned);
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                c.execute(
                    "INSERT INTO auth_tokens (token_hash, label, created_at) VALUES (?1, ?2, ?3)",
                    rusqlite::params![h, l, OffsetDateTime::now_utc().unix_timestamp()],
                )
                .map_err(|e| DomainError::Internal(e.to_string()))?;
                Ok::<_, DomainError>(())
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn count(&self) -> Result<i64, DomainError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                c.query_row("SELECT COUNT(*) FROM auth_tokens", [], |r| r.get(0))
                    .map_err(|e| DomainError::Internal(e.to_string()))
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::db::pool::open_memory;

    #[tokio::test]
    async fn insert_then_exists() {
        let db = open_memory().unwrap();
        let repo = SqliteTokenRepo { db };
        assert!(!repo.exists("aa").await.unwrap());
        repo.insert("aa", Some("first-run")).await.unwrap();
        assert!(repo.exists("aa").await.unwrap());
        assert!(!repo.exists("bb").await.unwrap());
    }

    #[tokio::test]
    async fn count_starts_at_zero() {
        let db = open_memory().unwrap();
        let repo = SqliteTokenRepo { db };
        assert_eq!(repo.count().await.unwrap(), 0);
        repo.insert("aa", None).await.unwrap();
        assert_eq!(repo.count().await.unwrap(), 1);
    }
}
