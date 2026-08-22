//! `ledgapi` binary entry point.

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ledgapi::run().await.context("ledgapi::run")
}
