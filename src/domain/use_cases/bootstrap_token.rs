//! First-run token generation. Returns the plaintext token ONCE — the
//! caller (bootstrap) is responsible for logging it.

use crate::domain::errors::DomainError;
use crate::domain::ports::TokenRepo;

/// Ensure the system has at least one token. If the table is empty,
/// generate one and persist its hash. Returns `(plaintext, true)` for
/// first-run, `(existing_marker, false)` otherwise.
pub async fn ensure(tokens: &dyn TokenRepo) -> Result<(String, bool), DomainError> {
    if tokens.count().await? > 0 {
        return Ok((String::new(), false));
    }

    let plaintext = generate_token();
    let hash = sha256_hex(&plaintext);
    tokens.insert(&hash, Some("first-run")).await?;
    Ok((plaintext, true))
}

/// Generate a new random token (32 random bytes, hex-encoded).
pub fn generate_token() -> String {
    use rand::RngCore;
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// SHA-256 hash of the token, returned as lowercase hex.
pub fn sha256_hex(t: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(t.as_bytes());
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::db::pool::open_memory;
    use crate::infra::repos::token_repo_sqlite::SqliteTokenRepo;

    #[tokio::test]
    async fn first_run_returns_plaintext_and_persists_hash() {
        let db = open_memory().unwrap();
        let repo = SqliteTokenRepo { db };
        let (plaintext, was_first) = ensure(&repo).await.unwrap();
        assert!(was_first);
        assert_eq!(plaintext.len(), 64);

        // Hash is stored, not the plaintext
        let hash = sha256_hex(&plaintext);
        assert!(repo.exists(&hash).await.unwrap());
        assert!(!repo.exists(&plaintext).await.unwrap());
    }

    #[tokio::test]
    async fn second_run_does_not_generate() {
        let db = open_memory().unwrap();
        let repo = SqliteTokenRepo { db };
        let (plaintext1, was_first1) = ensure(&repo).await.unwrap();
        assert!(was_first1);

        let (plaintext2, was_first2) = ensure(&repo).await.unwrap();
        assert!(!was_first2);
        assert_eq!(plaintext2, "");
        assert_ne!(plaintext1, "");
    }
}
