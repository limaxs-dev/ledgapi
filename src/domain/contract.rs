//! Contract aggregate — a single API endpoint definition.

use crate::core::id::Id;
use crate::domain::errors::DomainError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// HTTP method. Whitelist per spec §5.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    /// Construct from a string (case-insensitive). Returns `None` if
    /// the method is not in the whitelist.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "PATCH" => Some(Self::Patch),
            "DELETE" => Some(Self::Delete),
            _ => None,
        }
    }

    /// Borrow the canonical upper-case string form.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Contract lifecycle status.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Draft,
    Stable,
    Deprecated,
}

impl Status {
    /// Construct from a string (case-sensitive lowercase). Returns
    /// `None` for unknown values.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "stable" => Some(Self::Stable),
            "deprecated" => Some(Self::Deprecated),
            _ => None,
        }
    }

    /// Borrow the canonical lowercase string form.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Stable => "stable",
            Self::Deprecated => "deprecated",
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Auth type keyword — maps to OpenAPI `securitySchemes` per spec §13 #11.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    #[default]
    None,
    Bearer,
    ApiKey,
    Basic,
}

impl AuthType {
    /// Parse from a string (case-insensitive). Unknown values are
    /// treated as `None` (per spec §13 #11).
    #[must_use]
    pub fn parse_or_default(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "bearer" => Self::Bearer,
            "api_key" | "apikey" => Self::ApiKey,
            "basic" => Self::Basic,
            _ => Self::None,
        }
    }

    /// Borrow the canonical snake-case string form.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Bearer => "bearer",
            Self::ApiKey => "api_key",
            Self::Basic => "basic",
        }
    }
}

impl std::fmt::Display for AuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Normalize a path per spec §13 #6: trim trailing slash except for root
/// `/`; case-sensitive; no other normalization.
pub fn normalize_path(s: &str) -> String {
    let s = s.trim();
    if s.len() > 1 && s.ends_with('/') {
        let stripped = s.trim_end_matches('/');
        if stripped.is_empty() { "/".to_owned() } else { stripped.to_owned() }
    } else {
        s.to_owned()
    }
}

/// A contract stored in the registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contract {
    pub id: Id,
    pub project_id: Id,
    pub group_id: Option<Id>,
    pub method: Method,
    pub path: String,
    pub summary: String,
    pub description: Option<String>,
    pub request_headers: Option<serde_json::Value>,
    pub request_params: Option<serde_json::Value>,
    pub request_body_schema: Option<serde_json::Value>,
    pub request_example: Option<serde_json::Value>,
    pub response_schema: serde_json::Value,
    pub response_example: Option<serde_json::Value>,
    pub auth_type: Option<AuthType>,
    pub status: Status,
    pub tags: Vec<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Compact form used by `list_contracts` and search results.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ContractSummary {
    pub id: Id,
    pub method: Method,
    pub path: String,
    pub summary: String,
    pub status: Status,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f32>,
}

/// Input for creating a contract. Drives the `create_contract` MCP tool
/// and the `create_contract` use case.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContractCreate {
    pub method: Method,
    pub path: String,
    pub summary: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub request_headers: Option<serde_json::Value>,
    #[serde(default)]
    pub request_params: Option<serde_json::Value>,
    #[serde(default)]
    pub request_body_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub request_example: Option<serde_json::Value>,
    pub response_schema: serde_json::Value,
    #[serde(default)]
    pub response_example: Option<serde_json::Value>,
    #[serde(default)]
    pub auth_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub group_name: Option<String>,
    /// Bypass the similarity warning. Still returns similar matches.
    #[serde(default)]
    pub force: bool,
}

impl ContractCreate {
    /// Validate. Returns `Err(DomainError::Validation)` for the first
    /// failing field.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.summary.trim().is_empty() {
            return Err(DomainError::Validation {
                field: "summary".to_owned(),
                message: "must not be empty".to_owned(),
            });
        }
        if self.summary.len() > 300 {
            return Err(DomainError::Validation {
                field: "summary".to_owned(),
                message: "must be at most 300 characters".to_owned(),
            });
        }
        if self.path.trim().is_empty() {
            return Err(DomainError::Validation {
                field: "path".to_owned(),
                message: "must not be empty".to_owned(),
            });
        }
        if !self.path.starts_with('/') {
            return Err(DomainError::Validation {
                field: "path".to_owned(),
                message: "must start with '/'".to_owned(),
            });
        }
        if self.path.len() > 500 {
            return Err(DomainError::Validation {
                field: "path".to_owned(),
                message: "must be at most 500 characters".to_owned(),
            });
        }
        if self.response_schema.is_null() {
            return Err(DomainError::Validation {
                field: "response_schema".to_owned(),
                message: "must be a non-null JSON Schema".to_owned(),
            });
        }
        if let Some(tags) = &self.tags {
            if tags.len() > 32 {
                return Err(DomainError::Validation {
                    field: "tags".to_owned(),
                    message: "must be at most 32 entries".to_owned(),
                });
            }
            for t in tags {
                if t.is_empty() || t.len() > 64 {
                    return Err(DomainError::Validation {
                        field: "tags".to_owned(),
                        message: "each tag must be 1-64 characters".to_owned(),
                    });
                }
            }
        }
        if let Some(status) = &self.status {
            if Status::parse(status).is_none() {
                return Err(DomainError::Validation {
                    field: "status".to_owned(),
                    message: "must be one of: draft, stable, deprecated".to_owned(),
                });
            }
        }
        Ok(())
    }

    /// Build the embedding input string (spec §4.2).
    #[must_use]
    pub fn embedding_input(&self) -> String {
        format!(
            "{} {} {} {}",
            self.method.as_str(),
            self.path,
            self.summary,
            self.description.as_deref().unwrap_or(""),
        )
    }
}

