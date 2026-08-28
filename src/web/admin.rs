use crate::core::id::Id;
use crate::domain::auth::{Principal, Role, User, UserCreate};
use crate::domain::errors::DomainError;
use crate::domain::ports::Repos;
use crate::infra::auth::{password, token};
use crate::state::AppState;
use crate::web::auth::{cookie_value, csrf_cookie_value, session_principal_for_cookie};
use crate::web::templates::{AdminUserRow, AdminUsersTpl, AuditPageRow, AuditTpl};
use askama::Template;
use axum::extract::{Extension, Path, Request};
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
    let flash = req
        .uri()
        .query()
        .and_then(|q| {
            q.split('&').find_map(|pair| pair.split_once('=').filter(|(k, _)| *k == "flash"))
        })
        .map(|(_, v)| v.to_owned())
        .unwrap_or_default();
    let (error, success) = match flash.as_str() {
        "created" => (None, Some("User created.")),
        "updated" => (None, Some("User updated.")),
        "duplicate" => (Some("That username already exists."), None),
        "notfound" => (Some("No such user."), None),
        "invalid" => (
            Some("Could not apply that change. Check the role, active flag, or password (minimum 12 characters)."),
            None,
        ),
        _ => (None, None),
    };
    let csrf = csrf_cookie_value(&req).unwrap_or_default();
    let users = crate::domain::use_cases::manage_user::list(&*state.repos, &principal)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|user| AdminUserRow {
            id: user.id.to_string(),
            username: user.username,
            role: user.role.as_str().to_owned(),
            status: if user.active { "active" } else { "inactive" }.to_owned(),
            is_self: user.id == principal.user_id,
        })
        .collect();
    let tpl = AdminUsersTpl { users, csrf: &csrf, error, success };
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
            created_at: crate::web::handlers::format_dt(entry.created_at),
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
    let invalid_redirect =
        (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users?flash=invalid")]).into_response();
    let Ok(role) = Role::parse(&form.role) else {
        return invalid_redirect;
    };
    let Ok(password_hash) = password::hash_password(&form.password) else {
        return invalid_redirect;
    };
    match crate::domain::use_cases::manage_user::create(
        &*state.repos,
        &principal,
        UserCreate { username: form.username, password_hash, role },
    )
    .await
    {
        Ok(_) => (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users?flash=created")])
            .into_response(),
        Err(DomainError::DuplicateKey { .. }) => {
            (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users?flash=duplicate")])
                .into_response()
        }
        Err(_) => (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users?flash=invalid")])
            .into_response(),
    }
}

fn parse_form<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, ()> {
    let text = std::str::from_utf8(bytes).map_err(|_| ())?;
    let mut object = serde_json::Map::new();
    for pair in text.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        object.insert(
            super::auth::decode_form_component(key)?,
            super::auth::decode_form_component(value)?.into(),
        );
    }
    serde_json::from_value(serde_json::Value::Object(object)).map_err(|_| ())
}

#[derive(Debug, Deserialize)]
struct UpdateUserForm {
    role: String,
    active: String,
    csrf: String,
}

#[derive(Debug, Deserialize)]
struct PasswordForm {
    password: String,
    csrf: String,
}

/// POST /admin/users/{id}/update — change role and active state.
/// Flash values used on /admin/users redirect:
///   - `updated`      : success
///   - `invalid`      : role unknown or self-demote/self-deactivate attempt
///   - `notfound`     : no such user
pub async fn handle_update_user(
    Extension(state): Extension<AppState>,
    Path(id): Path<String>,
    req: Request,
) -> Response {
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
    let Some(csrf_cookie) = csrf_cookie_value(&req) else {
        return (StatusCode::FORBIDDEN, "csrf validation failed").into_response();
    };
    let Ok(bytes) = axum::body::to_bytes(req.into_body(), 16 * 1024).await else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(form) = parse_form::<UpdateUserForm>(&bytes) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if !token::constant_time_eq(&token::sha256_hex(&csrf_cookie), &token::sha256_hex(&form.csrf)) {
        return (StatusCode::FORBIDDEN, "csrf validation failed").into_response();
    }
    let invalid_redirect =
        (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users?flash=invalid")]).into_response();
    let Ok(role) = Role::parse(&form.role) else {
        return invalid_redirect;
    };
    let active = match form.active.as_str() {
        "true" | "1" | "on" => true,
        "false" | "0" | "" => false,
        _ => return invalid_redirect,
    };
    let Some(target_id) = Id::parse(&id) else {
        return (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users?flash=notfound")])
            .into_response();
    };
    let Some(target) = state.repos.users().find_by_id(target_id).await.ok().flatten() else {
        return (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users?flash=notfound")])
            .into_response();
    };
    let mut updated: User = target;
    updated.role = role;
    updated.active = active;
    match crate::domain::use_cases::manage_user::update(&*state.repos, &principal, updated).await {
        Ok(_) => (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users?flash=updated")])
            .into_response(),
        Err(DomainError::Forbidden { .. } | _) => invalid_redirect,
    }
}

/// POST /admin/users/{id}/password — set a new initial password.
pub async fn handle_reset_password(
    Extension(state): Extension<AppState>,
    Path(id): Path<String>,
    req: Request,
) -> Response {
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
    let Some(csrf_cookie) = csrf_cookie_value(&req) else {
        return (StatusCode::FORBIDDEN, "csrf validation failed").into_response();
    };
    let Ok(bytes) = axum::body::to_bytes(req.into_body(), 16 * 1024).await else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(form) = parse_form::<PasswordForm>(&bytes) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if !token::constant_time_eq(&token::sha256_hex(&csrf_cookie), &token::sha256_hex(&form.csrf)) {
        return (StatusCode::FORBIDDEN, "csrf validation failed").into_response();
    }
    let invalid_redirect =
        (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users?flash=invalid")]).into_response();
    let Some(target_id) = Id::parse(&id) else {
        return (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users?flash=notfound")])
            .into_response();
    };
    let Some(mut target) = state.repos.users().find_by_id(target_id).await.ok().flatten() else {
        return (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users?flash=notfound")])
            .into_response();
    };
    let Ok(password_hash) = password::hash_password(&form.password) else {
        return invalid_redirect;
    };
    target.password_hash = password_hash;
    match crate::domain::use_cases::manage_user::update(&*state.repos, &principal, target).await {
        Ok(_) => (StatusCode::SEE_OTHER, [(header::LOCATION, "/admin/users?flash=updated")])
            .into_response(),
        Err(_) => invalid_redirect,
    }
}

#[allow(dead_code)]
fn _principal_marker(_: Principal) {}
