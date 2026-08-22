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
use crate::web::setup;
use axum::Router;
use axum::Extension;
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
    Router::new()
        .route("/", get(handlers::dashboard))
        .route("/projects/{slug}", get(handlers::project))
        .route(
            "/projects/{slug}/contracts/{id}",
            get(handlers::contract),
        )
        .route("/projects/{slug}/search", get(handlers::search))
        .route("/projects/{slug}/openapi.yml", get(openapi_export::yaml))
        .route("/setup", get(setup::show))
        .route("/healthz", get(health::live))
        .route("/readyz", get(health::ready))
        .route("/static/style.css", get(serve_css))
        .fallback(handlers::not_found)
        .route(
            "/mcp",
            post(mcp_handle)
                .layer(from_fn(move |req, next| {
                    bearer_auth(req, next, state_for_bearer.clone())
                })),
        )
        .layer(Extension(state))
}

/// Serve the embedded CSS as `text/css`.
async fn serve_css() -> impl IntoResponse {
    let css = include_str!("../../templates/style.css");
    ([("content-type", "text/css")], css.to_owned())
}
