use crate::domain::auth::{Role, UserCreate};
use crate::domain::errors::DomainError;
use crate::domain::ports::{Repos, UserRepo};

pub async fn ensure(
    users: &dyn UserRepo,
    username: Option<&str>,
    password_hash: Option<&str>,
) -> Result<bool, DomainError> {
    if users.count().await? > 0 {
        return Ok(false);
    }

    let username = username.ok_or_else(|| DomainError::Validation {
        field: "auth.initial_admin_username".to_owned(),
        message: "is required when no users exist".to_owned(),
    })?;
    let password_hash = password_hash.ok_or_else(|| DomainError::Validation {
        field: "auth.initial_admin_password".to_owned(),
        message: "is required when no users exist".to_owned(),
    })?;
    let input = UserCreate {
        username: username.to_owned(),
        password_hash: password_hash.to_owned(),
        role: Role::SuperAdmin,
    };
    input.validate()?;
    users.create(&input).await?;
    Ok(true)
}

#[allow(dead_code)]
fn _repos_marker(_: &dyn Repos) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::db::pool::open_memory;
    use crate::infra::repos::SqliteRepos;

    #[tokio::test]
    async fn creates_one_super_admin_and_is_idempotent() {
        let repos = SqliteRepos::new(open_memory().unwrap());
        assert!(ensure(repos.users(), Some("admin"), Some("argon-hash")).await.unwrap());
        assert!(!ensure(repos.users(), Some("other"), Some("other-hash")).await.unwrap());
        let admin = repos.users().find_by_username("admin").await.unwrap().unwrap();
        assert_eq!(admin.role, Role::SuperAdmin);
        assert_eq!(admin.password_hash, "argon-hash");
    }

    #[tokio::test]
    async fn requires_credentials_on_empty_database() {
        let repos = SqliteRepos::new(open_memory().unwrap());
        assert!(ensure(repos.users(), None, None).await.is_err());
    }
}
