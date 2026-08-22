//! Bearer-token auth middleware. Applied only to `/mcp`.

use crate::core::id::Id;
use crate::domain::errors::DomainError;
use crate::errors::AppError;
use crate::infra::auth::token;
use crate::state::AppState;
use axum::extract::Request;
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;

/// Extract the bearer token from the `Authorization` header. Returns
/// `None` if the header is missing, malformed, or has the wrong scheme.
fn extract_bearer(req: &Request) -> Option<&str> {
    let header = req.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?.trim();
    if token.is_empty() || token.len() != 64 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(token)
}

/// axum middleware: enforces Bearer token on every request that hits
/// the protected route. The `AppState` is passed explicitly (since the
/// router no longer carries state via `with_state`); see
/// `web::router::router`.
pub async fn bearer_auth(
    req: Request,
    next: Next,
    state: AppState,
) -> Result<Response, AppError> {
    let token = extract_bearer(&req)
        .ok_or_else(|| AppError::from(DomainError::AuthMissing))?;

    let hash = token::sha256_hex(token);
    let valid = state
        .repos()
        .tokens()
        .exists(&hash)
        .await
        .map_err(AppError::from)?;
    if !valid {
        return Err(AppError::from(DomainError::AuthInvalid));
    }

    state.mark_setup_consumed();

    Ok(next.run(req).await)
}

/// Silence "unused" until `Id` is referenced by other middleware code.
#[allow(dead_code)]
fn _id_marker(_: Id) {}

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

    fn req_no_header() -> Request {
        AxumRequest::builder()
            .method(Method::GET)
            .uri("/mcp")
            .body(Body::empty())
            .unwrap()
    }

    #[test]
    fn extract_bearer_accepts_well_formed() {
        let r = req_with_header(
            "Bearer 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        assert!(extract_bearer(&r).is_some());
    }

    #[test]
    fn extract_bearer_rejects_missing() {
        assert!(extract_bearer(&req_no_header()).is_none());
    }

    #[test]
    fn extract_bearer_rejects_wrong_scheme() {
        let r = req_with_header("Basic dXNlcjpwYXNz");
        assert!(extract_bearer(&r).is_none());
    }

    #[test]
    fn extract_bearer_rejects_bad_length() {
        let r = req_with_header("Bearer abc");
        assert!(extract_bearer(&r).is_none());
    }

    #[test]
    fn extract_bearer_rejects_non_hex() {
        let r = req_with_header(
            "Bearer zzzz56789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        assert!(extract_bearer(&r).is_none());
    }
}
