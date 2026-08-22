//! Web router — mounts all unauthenticated routes.
//!
//! The MCP route is mounted here with the bearer-auth middleware so that
//! the public read-only routes (`/`, `/projects/{slug}`, `/search`,
//! `/openapi.yml`, `/setup`, `/healthz`, `/readyz`) stay open while the
//! MCP endpoint requires a token. Task 40 re-wires the final composition.

use crate::infra::auth::middleware::bearer_auth;
use crate::mcp::server::handle as mcp_handle;
use crate::state::AppState;
use crate::web::handlers;
use crate::web::health;
use crate::web::openapi_export;
use crate::web::setup;
use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::response::IntoResponse;
use axum::routing::{get, post};

/// Build the web router. The `AppState` is cloned into the bearer-auth
/// middleware; it is then bound to the router via `with_state`.
#[allow(clippy::needless_pass_by_value)]
pub fn router(state: AppState) -> Router<AppState> {
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
            post(mcp_handle).layer(from_fn_with_state(state.clone(), bearer_auth)),
        )
        .with_state(state)
}

/// Serve the embedded CSS as `text/css`.
async fn serve_css() -> impl IntoResponse {
    let css = include_str!("../../templates/style.css");
    ([("content-type", "text/css")], css.to_owned())
}
