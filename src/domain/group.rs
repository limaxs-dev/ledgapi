//! Group aggregate — a named bucket of contracts within a project.

use crate::core::id::Id;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// A group within a project, e.g. `"Auth"`, `"User Management"`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Group {
    pub id: Id,
    pub project_id: Id,
    pub name: String,
    pub description: Option<String>,
}

/// Summary used by `list_groups`.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GroupSummary {
    pub id: Id,
    pub name: String,
    pub contract_count: i64,
}

/// Input for creating or resolving a group by name.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GroupRef {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl GroupRef {
    /// Validate the group name.
    ///
    /// # Errors
    /// Returns [`DomainError::Validation`] when the name is empty or
    /// longer than 100 characters.
    pub fn validate(&self) -> Result<(), crate::domain::errors::DomainError> {
        use crate::domain::errors::DomainError;
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::Validation {
                field: "group_name".to_owned(),
                message: "must not be empty".to_owned(),
            });
        }
        if name.len() > 100 {
            return Err(DomainError::Validation {
                field: "group_name".to_owned(),
                message: "must be at most 100 characters".to_owned(),
            });
        }
        Ok(())
    }
}

#[allow(dead_code)]
fn _unused(_: OffsetDateTime) {} // keep time import used if Group gains timestamps later

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::errors::DomainError;

    #[test]
    fn group_ref_rejects_empty() {
        let g = GroupRef { name: String::new(), description: None };
        assert!(
            matches!(g.validate(), Err(DomainError::Validation { ref field, .. }) if field == "group_name")
        );
    }

    #[test]
    fn group_ref_accepts_valid() {
        let g = GroupRef { name: "Auth".to_owned(), description: None };
        assert!(g.validate().is_ok());
    }
}
