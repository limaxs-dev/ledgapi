//! `ledgapi` — API contracts, remembered by your agents.

#![doc = include_str!("../README.md")]
#![deny(clippy::correctness)]

pub mod bootstrap;
pub mod core;
pub mod domain;

/// Library entry point used by both the binary and integration tests.
///
/// The composition root lives here so that integration tests can call
/// [`run`] with a custom [`AppState`] without spawning a subprocess.
pub async fn run() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    bootstrap::run().await
}
