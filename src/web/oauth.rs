use crate::domain::auth::{AuthorizationCode, OAuthClient, OAuthToken, Principal, RefreshToken};
use crate::domain::errors::DomainError;
use crate::infra::auth::token;
use crate::state::AppState;
use crate::web::auth::{cookie_value, csrf_cookie_value, session_principal_for_cookie};
use crate::web::templates::OAuthConsentTpl;
use askama::Template;
use axum::Json;
use axum::extract::{Extension, Query, Request};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use time::OffsetDateTime;

const READ_SCOPE: &str = "ledgapi:read";
const WRITE_SCOPE: &str = "ledgapi:write";
const ADMIN_SCOPE: &str = "ledgapi:admin";

#[derive(Debug, Deserialize)]
pub struct RegistrationRequest {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RegistrationResponse {
    client_id: String,
    client_name: String,
    redirect_uris: Vec<String>,
    token_endpoint_auth_method: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeQuery {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConsentForm {
    pub decision: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scope: String,
    #[serde(default)]
    pub state: String,
    pub csrf: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenForm {
    pub grant_type: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub code_verifier: Option<String>,
}

#[derive(Debug, Serialize)]
struct TokenResponse {
    access_token: String,
    token_type: &'static str,
    expires_in: u64,
    refresh_token: String,
    scope: String,
}

pub async fn protected_resource_metadata(Extension(state): Extension<AppState>) -> Response {
    json_response(json!({
        "resource": format!("{}/mcp", state.config().auth.issuer.trim_end_matches('/')),
        "authorization_servers": [state.config().auth.issuer],
        "scopes_supported": [READ_SCOPE, WRITE_SCOPE, ADMIN_SCOPE]
    }))
}

pub async fn authorization_server_metadata(Extension(state): Extension<AppState>) -> Response {
    let issuer = state.config().auth.issuer.trim_end_matches('/');
    json_response(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/oauth/authorize"),
        "token_endpoint": format!("{issuer}/oauth/token"),
        "registration_endpoint": format!("{issuer}/oauth/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": [READ_SCOPE, WRITE_SCOPE, ADMIN_SCOPE]
    }))
}

pub async fn register(
    Extension(state): Extension<AppState>,
    Json(input): Json<RegistrationRequest>,
) -> Response {
    if input.client_name.trim().is_empty() || input.redirect_uris.is_empty() {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_client_metadata");
    }
    if input.redirect_uris.iter().any(|uri| !valid_redirect_uri(uri)) {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_redirect_uri");
    }
    let client_id = token::generate();
    let client = OAuthClient {
        client_id: client_id.clone(),
        client_name: input.client_name.trim().to_owned(),
        redirect_uris: input.redirect_uris.clone(),
        created_at: OffsetDateTime::now_utc(),
    };
    if state.repos().oauth().register_client(&client).await.is_err() {
        return oauth_error(StatusCode::INTERNAL_SERVER_ERROR, "server_error");
    }
    json_response(json!(RegistrationResponse {
        client_id,
        client_name: client.client_name,
        redirect_uris: client.redirect_uris,
        token_endpoint_auth_method: "none",
    }))
}

pub async fn authorize(
    Extension(state): Extension<AppState>,
    Query(query): Query<AuthorizeQuery>,
    req: Request,
) -> Response {
    let Ok(client) = validate_authorize_request(&state, &query).await else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request");
    };
    let raw_session = cookie_value(&req, "ledgapi_session");
    let csrf = csrf_cookie_value(&req).unwrap_or_default();
    let Some(principal) =
        session_principal_for_cookie(&state, raw_session.as_deref()).await.ok().flatten()
    else {
        let next = format!(
            "/oauth/authorize?client_id={}&redirect_uri={}&response_type={}&code_challenge={}&code_challenge_method={}&scope={}&state={}",
            urlencoding::encode(&query.client_id),
            urlencoding::encode(&query.redirect_uri),
            urlencoding::encode(&query.response_type),
            urlencoding::encode(&query.code_challenge),
            urlencoding::encode(&query.code_challenge_method),
            urlencoding::encode(query.scope.as_deref().unwrap_or(READ_SCOPE)),
            urlencoding::encode(query.state.as_deref().unwrap_or("")),
        );
        return Redirect::to(&format!("/login?next={}", urlencoding::encode(&next)))
            .into_response();
    };
    consent_response(&query, &client, &principal, &csrf).await
}

pub async fn consent(Extension(state): Extension<AppState>, req: Request) -> Response {
    let raw_session = cookie_value(&req, "ledgapi_session");
    let csrf_cookie = csrf_cookie_value(&req);
    let Ok(bytes) = axum::body::to_bytes(req.into_body(), 32 * 1024).await else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request");
    };
    let Ok(form) = parse_form::<ConsentForm>(&bytes) else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request");
    };
    let Some(principal) =
        session_principal_for_cookie(&state, raw_session.as_deref()).await.ok().flatten()
    else {
        return Redirect::to("/login").into_response();
    };
    let Some(session_csrf) = csrf_cookie else {
        return oauth_error(StatusCode::FORBIDDEN, "csrf_failed");
    };
    if !token::constant_time_eq(&token::sha256_hex(&session_csrf), &token::sha256_hex(&form.csrf)) {
        return oauth_error(StatusCode::FORBIDDEN, "csrf_failed");
    }
    let query = AuthorizeQuery {
        client_id: form.client_id.clone(),
        redirect_uri: form.redirect_uri.clone(),
        response_type: "code".to_owned(),
        code_challenge: form.code_challenge.clone(),
        code_challenge_method: form.code_challenge_method.clone(),
        scope: Some(form.scope.clone()),
        state: (!form.state.is_empty()).then_some(form.state.clone()),
    };
    let Ok(client) = validate_authorize_request(&state, &query).await else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request");
    };
    if form.decision != "approve" {
        return oauth_redirect_error(&query, "access_denied");
    }
    let raw_code = token::generate();
    let code = AuthorizationCode {
        code_hash: token::sha256_hex(&raw_code),
        client_id: client.client_id,
        user_id: principal.user_id,
        redirect_uri: query.redirect_uri.clone(),
        scope: effective_scopes(&principal, query.scope.as_deref().unwrap_or(READ_SCOPE)),
        code_challenge: query.code_challenge,
        code_challenge_method: query.code_challenge_method,
        expires_at: OffsetDateTime::now_utc()
            + time::Duration::seconds(state.config().auth.authorization_code_ttl.as_secs() as i64),
    };
    if state.repos().oauth().create_authorization_code(&code).await.is_err() {
        return oauth_error(StatusCode::INTERNAL_SERVER_ERROR, "server_error");
    }
    let mut redirect = format!("{}?code={}", query.redirect_uri, urlencoding::encode(&raw_code));
    if let Some(state) = query.state {
        let _ = write!(redirect, "&state={}", urlencoding::encode(&state));
    }
    Redirect::to(&redirect).into_response()
}

