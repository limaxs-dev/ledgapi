use crate::domain::auth::{Principal, Role};
use crate::domain::errors::DomainError;
use crate::errors::AppError;
use crate::infra::auth::token;
use crate::state::AppState;
use axum::extract::Request;
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use time::OffsetDateTime;

fn extract_bearer(req: &Request) -> Option<&str> {
    let header = req.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?.trim();
    (!token.is_empty()).then_some(token)
}

fn role_scopes(role: Role) -> Vec<&'static str> {
    match role {
        Role::SuperAdmin => vec!["ledgapi:read", "ledgapi:write", "ledgapi:admin"],
        Role::Editor => vec!["ledgapi:read", "ledgapi:write"],
        Role::Viewer => vec!["ledgapi:read"],
    }
}

pub async fn bearer_auth(
    mut req: Request,
    next: Next,
    state: AppState,
) -> Result<Response, AppError> {
    let raw_token = extract_bearer(&req).ok_or_else(|| AppError::from(DomainError::AuthMissing))?;
    let token_hash = token::sha256_hex(raw_token);
    let oauth_token = state
        .repos()
        .oauth()
        .find_access_token(&token_hash, OffsetDateTime::now_utc())
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::from(DomainError::AuthInvalid))?;
    let user = state
        .repos()
        .users()
        .find_by_id(oauth_token.user_id)
        .await
        .map_err(AppError::from)?
        .filter(|user| user.active)
        .ok_or_else(|| AppError::from(DomainError::AuthInvalid))?;
    let allowed_scopes = role_scopes(user.role);
    let scopes = oauth_token
        .scope
        .into_iter()
        .filter(|scope| allowed_scopes.contains(&scope.as_str()))
        .collect();
    req.extensions_mut().insert(Principal {
        user_id: user.id,
        username: user.username,
        role: user.role,
        client_id: Some(oauth_token.client_id),
        scopes,
    });
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request as AxumRequest};

    fn req_with_header(value: &str) -> Request {
        AxumRequest::builder()
            .method(Method::GET)
            .uri("/mcp")
            .header(AUTHORIZATION, value)
            .body(Body::empty())
            .unwrap()
    }

    #[test]
    fn extract_bearer_accepts_opaque_token() {
        assert_eq!(extract_bearer(&req_with_header("Bearer opaque.token")), Some("opaque.token"));
    }

    #[test]
    fn extract_bearer_rejects_missing_or_wrong_scheme() {
        assert!(extract_bearer(&req_with_header("Basic abc")).is_none());
        let request = AxumRequest::builder().uri("/mcp").body(Body::empty()).unwrap();
        assert!(extract_bearer(&request).is_none());
    }
}
