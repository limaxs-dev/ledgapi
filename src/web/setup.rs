//! `/setup` page — first-run token bootstrap. After a token is issued,
//! the page returns 410 Gone until either (a) the first valid MCP call
//! or (b) the 5-minute TTL elapses, whichever comes first (lazy check).

use crate::state::AppState;
use crate::web::templates::SetupTpl;
use askama::Template;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::sync::atomic::Ordering;
use std::time::Instant;

/// Render the bootstrap page if setup is still active, else 410 Gone.
pub async fn show(Extension(state): Extension<AppState>) -> Response {
    // Per spec §6.3: 410 once the first valid MCP call has consumed
    // setup — even if the 5-minute TTL has not yet elapsed. The auth
    // middleware flips `setup_active` to false; we check it here so the
    // plaintext token doesn't sit in browser back-button history for
    // up to 5 minutes after first use.
    if !state.setup_active.load(Ordering::Acquire) {
        return (StatusCode::GONE, "setup already completed").into_response();
    }
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
    let tpl = SetupTpl { title: "Setup · ledgapi", token };
    (StatusCode::OK, tpl.render().unwrap_or_default()).into_response()
}
