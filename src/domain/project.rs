//! Project aggregate.

use crate::core::id::Id;
use crate::domain::errors::DomainError;
use schemars::{JsonSchema, schema::Schema};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// A validated project slug. Lowercase alphanumerics, dashes, underscores.
/// Min length 1, max length 64.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectSlug(String);

impl ProjectSlug {
    /// Validate and construct a slug. Returns `Err(DomainError::Validation)`
    /// for invalid input.
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(DomainError::Validation {
                field: "slug".to_owned(),
                message: "must not be empty".to_owned(),
            });
        }
        if s.len() > 64 {
            return Err(DomainError::Validation {
                field: "slug".to_owned(),
                message: "must be at most 64 characters".to_owned(),
            });
        }
        if !s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_') {
            return Err(DomainError::Validation {
                field: "slug".to_owned(),
                message: "must contain only lowercase a-z, 0-9, '-', '_'".to_owned(),
            });
        }
        Ok(Self(s.to_owned()))
    }

    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProjectSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl JsonSchema for ProjectSlug {
    fn schema_name() -> String {
        "ProjectSlug".to_owned()
    }
    fn json_schema(r#gen: &mut schemars::SchemaGenerator) -> Schema {
        <String as JsonSchema>::json_schema(r#gen)
    }
}

/// A project is a logical grouping of API contracts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: Id,
    pub slug: ProjectSlug,
    pub name: String,
    pub description: Option<String>,
    pub created_at: OffsetDateTime,
}

/// Input for creating a project.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectCreate {
    pub slug: ProjectSlug,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl ProjectCreate {
    /// Validate. Returns `Err(DomainError::Validation)` on failure.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.name.trim().is_empty() {
            return Err(DomainError::Validation {
                field: "name".to_owned(),
                message: "must not be empty".to_owned(),
            });
        }
        if self.name.len() > 200 {
            return Err(DomainError::Validation {
                field: "name".to_owned(),
                message: "must be at most 200 characters".to_owned(),
            });
        }
        if let Some(desc) = &self.description {
            if desc.len() > 2000 {
                return Err(DomainError::Validation {
                    field: "description".to_owned(),
                    message: "must be at most 2000 characters".to_owned(),
                });
            }
        }
        Ok(())
    }
}

/// Summary used by `list_projects` — excludes `description` and `created_at`
/// for compact responses.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProjectSummary {
    pub slug: ProjectSlug,
    pub name: String,
    pub contract_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::envelope::ApiErrorCode;

    #[test]
    fn slug_accepts_valid() {
        assert!(ProjectSlug::parse("my-api_v1").is_ok());
        assert!(ProjectSlug::parse("abc123").is_ok());
        assert!(ProjectSlug::parse("a").is_ok());
    }

    #[test]
    fn slug_rejects_empty() {
        let err = ProjectSlug::parse("").unwrap_err();
        assert_eq!(err.code(), ApiErrorCode::ValidationFailed);
        assert_eq!(err.field(), Some("slug"));
    }

    #[test]
    fn slug_rejects_uppercase() {
        assert!(ProjectSlug::parse("MyApi").is_err());
    }

    #[test]
    fn slug_rejects_spaces_and_special() {
        assert!(ProjectSlug::parse("my api").is_err());
        assert!(ProjectSlug::parse("my.api").is_err());
        assert!(ProjectSlug::parse("my/api").is_err());
    }

    #[test]
    fn slug_rejects_too_long() {
        let s = "a".repeat(65);
        assert!(ProjectSlug::parse(&s).is_err());
    }

    #[test]
    fn project_create_rejects_empty_name() {
        let p = ProjectCreate {
            slug: ProjectSlug::parse("ok").unwrap(),
            name: "  ".to_owned(),
            description: None,
        };
        assert!(p.validate().is_err());
    }

    #[test]
    fn project_create_accepts_minimum() {
        let p = ProjectCreate {
            slug: ProjectSlug::parse("ok").unwrap(),
            name: "My API".to_owned(),
            description: None,
        };
        assert!(p.validate().is_ok());
    }
}
