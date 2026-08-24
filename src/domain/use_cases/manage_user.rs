use crate::domain::audit::{AuditAction, AuditResource};
use crate::domain::auth::{Principal, Role, User, UserCreate};
use crate::domain::errors::DomainError;
use crate::domain::ports::Repos;

pub async fn list(repos: &dyn Repos, principal: &Principal) -> Result<Vec<User>, DomainError> {
    principal.require_scope("ledgapi:admin")?;
    repos.users().list().await
}

pub async fn create(
    repos: &dyn Repos,
    principal: &Principal,
    input: UserCreate,
) -> Result<User, DomainError> {
    principal.require_scope("ledgapi:admin")?;
    input.validate()?;
    let user = repos.users().create(&input).await?;
    crate::domain::use_cases::audit::record(
        repos,
        principal,
        AuditAction::Create,
        AuditResource::User,
        Some(user.id),
        serde_json::json!({"username": user.username, "role": user.role.as_str()}),
    )
    .await?;
    Ok(user)
}

pub async fn update(
    repos: &dyn Repos,
    principal: &Principal,
    user: User,
) -> Result<User, DomainError> {
    principal.require_scope("ledgapi:admin")?;
    if user.id == principal.user_id && (!user.active || user.role != Role::SuperAdmin) {
        return Err(DomainError::Forbidden {
            message: "cannot disable or demote current super admin".to_owned(),
        });
    }
    let updated = repos.users().update(&user).await?;
    crate::domain::use_cases::audit::record(
        repos,
        principal,
        AuditAction::Update,
        AuditResource::User,
        Some(updated.id),
        serde_json::json!({"username": updated.username, "role": updated.role.as_str(), "active": updated.active}),
    )
    .await?;
    Ok(updated)
}

#[allow(dead_code)]
fn _role_marker(_: Role) {}