/// Input for updating a contract. Every field is optional; absent means
/// unchanged. `method` and `path` updates trigger embedding regeneration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct ContractUpdate {
    #[serde(default)]
    pub method: Option<Method>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub request_headers: Option<serde_json::Value>,
    #[serde(default)]
    pub request_params: Option<serde_json::Value>,
    #[serde(default)]
    pub request_body_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub request_example: Option<serde_json::Value>,
    #[serde(default)]
    pub response_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub response_example: Option<serde_json::Value>,
    #[serde(default)]
    pub auth_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub group_name: Option<String>,
}

impl ContractUpdate {
    /// True if any change touches a field that affects the embedding
    /// (spec §13 #3). Used by `update_contract` to decide whether to
    /// regenerate.
    #[must_use]
    pub fn affects_embedding(&self) -> bool {
        self.method.is_some()
            || self.path.is_some()
            || self.summary.is_some()
            || self.description.is_some()
    }

    /// True if the patch has at least one field set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.method.is_none()
            && self.path.is_none()
            && self.summary.is_none()
            && self.description.is_none()
            && self.request_headers.is_none()
            && self.request_params.is_none()
            && self.request_body_schema.is_none()
            && self.request_example.is_none()
            && self.response_schema.is_none()
            && self.response_example.is_none()
            && self.auth_type.is_none()
            && self.status.is_none()
            && self.tags.is_none()
            && self.group_name.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_parse_roundtrip() {
        for m in [Method::Get, Method::Post, Method::Put, Method::Patch, Method::Delete] {
            assert_eq!(Method::parse(m.as_str()), Some(m));
        }
        assert_eq!(Method::parse("get"), Some(Method::Get));
        assert_eq!(Method::parse("OPTION"), None);
    }

    #[test]
    fn status_parse_roundtrip() {
        assert_eq!(Status::parse("draft"), Some(Status::Draft));
        assert_eq!(Status::parse("DRAFT"), None);
    }

    #[test]
    fn auth_type_unknown_defaults_to_none() {
        assert_eq!(AuthType::parse_or_default("oauth"), AuthType::None);
        assert_eq!(AuthType::parse_or_default("BEARER"), AuthType::Bearer);
    }

    #[test]
    fn normalize_path_strips_trailing_slash() {
        assert_eq!(normalize_path("/api/users/"), "/api/users");
        assert_eq!(normalize_path("/api/users"), "/api/users");
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path("//"), "/");
    }

    fn make_valid_create() -> ContractCreate {
        ContractCreate {
            method: Method::Get,
            path: "/api/users".to_owned(),
            summary: "List users".to_owned(),
            description: None,
            request_headers: None,
            request_params: None,
            request_body_schema: None,
            request_example: None,
            response_schema: serde_json::json!({"type": "object"}),
            response_example: None,
            auth_type: None,
            status: None,
            tags: None,
            group_name: None,
            force: false,
        }
    }

    #[test]
    fn create_validate_accepts_minimum() {
        assert!(make_valid_create().validate().is_ok());
    }

    #[test]
    fn create_validate_rejects_empty_summary() {
        let mut c = make_valid_create();
        c.summary = "  ".to_owned();
        assert!(c.validate().is_err());
    }

    #[test]
    fn create_validate_rejects_path_without_slash() {
        let mut c = make_valid_create();
        c.path = "api/users".to_owned();
        let e = c.validate().unwrap_err();
        assert_eq!(e.field(), Some("path"));
    }

    #[test]
    fn create_validate_rejects_null_response_schema() {
        let mut c = make_valid_create();
        c.response_schema = serde_json::Value::Null;
        assert!(c.validate().is_err());
    }

    #[test]
    fn create_validate_rejects_too_many_tags() {
        let mut c = make_valid_create();
        c.tags = Some(vec!["t".to_owned(); 33]);
        assert!(c.validate().is_err());
    }

    #[test]
    fn embedding_input_includes_all_fields() {
        let c = ContractCreate {
            description: Some("Returns all users".to_owned()),
            ..make_valid_create()
        };
        let s = c.embedding_input();
        assert!(s.contains("GET"));
        assert!(s.contains("/api/users"));
        assert!(s.contains("List users"));
        assert!(s.contains("Returns all users"));
    }

    #[test]
    fn update_is_empty_default() {
        let u = ContractUpdate::default();
        assert!(u.is_empty());
        assert!(!u.affects_embedding());
    }

    #[test]
    fn update_affects_embedding_on_summary_change() {
        let u = ContractUpdate { summary: Some("New".to_owned()), ..Default::default() };
        assert!(u.affects_embedding());
    }
}