pub async fn token_endpoint(Extension(state): Extension<AppState>, req: Request) -> Response {
    let Ok(bytes) = axum::body::to_bytes(req.into_body(), 32 * 1024).await else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request");
    };
    let Ok(form) = parse_form::<TokenForm>(&bytes) else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request");
    };
    let result = match form.grant_type.as_str() {
        "authorization_code" => exchange_code(&state, &form).await,
        "refresh_token" => exchange_refresh(&state, &form).await,
        _ => Err((StatusCode::BAD_REQUEST, "unsupported_grant_type")),
    };
    match result {
        Ok(response) => json_response(serde_json::to_value(response).unwrap_or_default()),
        Err((status, code)) => oauth_error(status, code),
    }
}

async fn exchange_code(
    state: &AppState,
    form: &TokenForm,
) -> Result<TokenResponse, (StatusCode, &'static str)> {
    let raw_code = form.code.as_deref().ok_or((StatusCode::BAD_REQUEST, "invalid_grant"))?;
    let verifier =
        form.code_verifier.as_deref().ok_or((StatusCode::BAD_REQUEST, "invalid_grant"))?;
    let code = state
        .repos()
        .oauth()
        .consume_authorization_code(&token::sha256_hex(raw_code), OffsetDateTime::now_utc())
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_grant"))?
        .ok_or((StatusCode::BAD_REQUEST, "invalid_grant"))?;
    if form.client_id.as_deref() != Some(code.client_id.as_str())
        || form.redirect_uri.as_deref() != Some(code.redirect_uri.as_str())
        || !verify_pkce(verifier, &code.code_challenge, &code.code_challenge_method)
    {
        return Err((StatusCode::BAD_REQUEST, "invalid_grant"));
    }
    issue_tokens(state, code.user_id, &code.client_id, code.scope).await
}

async fn exchange_refresh(
    state: &AppState,
    form: &TokenForm,
) -> Result<TokenResponse, (StatusCode, &'static str)> {
    let raw_refresh =
        form.refresh_token.as_deref().ok_or((StatusCode::BAD_REQUEST, "invalid_grant"))?;
    let refresh = state
        .repos()
        .oauth()
        .consume_refresh_token(&token::sha256_hex(raw_refresh), OffsetDateTime::now_utc())
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_grant"))?
        .ok_or((StatusCode::BAD_REQUEST, "invalid_grant"))?;
    if form.client_id.as_deref() != Some(refresh.client_id.as_str()) {
        return Err((StatusCode::BAD_REQUEST, "invalid_grant"));
    }
    issue_tokens(state, refresh.user_id, &refresh.client_id, refresh.scope).await
}

