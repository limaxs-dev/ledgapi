use crate::core::id::Id;
use crate::domain::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    SuperAdmin,
    Editor,
    Viewer,
}

impl Role {
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "super_admin" => Ok(Self::SuperAdmin),
            "editor" => Ok(Self::Editor),
            "viewer" => Ok(Self::Viewer),
            _ => Err(DomainError::Validation {
                field: "role".to_owned(),
                message: "must be super_admin, editor, or viewer".to_owned(),
            }),
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SuperAdmin => "super_admin",
            Self::Editor => "editor",
            Self::Viewer => "viewer",
        }
    }

    #[must_use]
    pub const fn can_write(self) -> bool {
        matches!(self, Self::SuperAdmin | Self::Editor)
    }

    #[must_use]
    pub const fn can_manage_users(self) -> bool {
        matches!(self, Self::SuperAdmin)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: Id,
    pub username: String,
    pub password_hash: String,
    pub role: Role,
    pub active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserCreate {
    pub username: String,
    pub password_hash: String,
    pub role: Role,
}

impl UserCreate {
    pub fn validate(&self) -> Result<(), DomainError> {
        let username = self.username.trim();
        if username.is_empty() || username.len() > 100 {
            return Err(DomainError::Validation {
                field: "username".to_owned(),
                message: "must be 1-100 characters".to_owned(),
            });
        }
        if !username.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
            return Err(DomainError::Validation {
                field: "username".to_owned(),
                message: "must contain only letters, numbers, '.', '_' or '-'".to_owned(),
            });
        }
        if self.password_hash.trim().is_empty() {
            return Err(DomainError::Validation {
                field: "password_hash".to_owned(),
                message: "must not be empty".to_owned(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub token_hash: String,
    pub user_id: Id,
    pub csrf_token_hash: String,
    pub expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    pub user_id: Id,
    pub username: String,
    pub role: Role,
    pub client_id: Option<String>,
    pub scopes: Vec<String>,
}

impl Principal {
    #[must_use]
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|candidate| candidate == scope)
    }

    pub fn require_scope(&self, scope: &str) -> Result<(), DomainError> {
        if self.has_scope(scope) {
            Ok(())
        } else {
            Err(DomainError::Forbidden { message: format!("scope {scope} is required") })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationCode {
    pub code_hash: String,
    pub client_id: String,
    pub user_id: Id,
    pub redirect_uri: String,
    pub scope: Vec<String>,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthToken {
    pub token_hash: String,
    pub client_id: String,
    pub user_id: Id,
    pub scope: Vec<String>,
    pub expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshToken {
    pub token_hash: String,
    pub client_id: String,
    pub user_id: Id,
    pub scope: Vec<String>,
    pub expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_have_stable_names_and_permissions() {
        assert_eq!(Role::SuperAdmin.as_str(), "super_admin");
        assert!(Role::SuperAdmin.can_manage_users());
        assert!(Role::Editor.can_write());
        assert!(!Role::Viewer.can_write());
    }

    #[test]
    fn principal_requires_requested_scope() {
        let principal = Principal {
            user_id: Id::new(),
            username: "admin".to_owned(),
            role: Role::SuperAdmin,
            client_id: None,
            scopes: vec!["ledgapi:read".to_owned()],
        };
        assert!(principal.require_scope("ledgapi:read").is_ok());
        assert!(matches!(
            principal.require_scope("ledgapi:write"),
            Err(DomainError::Forbidden { .. })
        ));
    }
}
