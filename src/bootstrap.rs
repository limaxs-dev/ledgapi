//! Composition root placeholder. Real implementation arrives in Task 40.

use crate::config::AppConfig;
use crate::telemetry;

/// Bootstrap entry point — currently initializes telemetry only. The
/// full composition (DB pool, embedder, AppState, axum router) is added
/// in Task 40.
pub async fn run() -> anyhow::Result<()> {
    let cfg = AppConfig::from_env()?;
    telemetry::init(&cfg.log)?;
    tracing::info!(
        bind = %cfg.server.bind,
        db = %cfg.database.path,
        "ledgapi bootstrap started"
    );
    // Server bind/serve is wired in Task 40.
    Ok(())
}
