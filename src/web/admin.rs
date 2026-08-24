use crate::domain::auth::{Principal, Role, UserCreate};
use crate::domain::errors::DomainError;
use crate::infra::auth::{password, token};
use crate::state::AppState;
use crate::web::auth::{cookie_value, csrf_cookie_value, session_principal_for_cookie};
use crate::web::templates::{AdminUserRow, AdminUsersTpl, AuditPageRow, AuditTpl};
use askama::Template;
use axum::extract::{Extension, Request};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct UserForm {
    username: String,
    password: String,
    role: String,
    csrf: String,
}

pub async fn users(Extension(state): Extension<AppState>, req: Request) -> Response {
    let Some(principal) =
        session_principal_for_cookie(&state, cookie_value(&req, "ledgapi_session").as_deref())
            .await
            .ok()
            .flatten()
    else {
        return (StatusCode::UNAUTHORIZED, "authentication required").into_response();
    };
    if !principal.role.can_manage_users() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let csrf = csrf_cookie_value(&req).unwrap_or_default();
    let users = crate::domain::use_cases::manage_user::list(&*state.repos, &principal)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|user| AdminUserRow {
            username: user.username,
            role: user.role.as_str().to_owned(),
            status: if user.active { "active" } else { "inactive" }.to_owned(),
        })
        .collect();
    let tpl = AdminUsersTpl { users, csrf: &csrf, error: None };
    Html(tpl.render().unwrap_or_default()).into_response()
}

pub async fn audit(Extension(state): Extension<AppState>, req: Request) -> Response {
    let Some(principal) =
        session_principal_for_cookie(&state, cookie_value(&req, "ledgapi_session").as_deref())
            .await
            .ok()
            .flatten()
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !principal.role.can_manage_users() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let entries = state
        .repos()
        .audit()
        .list(&crate::domain::audit::AuditFilter { limit: 100, ..Default::default() })
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|entry| AuditPageRow {
            actor: entry.actor_username.unwrap_or_else(|| "system".to_owned()),
            action: entry.action.as_str().to_owned(),
            resource: entry.resource.as_str().to_owned(),
            created_at: entry.created_at.to_string(),
        })
        .collect();
    Html(AuditTpl { entries }.render().unwrap_or_default()).into_response()
}

pub async fn create_user(Extension(state): Extension<AppState>, req: Request) -> Response {
    let Some(principal) =
        session_principal_for_cookie(&state, cookie_value(&req, "ledgapi_session").as_deref())
            .await
            .ok()
            .flatten()
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !principal.role.can_manage_users() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(csrf_cookie) = csrf_cookie_value(&req) else {
        return (StatusCode::FORBIDDEN, "csrf validation failed").into_response();
    };
    let Ok(bytes) = axum::body::to_bytes(req.into_body(), 16 * 1024).await else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(form) = parse_form::<UserForm>(&bytes) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if !token::constant_time_eq(&token::sha256_hex(&csrf_cookie), &token::sha256_hex(&form.csrf)) {
        return (StatusCode::FORBIDDEN, "csrf validation failed").into_response();
    }
    let Ok(role) = Role::parse(&form.role) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(password_hash) = password::hash_password(&form.password) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    match crate::domain::use_cases::manage_user::create(
        &*state.repos,
        &principal,
        UserCreate { username: form.username, password_hash, role },
    )
    .await
    {
        Ok(_) => (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users")]).into_response(),
        Err(DomainError::DuplicateKey { .. }) => StatusCode::CONFLICT.into_response(),
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

fn parse_form<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, ()> {
    let text = std::str::from_utf8(bytes).map_err(|_| ())?;
    let mut object = serde_json::Map::new();
    for pair in text.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        object.insert(
            urlencoding::decode(key).map_err(|_| ())?.into_owned(),
            urlencoding::decode(value).map_err(|_| ())?.into_owned().into(),
        );
    }
    serde_json::from_value(serde_json::Value::Object(object)).map_err(|_| ())
}

#[allow(dead_code)]
fn _principal_marker(_: Principal) {}
