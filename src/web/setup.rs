//! `/setup` page — first-run token bootstrap. After a token is issued,
//! the page returns 410 Gone until either (a) the first valid MCP call
//! or (b) the 5-minute TTL elapses, whichever comes first (lazy check).

use crate::state::AppState;
use crate::web::templates::SetupTpl;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::time::Instant;

/// Render the bootstrap page if setup is still active, else 410 Gone.
pub async fn show(State(state): State<AppState>) -> Response {
    let setup = state.setup();
    if !setup.active {
        return (StatusCode::GONE, "setup already completed").into_response();
    }
    if Instant::now() >= setup.expires_at {
        state.mark_setup_consumed();
        return (StatusCode::GONE, "setup window expired").into_response();
    }
    // The plaintext token was logged at boot. We cannot re-display it
    // without storing it; per spec §6.3 we only show it ONCE. For the
    // page to be useful, we keep a copy in memory while setup is active.
    let token = state.bootstrap_token_plaintext().unwrap_or_default();
    let tpl = SetupTpl {
        title: "Setup · ledgapi",
        token,
    };
    (StatusCode::OK, tpl.render().unwrap_or_default()).into_response()
}