async fn issue_tokens(
    state: &AppState,
    user_id: crate::core::id::Id,
    client_id: &str,
    scope: Vec<String>,
) -> Result<TokenResponse, (StatusCode, &'static str)> {
    let Some(user) = state
        .repos()
        .users()
        .find_by_id(user_id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "server_error"))?
    else {
        return Err((StatusCode::BAD_REQUEST, "invalid_grant"));
    };
    if !user.active {
        return Err((StatusCode::BAD_REQUEST, "invalid_grant"));
    }
    let now = OffsetDateTime::now_utc();
    let raw_access = token::generate();
    let raw_refresh = token::generate();
    let access_expires =
        now + time::Duration::seconds(state.config().auth.access_token_ttl.as_secs() as i64);
    let refresh_expires =
        now + time::Duration::seconds(state.config().auth.refresh_token_ttl.as_secs() as i64);
    state
        .repos()
        .oauth()
        .create_access_token(&OAuthToken {
            token_hash: token::sha256_hex(&raw_access),
            client_id: client_id.to_owned(),
            user_id,
            scope: scope.clone(),
            expires_at: access_expires,
            revoked_at: None,
        })
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "server_error"))?;
    state
        .repos()
        .oauth()
        .create_refresh_token(&RefreshToken {
            token_hash: token::sha256_hex(&raw_refresh),
            client_id: client_id.to_owned(),
            user_id,
            scope: scope.clone(),
            expires_at: refresh_expires,
            revoked_at: None,
        })
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "server_error"))?;
    Ok(TokenResponse {
        access_token: raw_access,
        token_type: "Bearer",
        expires_in: state.config().auth.access_token_ttl.as_secs(),
        refresh_token: raw_refresh,
        scope: scope.join(" "),
    })
}

async fn validate_authorize_request(
    state: &AppState,
    query: &AuthorizeQuery,
) -> Result<OAuthClient, DomainError> {
    if query.response_type != "code"
        || query.code_challenge_method != "S256"
        || query.code_challenge.is_empty()
    {
        return Err(DomainError::OAuthInvalidRequest {
            message: "unsupported authorization request".to_owned(),
        });
    }
    let client = state
        .repos()
        .oauth()
        .find_client(&query.client_id)
        .await?
        .ok_or(DomainError::OAuthInvalidRequest { message: "unknown client".to_owned() })?;
    if !client.redirect_uris.iter().any(|uri| uri == &query.redirect_uri) {
        return Err(DomainError::OAuthInvalidRequest {
            message: "redirect uri is not registered".to_owned(),
        });
    }
    Ok(client)
}

async fn consent_response(
    query: &AuthorizeQuery,
    client: &OAuthClient,
    principal: &Principal,
    csrf: &str,
) -> Response {
    let requested = query.scope.as_deref().unwrap_or(READ_SCOPE);
    let scopes = effective_scopes(principal, requested);
    let tpl = OAuthConsentTpl {
        client_name: &client.client_name,
        username: &principal.username,
        client_id: &client.client_id,
        redirect_uri: &query.redirect_uri,
        code_challenge: &query.code_challenge,
        code_challenge_method: &query.code_challenge_method,
        scope: &scopes.join(" "),
        state: query.state.as_deref().unwrap_or(""),
        csrf,
    };
    let mut response = Html(tpl.render().unwrap_or_default()).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store"));
    response
}

fn effective_scopes(principal: &Principal, requested: &str) -> Vec<String> {
    requested
        .split_whitespace()
        .filter(|scope| match *scope {
            READ_SCOPE => true,
            WRITE_SCOPE => principal.role.can_write(),
            ADMIN_SCOPE => principal.role.can_manage_users(),
            _ => false,
        })
        .map(str::to_owned)
        .collect()
}

fn verify_pkce(verifier: &str, challenge: &str, method: &str) -> bool {
    if method != "S256" {
        return false;
    }
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest) == challenge
}

fn valid_redirect_uri(uri: &str) -> bool {
    uri.starts_with("https://")
        || uri.starts_with("http://localhost")
        || uri.starts_with("http://127.0.0.1")
}

fn oauth_redirect_error(query: &AuthorizeQuery, error: &str) -> Response {
    let mut redirect = format!("{}?error={}", query.redirect_uri, urlencoding::encode(error));
    if let Some(state) = &query.state {
        let _ = write!(redirect, "&state={}", urlencoding::encode(state));
    }
    Redirect::to(&redirect).into_response()
}

fn json_response(value: serde_json::Value) -> Response {
    let mut response = Json(value).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store"));
    response
}

fn oauth_error(status: StatusCode, error: &str) -> Response {
    let mut response = (status, Json(json!({"error": error}))).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store"));
    response
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
