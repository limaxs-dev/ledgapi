//! Standard API envelope. Mirrors the rust-starter convention so future
//! migration to a multi-crate layout stays trivial.

use serde::{Deserialize, Serialize};

/// Stable error code per spec §5.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    ValidationFailed,
    NotFound,
    DuplicateKey,
    Unauthorized,
    Forbidden,
    InternalError,
    ServiceUnavailable,
}

impl ApiErrorCode {
    /// String form for the `code` field in [`ApiError`].
    #[must_use]
    pub fn as_symbol(self) -> &'static str {
        match self {
            Self::ValidationFailed => "validation_failed",
            Self::NotFound => "not_found",
            Self::DuplicateKey => "duplicate_key",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::InternalError => "internal_error",
            Self::ServiceUnavailable => "service_unavailable",
        }
    }

    /// HTTP status code matching the error.
    #[must_use]
    pub fn http_status(self) -> u16 {
        match self {
            Self::ValidationFailed => 400,
            Self::NotFound => 404,
            Self::DuplicateKey => 409,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::InternalError => 500,
            Self::ServiceUnavailable => 503,
        }
    }
}

/// Single error entry inside the envelope's `errors` array.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiError {
    /// Stable error code, e.g. `"validation_failed"`.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Optional field path (e.g. `"email"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl ApiError {
    /// Build an `ApiError` from a code symbol and message.
    #[must_use]
    pub fn new(code: ApiErrorCode, message: impl Into<String>) -> Self {
        Self { code: code.as_symbol().to_owned(), message: message.into(), field: None }
    }

    /// Attach a field path (mutates and returns self).
    #[must_use]
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

/// Envelope returned by every JSON endpoint.
///
/// Success: `{ success: true, code: 200, message, data: <T>, errors: null }`.
/// Failure: `{ success: false, code: <status>, message, data: null, errors: [ApiError] }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub code: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ApiError>>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Build a successful response.
    #[must_use]
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            code: 200,
            message: "OK".to_owned(),
            data: Some(data),
            errors: None,
        }
    }

    /// Build a successful response with a custom message.
    #[must_use]
    pub fn ok_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            code: 200,
            message: message.into(),
            data: Some(data),
            errors: None,
        }
    }
}

impl ApiResponse<()> {
    /// Build a failure response from a code and a single error.
    #[must_use]
    pub fn err(code: ApiErrorCode, message: impl Into<String>) -> Self {
        Self {
            success: false,
            code: code.http_status(),
            message: "Operation failed".to_owned(),
            data: None,
            errors: Some(vec![ApiError::new(code, message)]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ok_serializes_data() {
        let env = ApiResponse::ok(json!({"id": "abc"}));
        let v = serde_json::to_value(&env).unwrap();
        assert_eq!(v["success"], json!(true));
        assert_eq!(v["code"], json!(200));
        assert_eq!(v["data"]["id"], json!("abc"));
        assert!(v.get("errors").is_none());
    }

    #[test]
    fn err_serializes_errors_array() {
        let env: ApiResponse<()> = ApiResponse::err(ApiErrorCode::NotFound, "project not found");
        let v = serde_json::to_value(&env).unwrap();
        assert_eq!(v["success"], json!(false));
        assert_eq!(v["code"], json!(404));
        assert_eq!(v["errors"][0]["code"], json!("not_found"));
        assert_eq!(v["errors"][0]["message"], json!("project not found"));
    }

    #[test]
    fn with_field_attaches_field() {
        let err = ApiError::new(ApiErrorCode::ValidationFailed, "must be valid")
            .with_field("email");
        assert_eq!(err.field.as_deref(), Some("email"));
    }

    #[test]
    fn code_http_status_roundtrip() {
        assert_eq!(ApiErrorCode::NotFound.http_status(), 404);
        assert_eq!(ApiErrorCode::DuplicateKey.http_status(), 409);
        assert_eq!(ApiErrorCode::InternalError.http_status(), 500);
    }
}