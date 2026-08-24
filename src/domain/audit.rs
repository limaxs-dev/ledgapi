use crate::core::id::Id;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Create,
    Update,
    Delete,
}

impl AuditAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditResource {
    User,
    Project,
    Group,
    Contract,
}

impl AuditResource {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
            Self::Group => "group",
            Self::Contract => "contract",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub actor_user_id: Option<Id>,
    pub action: AuditAction,
    pub resource: AuditResource,
    pub resource_id: Option<Id>,
    pub metadata: Value,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Id,
    pub actor_user_id: Option<Id>,
    pub actor_username: Option<String>,
    pub action: AuditAction,
    pub resource: AuditResource,
    pub resource_id: Option<Id>,
    pub metadata: Value,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditFilter {
    pub actor_user_id: Option<Id>,
    pub action: Option<AuditAction>,
    pub resource: Option<AuditResource>,
    pub limit: i64,
    pub offset: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_event_serializes_stable_action_and_resource_names() {
        let value = serde_json::to_value(AuditEvent {
            actor_user_id: None,
            action: AuditAction::Update,
            resource: AuditResource::Contract,
            resource_id: None,
            metadata: serde_json::json!({"path": "/users"}),
            created_at: OffsetDateTime::UNIX_EPOCH,
        })
        .unwrap();
        assert_eq!(value["action"], "update");
        assert_eq!(value["resource"], "contract");
    }
}
