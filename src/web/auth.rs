use crate::domain::auth::Principal;
use crate::domain::errors::DomainError;
use crate::errors::AppError;
use crate::infra::auth::{password, token};
use crate::state::AppState;
use crate::web::templates::LoginTpl;
use askama::Template;
use axum::body::Body;
use axum::extract::{Extension, Request};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;
use time::OffsetDateTime;

const SESSION_COOKIE: &str = "ledgapi_session";
const CSRF_COOKIE: &str = "ledgapi_csrf";

#[derive(Debug, Deserialize)]
struct LoginForm {
    username: String,
    password: String,
    #[serde(default)]
    next: String,
}

#[derive(Debug, Deserialize)]
struct LogoutForm {
    csrf: String,
}

pub async fn show_login(query: axum::extract::Query<LoginQuery>) -> Response {
    let next = query.next.as_deref().and_then(safe_next).unwrap_or("/");
    render_login(next, None)
}

#[derive(Debug, Deserialize, Default)]
pub struct LoginQuery {
    pub next: Option<String>,
}

pub async fn login(Extension(state): Extension<AppState>, req: Request) -> Response {
    let Ok(bytes) = axum::body::to_bytes(req.into_body(), 16 * 1024).await else {
        return render_login("/", Some("invalid login request"));
    };
    let Ok(form) = parse_form::<LoginForm>(&bytes) else {
        return render_login("/", Some("invalid username or password"));
    };
    let next = safe_next(&form.next).unwrap_or("/");
    let user = match state.repos().users().find_by_username(form.username.trim()).await {
        Ok(Some(user)) => user,
        Ok(None) | Err(DomainError::NotFound { .. }) => {
            let attempted = form.username.trim().to_owned();
            return render_login_with_username(
                next,
                Some("invalid username or password"),
                &attempted,
            );
        }
        Err(_) => return render_login(next, Some("invalid username or password")),
    };
    let valid = password::verify_password(&form.password, &user.password_hash).unwrap_or(false);
    if !valid || !user.active {
        let attempted = form.username.trim().to_owned();
        return render_login_with_username(next, Some("invalid username or password"), &attempted);
    }

    let raw_session = token::generate();
    let raw_csrf = token::generate();
    let now = OffsetDateTime::now_utc();
    let expires_at =
        now + time::Duration::seconds(state.config().auth.session_ttl.as_secs() as i64);
    let session = crate::domain::auth::Session {
        token_hash: token::sha256_hex(&raw_session),
        user_id: user.id,
        csrf_token_hash: token::sha256_hex(&raw_csrf),
        expires_at,
        revoked_at: None,
    };
    if state.repos().sessions().create(&session).await.is_err() {
        return render_login(next, Some("unable to create session"));
    }

    let mut response = Redirect::to(next).into_response();
    let cookie = cookie_header(
        SESSION_COOKIE,
        &raw_session,
        state.config().auth.session_ttl.as_secs(),
        state.config().auth.cookie_secure,
    );
    response.headers_mut().append(header::SET_COOKIE, cookie);
    response.headers_mut().append(
        header::SET_COOKIE,
        cookie_header(
            CSRF_COOKIE,
            &raw_csrf,
            state.config().auth.session_ttl.as_secs(),
            state.config().auth.cookie_secure,
        ),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store"));
    response
}

pub async fn logout(Extension(state): Extension<AppState>, req: Request) -> Response {
    let Some(raw_session) = cookie_value(&req, SESSION_COOKIE) else {
        return Redirect::to("/login").into_response();
    };
    let Some(session) = state
        .repos()
        .sessions()
        .find(&token::sha256_hex(&raw_session), OffsetDateTime::now_utc())
        .await
        .ok()
        .flatten()
    else {
        return clear_cookie_response();
    };
    let bytes = axum::body::to_bytes(req.into_body(), 16 * 1024).await.unwrap_or_default();
    let Ok(form) = parse_form::<LogoutForm>(&bytes) else {
        return (StatusCode::FORBIDDEN, "csrf validation failed").into_response();
    };
    if !token::constant_time_eq(&session.csrf_token_hash, &token::sha256_hex(&form.csrf)) {
        return (StatusCode::FORBIDDEN, "csrf validation failed").into_response();
    }
    let _ = state.repos().sessions().revoke(&session.token_hash, OffsetDateTime::now_utc()).await;
    clear_cookie_response()
}

pub async fn require_web_auth(
    mut req: Request<Body>,
    next: Next,
    state: AppState,
) -> Result<Response, AppError> {
    let Some(raw_session) = cookie_value(&req, SESSION_COOKIE) else {
        return Ok(login_redirect(req.uri().path()));
    };
    let Some(session) = state
        .repos()
        .sessions()
        .find(&token::sha256_hex(&raw_session), OffsetDateTime::now_utc())
        .await
        .map_err(AppError::from)?
    else {
        return Ok(login_redirect(req.uri().path()));
    };
    let Some(user) =
        state.repos().users().find_by_id(session.user_id).await.map_err(AppError::from)?
    else {
        return Ok(login_redirect(req.uri().path()));
    };
    if !user.active {
        return Ok(login_redirect(req.uri().path()));
    }
    let scopes = match user.role {
        crate::domain::auth::Role::SuperAdmin => {
            vec!["ledgapi:read", "ledgapi:write", "ledgapi:admin"]
        }
        crate::domain::auth::Role::Editor => vec!["ledgapi:read", "ledgapi:write"],
        crate::domain::auth::Role::Viewer => vec!["ledgapi:read"],
    }
    .into_iter()
    .map(str::to_owned)
    .collect();
    req.extensions_mut().insert(Principal {
        user_id: user.id,
        username: user.username,
        role: user.role,
        client_id: None,
        scopes,
    });
    Ok(next.run(req).await)
}

fn render_login(next: &str, error: Option<&str>) -> Response {
    render_login_with_username(next, error, "")
}

fn render_login_with_username(next: &str, error: Option<&str>, username: &str) -> Response {
    let tpl = LoginTpl { next, error, username };
    let mut response = Html(tpl.render().unwrap_or_default()).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store"));
    response
}

fn login_redirect(path: &str) -> Response {
    let location = format!("/login?next={}", urlencoding::encode(path));
    let mut response = (StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store"));
    response
}

fn clear_cookie_response() -> Response {
    let mut response = Redirect::to("/login").into_response();
    response.headers_mut().append(header::SET_COOKIE, expired_cookie(SESSION_COOKIE));
    response.headers_mut().append(header::SET_COOKIE, expired_cookie(CSRF_COOKIE));
    response
}

fn cookie_header(name: &str, value: &str, max_age: u64, secure: bool) -> header::HeaderValue {
    let secure_suffix = if secure { "; Secure" } else { "" };
    header::HeaderValue::from_str(&format!(
        "{name}={value}; Path=/; Max-Age={max_age}; HttpOnly; SameSite=Lax{secure_suffix}"
    ))
    .expect("session cookie is valid")
}

fn expired_cookie(name: &str) -> header::HeaderValue {
    header::HeaderValue::from_str(&format!("{name}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax"))
        .expect("expired cookie is valid")
}

pub(crate) fn cookie_value(req: &Request, name: &str) -> Option<String> {
    let value = req.headers().get(header::COOKIE)?.to_str().ok()?;
    value.split(';').find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        (key == name).then(|| value.to_owned())
    })
}

