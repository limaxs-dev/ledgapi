//! Token generation, hashing, and constant-time comparison.
//!
//! Tokens are 32 random bytes encoded as 64 lowercase hex chars.
//! Stored as `sha256(token)` hex. Per spec §7.1, the DB-side compare
//! is constant-time; header parsing is just a `strip_prefix`.

use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Generate a new random token. 32 bytes → 64 lowercase hex chars.
#[must_use]
pub fn generate() -> String {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// SHA-256 hash of the token, returned as lowercase hex (64 chars).
#[must_use]
pub fn sha256_hex(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Constant-time equality on two hex strings of equal length. Returns
/// `false` immediately if lengths differ (lengths are not secret).
#[must_use]
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let a = a.as_bytes();
    let b = b.as_bytes();
    a.ct_eq(b).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_is_64_lowercase_hex() {
        let t = generate();
        assert_eq!(t.len(), 64);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn generate_produces_different_tokens() {
        let a = generate();
        let b = generate();
        assert_ne!(a, b);
    }

    #[test]
    fn sha256_is_stable_and_correct_length() {
        let h = sha256_hex("test");
        assert_eq!(h.len(), 64);
        assert_eq!(h, sha256_hex("test"));
    }

    #[test]
    fn constant_time_eq_matches_equality() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
    }

    #[test]
    fn constant_time_eq_rejects_length_mismatch() {
        assert!(!constant_time_eq("abc", "abcd"));
    }
}
