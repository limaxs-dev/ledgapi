//! Web router — mounts all unauthenticated routes.
//!
//! The MCP route is mounted here with the bearer-auth middleware so that
//! the public read-only routes (`/`, `/projects/{slug}`, `/search`,
//! `/openapi.yml`, `/setup`, `/healthz`, `/readyz`) stay open while the
//! MCP endpoint requires a token.
//!
//! The router returns `Router<()>` so that it composes with
//! `axum::serve(listener, app.into_make_service())`. Handlers extract
//! shared state via `Extension<AppState>` (inserted by the layer below),
//! not `State<AppState>`. The bearer-auth middleware also receives the
//! state via the same layer.

use crate::infra::auth::middleware::bearer_auth;
use crate::mcp::server::handle as mcp_handle;
use crate::state::AppState;
use crate::web::handlers;
use crate::web::health;
use crate::web::openapi_export;
use axum::Extension;
use axum::Router;
use axum::middleware::from_fn;
use axum::response::IntoResponse;
use axum::routing::{get, post};

/// Build the web router. `state` is cloned into request extensions and
/// into the bearer-auth middleware so that every handler (including the
/// MCP dispatcher) can pull `AppState` via the `Extension` extractor.
///
/// Returns `Router<()>` so it composes with `axum::serve` directly.
#[allow(clippy::needless_pass_by_value)]
pub fn router(state: AppState) -> Router {
    let state_for_bearer = state.clone();
    let state_for_web_auth = state.clone();
    let protected = Router::new()
        .route("/", get(handlers::dashboard))
        .route("/projects/{slug}", get(handlers::project))
        .route("/projects/{slug}/contracts/{id}", get(handlers::contract))
        .route("/projects/{slug}/search", get(handlers::search))
        .route("/projects/{slug}/openapi.yml", get(openapi_export::yaml))
        .route("/admin/users", get(crate::web::admin::users).post(crate::web::admin::create_user))
        .route("/admin/audit", get(crate::web::admin::audit))
        .route("/docs", get(crate::web::docs::home))
        .route("/docs/{*rest}", get(crate::web::docs::page))
        .layer(from_fn(move |req, next| {
            crate::web::auth::require_web_auth(req, next, state_for_web_auth.clone())
        }));

    Router::new()
        .merge(protected)
        .route("/login", get(crate::web::auth::show_login).post(crate::web::auth::login))
        .route("/logout", post(crate::web::auth::logout))
        .route(
            "/.well-known/oauth-protected-resource",
            get(crate::web::oauth::protected_resource_metadata),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(crate::web::oauth::authorization_server_metadata),
        )
        .route("/oauth/register", post(crate::web::oauth::register))
        .route("/oauth/authorize", get(crate::web::oauth::authorize))
        .route("/oauth/consent", post(crate::web::oauth::consent))
        .route("/oauth/token", post(crate::web::oauth::token_endpoint))
        .route("/healthz", get(health::live))
        .route("/readyz", get(health::ready))
        .route("/static/style.css", get(serve_css))
        .route("/static/logo.svg", get(serve_logo))
        .fallback(handlers::not_found)
        .route(
            "/mcp",
            post(mcp_handle)
                .layer(from_fn(move |req, next| bearer_auth(req, next, state_for_bearer.clone()))),
        )
        .layer(Extension(state))
}

/// Serve the embedded CSS as `text/css`.
async fn serve_css() -> impl IntoResponse {
    let css = include_str!("../../templates/style.css");
    ([("content-type", "text/css")], css.to_owned())
}

/// Serve the embedded brand logo as `image/svg+xml`.
async fn serve_logo() -> impl IntoResponse {
    let svg = include_str!("../../logo.svg");
    ([("content-type", "image/svg+xml")], svg.to_owned())
}
