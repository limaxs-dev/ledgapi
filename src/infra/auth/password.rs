use crate::domain::errors::DomainError;
use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordVerifier};

pub fn hash_password(password: &str) -> Result<String, DomainError> {
    if password.len() < 12 {
        return Err(DomainError::Validation {
            field: "password".to_owned(),
            message: "must be at least 12 characters".to_owned(),
        });
    }
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| DomainError::Internal(format!("password hash failed: {error}")))
}

pub fn verify_password(password: &str, encoded_hash: &str) -> Result<bool, DomainError> {
    let hash = PasswordHash::new(encoded_hash)
        .map_err(|error| DomainError::Internal(format!("invalid password hash: {error}")))?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &hash).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_and_verifies_passwords() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(verify_password("correct horse battery staple", &hash).unwrap());
        assert!(!verify_password("wrong password", &hash).unwrap());
    }

    #[test]
    fn rejects_short_passwords() {
        assert!(matches!(hash_password("too short"), Err(DomainError::Validation { .. })));
    }
}
