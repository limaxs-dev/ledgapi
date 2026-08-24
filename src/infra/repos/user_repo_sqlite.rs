use crate::core::id::Id;
use crate::domain::auth::{Role, User, UserCreate};
use crate::domain::errors::DomainError;
use crate::domain::ports::UserRepo;
use crate::infra::db::Db;
use crate::infra::repos::project_repo_sqlite::parse_id;
use async_trait::async_trait;
use rusqlite::{OptionalExtension, params};
use time::OffsetDateTime;

pub struct SqliteUserRepo {
    pub(crate) db: Db,
}

fn parse_role(value: &str) -> Result<Role, DomainError> {
    Role::parse(value).map_err(|error| DomainError::Internal(error.to_string()))
}

fn load_user_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(String, String, String, String, i64, i64, i64)> {
    Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, String>(3)?,
        row.get::<_, i64>(4)?,
        row.get::<_, i64>(5)?,
        row.get::<_, i64>(6)?,
    ))
}

fn map_user(row: (String, String, String, String, i64, i64, i64)) -> Result<User, DomainError> {
    let (id, username, password_hash, role, active, created_at, updated_at) = row;
    Ok(User {
        id: parse_id(&id)?,
        username,
        password_hash,
        role: parse_role(&role)?,
        active: active != 0,
        created_at: OffsetDateTime::from_unix_timestamp(created_at)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
        updated_at: OffsetDateTime::from_unix_timestamp(updated_at)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
    })
}

#[async_trait]
impl UserRepo for SqliteUserRepo {
    async fn find_by_id(&self, id: Id) -> Result<Option<User>, DomainError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.query_row(
                    "SELECT id, username, password_hash, role, active, created_at, updated_at FROM users WHERE id = ?1",
                    [id.to_string()],
                    load_user_row,
                )
                .optional()
                .map_err(|error| DomainError::Internal(error.to_string()))?
                .map(map_user)
                .transpose()
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, DomainError> {
        let db = self.db.clone();
        let username = username.to_owned();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.query_row(
                    "SELECT id, username, password_hash, role, active, created_at, updated_at FROM users WHERE username = ?1",
                    [username],
                    load_user_row,
                )
                .optional()
                .map_err(|error| DomainError::Internal(error.to_string()))?
                .map(map_user)
                .transpose()
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn count(&self) -> Result<i64, DomainError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
                    .map_err(|error| DomainError::Internal(error.to_string()))
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn create(&self, input: &UserCreate) -> Result<User, DomainError> {
        let db = self.db.clone();
        let input = input.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                let id = Id::new();
                let now = OffsetDateTime::now_utc();
                conn.execute(
                    "INSERT INTO users (id, username, password_hash, role, active, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
                    params![
                        id.to_string(),
                        input.username,
                        input.password_hash,
                        input.role.as_str(),
                        now.unix_timestamp()
                    ],
                )
                .map_err(|error| match error {
                    rusqlite::Error::SqliteFailure(failure, _)
                        if failure.code == rusqlite::ErrorCode::ConstraintViolation =>
                    {
                        DomainError::DuplicateKey {
                            resource: "user",
                            key: input.username.clone(),
                        }
                    }
                    other => DomainError::Internal(other.to_string()),
                })?;
                Ok(User {
                    id,
                    username: input.username,
                    password_hash: input.password_hash,
                    role: input.role,
                    active: true,
                    created_at: now,
                    updated_at: now,
                })
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn update(&self, user: &User) -> Result<User, DomainError> {
        let db = self.db.clone();
        let user = user.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                let now = OffsetDateTime::now_utc();
                let changed = conn
                    .execute(
                        "UPDATE users SET username = ?1, password_hash = ?2, role = ?3, active = ?4, updated_at = ?5 WHERE id = ?6",
                        params![
                            user.username,
                            user.password_hash,
                            user.role.as_str(),
                            i64::from(user.active),
                            now.unix_timestamp(),
                            user.id.to_string()
                        ],
                    )
                    .map_err(|error| DomainError::Internal(error.to_string()))?;
                if changed == 0 {
                    return Err(DomainError::NotFound { resource: "user" });
                }
                Ok(User { updated_at: now, ..user })
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn list(&self) -> Result<Vec<User>, DomainError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                let mut statement = conn
                    .prepare("SELECT id, username, password_hash, role, active, created_at, updated_at FROM users ORDER BY username")
                    .map_err(|error| DomainError::Internal(error.to_string()))?;
                let rows = statement
                    .query_map([], load_user_row)
                    .map_err(|error| DomainError::Internal(error.to_string()))?;
                rows.map(|row| {
                    row.map_err(|error| DomainError::Internal(error.to_string()))?
                        .pipe(map_user)
                })
                .collect()
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }
}

trait Pipe: Sized {
    fn pipe<T>(self, function: impl FnOnce(Self) -> T) -> T {
        function(self)
    }
}

impl<T> Pipe for T {}
