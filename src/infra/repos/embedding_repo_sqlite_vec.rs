//! SQLite-vec adapter for [`EmbeddingRepo`].
//!
//! Stores 384-dim f32 embeddings (MiniLM). Embeddings are serialized
//! as little-endian bytes for sqlite-vec.

use crate::core::id::Id;
use crate::domain::errors::DomainError;
use crate::domain::ports::EmbeddingRepo;
use crate::infra::db::Db;
use async_trait::async_trait;
use rusqlite::params;

/// `contract_embeddings` (vec0) adapter. Implements [`EmbeddingRepo`].
pub struct SqliteEmbeddingRepo {
    pub(crate) db: Db,
}

/// Embedding dimensionality (all-MiniLM-L6-v2).
const DIM: usize = 384;

#[async_trait]
impl EmbeddingRepo for SqliteEmbeddingRepo {
    async fn upsert(
        &self,
        contract_id: Id,
        project_id: Id,
        embedding: &[f32],
    ) -> Result<(), DomainError> {
        if embedding.len() != DIM {
            return Err(DomainError::Validation {
                field: "embedding".to_owned(),
                message: format!("expected {DIM} dims, got {}", embedding.len()),
            });
        }
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                c.execute(
                    "INSERT OR REPLACE INTO contract_embeddings (contract_id, project_id, embedding)
                     VALUES (?1, ?2, ?3)",
                    params![contract_id.to_string(), project_id.to_string(), bytes],
                )
                .map_err(|e| DomainError::Internal(e.to_string()))?;
                Ok::<_, DomainError>(())
            })
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }

    async fn delete(&self, contract_id: Id) -> Result<(), DomainError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                c.execute(
                    "DELETE FROM contract_embeddings WHERE contract_id = ?1",
                    [contract_id.to_string()],
                )
                .map_err(|e| DomainError::Internal(e.to_string()))?;
                Ok::<_, DomainError>(())
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

    fn vec384(seed: f32) -> Vec<f32> {
        (0..DIM).map(|i| seed + i as f32 * 0.001).collect()
    }

    #[tokio::test]
    async fn upsert_and_delete() {
        let db = open_memory().unwrap();
        let repo = SqliteEmbeddingRepo { db: db.clone() };
        let id = Id::new();
        let pid = Id::new();
        repo.upsert(id, pid, &vec384(0.0)).await.unwrap();
        // Delete
        repo.delete(id).await.unwrap();
        db.with_conn(|c| {
            let n: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM contract_embeddings WHERE contract_id = ?1",
                    [id.to_string()],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 0);
        });
    }

    #[tokio::test]
    async fn rejects_wrong_dim() {
        let db = open_memory().unwrap();
        let repo = SqliteEmbeddingRepo { db };
        let id = Id::new();
        let pid = Id::new();
        let err = repo.upsert(id, pid, &vec![0.0; 100]).await.unwrap_err();
        assert!(matches!(err, DomainError::Validation { .. }));
    }
}
