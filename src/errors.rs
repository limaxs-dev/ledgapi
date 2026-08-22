//! Top-level [`AppError`]. Wraps [`DomainError`] and shapes the HTTP
//! response. Web handlers convert to this; MCP handlers convert
//! `DomainError` directly to JSON-RPC frames.

use crate::core::envelope::{ApiErrorCode, ApiResponse};
use crate::domain::errors::DomainError;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Top-level application error. Wraps a [`DomainError`].
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// A domain-level error.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// The request body could not be parsed.
    #[error("invalid request: {0}")]
    BadRequest(String),
    /// Internal error with debug-only message (not exposed to client).
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// HTTP status code matching this error.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Domain(d) => {
                StatusCode::from_u16(d.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Stable error code for clients.
    #[must_use]
    pub fn code(&self) -> ApiErrorCode {
        match self {
            Self::Domain(d) => d.code(),
            Self::BadRequest(_) => ApiErrorCode::ValidationFailed,
            Self::Internal(_) => ApiErrorCode::InternalError,
        }
    }

    /// Client-safe message.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Domain(d) => d.message(),
            Self::BadRequest(m) => m.clone(),
            Self::Internal(_) => "internal error".to_owned(),
        }
    }

    /// Build the JSON envelope body.
    #[must_use]
    pub fn envelope(&self) -> ApiResponse<()> {
        let mut err = crate::core::envelope::ApiError::new(self.code(), self.message());
        if let Self::Domain(d) = self {
            if let Some(field) = d.field() {
                err = err.with_field(field.to_owned());
            }
        }
        ApiResponse::err(self.code(), self.message()).clone_with_errors(vec![err])
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Log 5xx loudly. 4xx is debug-level (agent errors are routine).
        if self.status().is_server_error() {
            tracing::error!(error = %self, "request failed");
        } else {
            tracing::debug!(error = %self, "request rejected");
        }
        (self.status(), Json(serde_json::to_value(self.envelope()).unwrap_or_default()))
            .into_response()
    }
}

// Helper extension so the envelope builder can keep its previous shape.
impl ApiResponse<()> {
    /// Replace the errors array of an envelope.
    #[must_use]
    pub fn clone_with_errors(self, errors: Vec<crate::core::envelope::ApiError>) -> Self {
        Self { errors: Some(errors), ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn validation_envelope_has_field() {
        let err: AppError = DomainError::Validation {
            field: "path".to_owned(),
            message: "must start with /".to_owned(),
        }
        .into();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        let env = err.envelope();
        let v = serde_json::to_value(env).unwrap();
        assert_eq!(v["errors"][0]["field"], Value::from("path"));
        assert_eq!(v["errors"][0]["code"], Value::from("validation_failed"));
    }

    #[test]
    fn not_found_status() {
        let err: AppError = DomainError::NotFound { resource: "project" }.into();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        assert_eq!(err.code(), ApiErrorCode::NotFound);
    }

    #[test]
    fn internal_error_redacts_message() {
        let err = AppError::Internal("DB at /secret/path is locked".to_owned());
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.message(), "internal error");
    }

    #[test]
    fn auth_missing_is_401() {
        let err: AppError = DomainError::AuthMissing.into();
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }
}