pub(crate) fn csrf_cookie_value(req: &Request) -> Option<String> {
    cookie_value(req, CSRF_COOKIE)
}

pub(crate) async fn session_principal_for_cookie(
    state: &AppState,
    raw_session: Option<&str>,
) -> Result<Option<Principal>, DomainError> {
    let Some(raw_session) = raw_session else {
        return Ok(None);
    };
    let Some(session) = state
        .repos()
        .sessions()
        .find(&token::sha256_hex(raw_session), OffsetDateTime::now_utc())
        .await?
    else {
        return Ok(None);
    };
    let Some(user) = state.repos().users().find_by_id(session.user_id).await? else {
        return Ok(None);
    };
    if !user.active {
        return Ok(None);
    }
    let scopes = match user.role {
        crate::domain::auth::Role::SuperAdmin => {
            vec!["ledgapi:read", "ledgapi:write", "ledgapi:admin"]
        }
        crate::domain::auth::Role::Editor => vec!["ledgapi:read", "ledgapi:write"],
        crate::domain::auth::Role::Viewer => vec!["ledgapi:read"],
    };
    Ok(Some(Principal {
        user_id: user.id,
        username: user.username,
        role: user.role,
        client_id: None,
        scopes: scopes.into_iter().map(str::to_owned).collect(),
    }))
}

fn safe_next(value: &str) -> Option<&str> {
    (!value.is_empty()
        && value.starts_with('/')
        && !value.starts_with("//")
        && !value.contains('\\'))
    .then_some(value)
}

fn parse_form<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, ()> {
    let text = std::str::from_utf8(bytes).map_err(|_| ())?;
    let mut pairs = Vec::new();
    for pair in text.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = decode_form_component(key)?;
        let value = decode_form_component(value)?;
        pairs.push((key, value));
    }
    serde_json::from_value(serde_json::Value::Object(
        pairs.into_iter().map(|(k, v)| (k, v.into())).collect(),
    ))
    .map_err(|_| ())
}

#[allow(clippy::result_unit_err)]
pub fn decode_form_component(component: &str) -> Result<String, ()> {
    urlencoding::decode(&component.replace('+', "%20"))
        .map(std::borrow::Cow::into_owned)
        .map_err(|_| ())
}
