//! `ledgapi` — API contracts, remembered by your agents.

#![doc = include_str!("../README.md")]
#![deny(clippy::correctness)]

pub mod bootstrap;
pub mod config;
pub mod core;
pub mod domain;
pub mod errors;
pub mod infra;
pub mod state;
pub mod telemetry;

/// Library entry point used by both the binary and integration tests.
///
/// The composition root lives here so that integration tests can call
/// [`run`] with a custom [`AppState`] without spawning a subprocess.
pub async fn run() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    bootstrap::run().await
}
