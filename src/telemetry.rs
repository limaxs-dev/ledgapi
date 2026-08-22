//! Tracing initialization. Called once at the top of [`crate::run`].

use crate::config::{LogConfig, LogFormat};
use anyhow::Context;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Initialize the global tracing subscriber. Idempotent — calling twice
/// is a no-op (the second init is logged and ignored).
///
/// # Errors
/// Returns `Err` if the subscriber fails to build (extremely rare).
pub fn init(cfg: &LogConfig) -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(cfg.level.clone()));

    let registry = tracing_subscriber::registry().with(filter);

    match cfg.format {
        LogFormat::Pretty => {
            registry
                .with(fmt::layer().with_target(true).with_writer(std::io::stderr))
                .try_init()
                .context("init tracing (pretty)")?;
        }
        LogFormat::Json => {
            registry
                .with(
                    fmt::layer()
                        .json()
                        .with_current_span(true)
                        .with_writer(std::io::stderr),
                )
                .try_init()
                .context("init tracing (json)")?;
        }
    }

    Ok(())
}
