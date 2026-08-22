//! Domain error taxonomy. Every use case returns [`Result<T, DomainError>`].

use crate::core::envelope::ApiErrorCode;
use crate::core::id::Id;
use serde::Serialize;
use thiserror::Error;

/// A single semantic match returned by the dup-check on `create_contract`.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SimilarContract {
    /// Existing contract id.
    pub id: Id,
    /// HTTP method (e.g., `"GET"`).
    pub method: String,
    /// Path template (e.g., `"/api/v1/users/{id}"`).
    pub path: String,
    /// Cosine similarity in `[0.0, 1.0]`. Higher is closer.
    pub similarity: f32,
}

/// Top-level domain error.
#[derive(Debug, Error)]
pub enum DomainError {
    /// 400 — input failed validation. Always carries the offending field.
    #[error("validation failed: {field}: {message}")]
    Validation { field: String, message: String },

    /// 404 — the requested resource does not exist.
    #[error("{resource} not found")]
    NotFound { resource: &'static str },

    /// 409 — a uniqueness constraint was violated (e.g., same
    /// `(project, method, path)` as an existing contract).
    #[error("duplicate {resource}: {key}")]
    DuplicateKey { resource: &'static str, key: String },

    /// **Not** a JSON-RPC error — returned as a successful tool result with
    /// `status = "warning_similar_found"`. The caller (agent) decides to
    /// either `update_contract` one of the matches or re-call
    /// `create_contract` with `force=true`.
    #[error("similar contracts found")]
    SimilarFound { candidates: Vec<SimilarContract> },

    /// 401 — bearer token missing.
    #[error("missing bearer token")]
    AuthMissing,

    /// 403 — bearer token invalid.
    #[error("invalid bearer token")]
    AuthInvalid,

    /// 503 — embedder not yet loaded (cold start) or unhealthy.
    #[error("embedding service unavailable")]
    EmbeddingUnavailable,

    /// 500 — wrap-and-log. The inner message is for logs only; never
    /// echoed to the client verbatim.
    #[error("internal error: {0}")]
    Internal(String),
}

impl DomainError {
    /// Stable error code for the JSON envelope and MCP `data.code`.
    #[must_use]
    pub fn code(&self) -> ApiErrorCode {
        match self {
            Self::Validation { .. } => ApiErrorCode::ValidationFailed,
            Self::NotFound { .. } => ApiErrorCode::NotFound,
            Self::DuplicateKey { .. } => ApiErrorCode::DuplicateKey,
            Self::AuthMissing => ApiErrorCode::Unauthorized,
            Self::AuthInvalid => ApiErrorCode::Forbidden,
            Self::EmbeddingUnavailable => ApiErrorCode::ServiceUnavailable,
            Self::Internal(_) | Self::SimilarFound { .. } => ApiErrorCode::InternalError,
        }
    }

    /// HTTP status code matching this error.
    #[must_use]
    pub fn http_status(&self) -> u16 {
        self.code().http_status()
    }

    /// Field name, if this is a validation-style error.
    #[must_use]
    pub fn field(&self) -> Option<&str> {
        match self {
            Self::Validation { field, .. } => Some(field.as_str()),
            _ => None,
        }
    }

    /// Human-readable message (safe for clients).
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Validation { message, .. } => message.clone(),
            Self::NotFound { resource } => format!("{resource} not found"),
            Self::DuplicateKey { resource, key } => format!("{resource} already exists: {key}"),
            Self::SimilarFound { .. } => "similar contracts found".to_owned(),
            Self::AuthMissing => "missing bearer token".to_owned(),
            Self::AuthInvalid => "invalid bearer token".to_owned(),
            Self::EmbeddingUnavailable => "embedding service unavailable".to_owned(),
            Self::Internal(_) => "internal error".to_owned(),
        }
    }

    /// Returns true if this error should be returned as a JSON-RPC error
    /// (vs. a successful tool result). See spec §5.4.
    #[must_use]
    pub fn is_mcp_error(&self) -> bool {
        !matches!(self, Self::SimilarFound { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_carries_field() {
        let e = DomainError::Validation {
            field: "path".to_owned(),
            message: "must start with /".to_owned(),
        };
        assert_eq!(e.field(), Some("path"));
        assert_eq!(e.http_status(), 400);
        assert_eq!(e.code(), ApiErrorCode::ValidationFailed);
    }

    #[test]
    fn not_found_maps_to_404() {
        let e = DomainError::NotFound { resource: "project" };
        assert_eq!(e.http_status(), 404);
        assert_eq!(e.code(), ApiErrorCode::NotFound);
    }

    #[test]
    fn similar_found_is_not_mcp_error() {
        let e = DomainError::SimilarFound { candidates: vec![] };
        assert!(!e.is_mcp_error());
    }

    #[test]
    fn auth_missing_is_401() {
        assert_eq!(DomainError::AuthMissing.http_status(), 401);
        assert_eq!(DomainError::AuthInvalid.http_status(), 403);
    }

    #[test]
    fn internal_message_is_redacted_for_client() {
        let e = DomainError::Internal("database locked at /secret/path".to_owned());
        assert_eq!(e.message(), "internal error");
        // The original message is still preserved in Display for logs:
        assert!(e.to_string().contains("database locked"));
    }
}
