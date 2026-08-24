use crate::domain::auth::{AuthorizationCode, OAuthClient, OAuthToken, RefreshToken};
use crate::domain::errors::DomainError;
use crate::domain::ports::OAuthRepo;
use crate::infra::db::Db;
use crate::infra::repos::project_repo_sqlite::parse_id;
use async_trait::async_trait;
use rusqlite::{OptionalExtension, params};
use time::OffsetDateTime;

pub struct SqliteOAuthRepo {
    pub(crate) db: Db,
}

fn encode_list(values: &[String]) -> String {
    serde_json::to_string(values).unwrap_or_else(|_| "[]".to_owned())
}

fn decode_list(value: String) -> Result<Vec<String>, DomainError> {
    serde_json::from_str(&value).map_err(|error| DomainError::Internal(error.to_string()))
}

#[async_trait]
impl OAuthRepo for SqliteOAuthRepo {
    async fn register_client(&self, client: &OAuthClient) -> Result<(), DomainError> {
        let db = self.db.clone();
        let client = client.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.execute(
                    "INSERT INTO oauth_clients (client_id, client_name, redirect_uris, created_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        client.client_id,
                        client.client_name,
                        encode_list(&client.redirect_uris),
                        client.created_at.unix_timestamp()
                    ],
                )
                .map_err(|error| match error {
                    rusqlite::Error::SqliteFailure(failure, _)
                        if failure.code == rusqlite::ErrorCode::ConstraintViolation =>
                    {
                        DomainError::DuplicateKey {
                            resource: "oauth_client",
                            key: client.client_id,
                        }
                    }
                    other => DomainError::Internal(other.to_string()),
                })?;
                Ok(())
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn find_client(&self, client_id: &str) -> Result<Option<OAuthClient>, DomainError> {
        let db = self.db.clone();
        let client_id = client_id.to_owned();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.query_row(
                    "SELECT client_id, client_name, redirect_uris, created_at
                     FROM oauth_clients WHERE client_id = ?1",
                    [client_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(|error| DomainError::Internal(error.to_string()))?
                .map(|(client_id, client_name, redirect_uris, created_at)| {
                    Ok(OAuthClient {
                        client_id,
                        client_name,
                        redirect_uris: decode_list(redirect_uris)?,
                        created_at: OffsetDateTime::from_unix_timestamp(created_at)
                            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                    })
                })
                .transpose()
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn create_authorization_code(&self, code: &AuthorizationCode) -> Result<(), DomainError> {
        let db = self.db.clone();
        let code = code.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.execute(
                    "INSERT INTO oauth_authorization_codes
                     (code_hash, client_id, user_id, redirect_uri, scope, code_challenge,
                      code_challenge_method, expires_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        code.code_hash,
                        code.client_id,
                        code.user_id.to_string(),
                        code.redirect_uri,
                        encode_list(&code.scope),
                        code.code_challenge,
                        code.code_challenge_method,
                        code.expires_at.unix_timestamp()
                    ],
                )
                .map_err(|error| DomainError::Internal(error.to_string()))?;
                Ok(())
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn consume_authorization_code(
        &self,
        code_hash: &str,
        now: OffsetDateTime,
    ) -> Result<Option<AuthorizationCode>, DomainError> {
        let db = self.db.clone();
        let code_hash = code_hash.to_owned();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                let tx =
                    conn.transaction().map_err(|error| DomainError::Internal(error.to_string()))?;
                let row = tx
                    .query_row(
                        "SELECT code_hash, client_id, user_id, redirect_uri, scope, code_challenge,
                                code_challenge_method, expires_at
                         FROM oauth_authorization_codes
                         WHERE code_hash = ?1 AND consumed_at IS NULL AND expires_at > ?2",
                        params![code_hash, now.unix_timestamp()],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, String>(3)?,
                                row.get::<_, String>(4)?,
                                row.get::<_, String>(5)?,
                                row.get::<_, String>(6)?,
                                row.get::<_, i64>(7)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(|error| DomainError::Internal(error.to_string()))?;
                let Some((
                    hash,
                    client_id,
                    user_id,
                    redirect_uri,
                    scope,
                    challenge,
                    method,
                    expires_at,
                )) = row
                else {
                    tx.commit().map_err(|error| DomainError::Internal(error.to_string()))?;
                    return Ok(None);
                };
                tx.execute(
                    "UPDATE oauth_authorization_codes SET consumed_at = ?1
                     WHERE code_hash = ?2 AND consumed_at IS NULL",
                    params![now.unix_timestamp(), hash],
                )
                .map_err(|error| DomainError::Internal(error.to_string()))?;
                tx.commit().map_err(|error| DomainError::Internal(error.to_string()))?;
                Ok(Some(AuthorizationCode {
                    code_hash: hash,
                    client_id,
                    user_id: parse_id(&user_id)?,
                    redirect_uri,
                    scope: decode_list(scope)?,
                    code_challenge: challenge,
                    code_challenge_method: method,
                    expires_at: OffsetDateTime::from_unix_timestamp(expires_at)
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                }))
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn create_access_token(&self, token: &OAuthToken) -> Result<(), DomainError> {
        let db = self.db.clone();
        let token = token.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.execute(
                    "INSERT INTO oauth_access_tokens
                     (token_hash, client_id, user_id, scope, expires_at, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        token.token_hash,
                        token.client_id,
                        token.user_id.to_string(),
                        encode_list(&token.scope),
                        token.expires_at.unix_timestamp(),
                        OffsetDateTime::now_utc().unix_timestamp()
                    ],
                )
                .map_err(|error| DomainError::Internal(error.to_string()))?;
                Ok(())
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn find_access_token(
        &self,
        token_hash: &str,
        now: OffsetDateTime,
    ) -> Result<Option<OAuthToken>, DomainError> {
        let db = self.db.clone();
        let token_hash = token_hash.to_owned();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.query_row(
                    "SELECT token_hash, client_id, user_id, scope, expires_at, revoked_at
                     FROM oauth_access_tokens
                     WHERE token_hash = ?1 AND revoked_at IS NULL AND expires_at > ?2",
                    params![token_hash, now.unix_timestamp()],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, i64>(4)?,
                            row.get::<_, Option<i64>>(5)?,
                        ))
                    },
                )
                .optional()
                .map_err(|error| DomainError::Internal(error.to_string()))?
                .map(|(hash, client_id, user_id, scope, expires_at, revoked_at)| {
                    Ok(OAuthToken {
                        token_hash: hash,
                        client_id,
                        user_id: parse_id(&user_id)?,
                        scope: decode_list(scope)?,
                        expires_at: OffsetDateTime::from_unix_timestamp(expires_at)
                            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                        revoked_at: revoked_at.map(|value| {
                            OffsetDateTime::from_unix_timestamp(value)
                                .unwrap_or(OffsetDateTime::UNIX_EPOCH)
                        }),
                    })
                })
                .transpose()
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn revoke_access_token(
        &self,
        token_hash: &str,
        now: OffsetDateTime,
    ) -> Result<(), DomainError> {
        let db = self.db.clone();
        let token_hash = token_hash.to_owned();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.execute(
                    "UPDATE oauth_access_tokens SET revoked_at = ?1 WHERE token_hash = ?2 AND revoked_at IS NULL",
                    params![now.unix_timestamp(), token_hash],
                )
                .map_err(|error| DomainError::Internal(error.to_string()))?;
                Ok(())
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn create_refresh_token(&self, token: &RefreshToken) -> Result<(), DomainError> {
        let db = self.db.clone();
        let token = token.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                conn.execute(
                    "INSERT INTO oauth_refresh_tokens
                     (token_hash, client_id, user_id, scope, expires_at, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        token.token_hash,
                        token.client_id,
                        token.user_id.to_string(),
                        encode_list(&token.scope),
                        token.expires_at.unix_timestamp(),
                        OffsetDateTime::now_utc().unix_timestamp()
                    ],
                )
                .map_err(|error| DomainError::Internal(error.to_string()))?;
                Ok(())
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }

    async fn consume_refresh_token(
        &self,
        token_hash: &str,
        now: OffsetDateTime,
    ) -> Result<Option<RefreshToken>, DomainError> {
        let db = self.db.clone();
        let token_hash = token_hash.to_owned();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|conn| {
                let tx = conn.transaction().map_err(|error| DomainError::Internal(error.to_string()))?;
                let row = tx
                    .query_row(
                        "SELECT token_hash, client_id, user_id, scope, expires_at
                         FROM oauth_refresh_tokens
                         WHERE token_hash = ?1 AND revoked_at IS NULL AND expires_at > ?2",
                        params![token_hash, now.unix_timestamp()],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, String>(3)?,
                                row.get::<_, i64>(4)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(|error| DomainError::Internal(error.to_string()))?;
                let Some((hash, client_id, user_id, scope, expires_at)) = row else {
                    tx.commit().map_err(|error| DomainError::Internal(error.to_string()))?;
                    return Ok(None);
                };
                tx.execute(
                    "UPDATE oauth_refresh_tokens SET revoked_at = ?1 WHERE token_hash = ?2 AND revoked_at IS NULL",
                    params![now.unix_timestamp(), hash],
                )
                .map_err(|error| DomainError::Internal(error.to_string()))?;
                tx.commit().map_err(|error| DomainError::Internal(error.to_string()))?;
                Ok(Some(RefreshToken {
                    token_hash: hash,
                    client_id,
                    user_id: parse_id(&user_id)?,
                    scope: decode_list(scope)?,
                    expires_at: OffsetDateTime::from_unix_timestamp(expires_at)
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                    revoked_at: Some(now),
                }))
            })
        })
        .await
        .map_err(|error| DomainError::Internal(format!("join: {error}")))?
    }
}
