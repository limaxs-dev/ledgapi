use crate::core::id::Id;
use crate::domain::audit::{AuditAction, AuditEvent, AuditResource};
use crate::domain::auth::Principal;
use crate::domain::errors::DomainError;
use crate::domain::ports::Repos;
use serde_json::Value;
use time::OffsetDateTime;

pub async fn record(
    repos: &dyn Repos,
    principal: &Principal,
    action: AuditAction,
    resource: AuditResource,
    resource_id: Option<Id>,
    metadata: Value,
) -> Result<(), DomainError> {
    repos
        .audit()
        .append(&AuditEvent {
            actor_user_id: Some(principal.user_id),
            action,
            resource,
            resource_id,
            metadata,
            created_at: OffsetDateTime::now_utc(),
        })
        .await
        .map(|_| ())
}
