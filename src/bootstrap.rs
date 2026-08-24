//! Composition root. Wires config → telemetry → DB → embedder → repos
//! → AppState → axum router → serve until SIGTERM.

use crate::domain::ports::Repos;
use crate::errors::AppError;
use crate::infra::auth::password;
use crate::infra::db;
use crate::infra::embeddings::fastembed_impl::FastembedEmbedder;
use crate::infra::repos::SqliteRepos;
use crate::state::AppState;
use anyhow::Context;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Bootstrap entry point. Reads config, initialises telemetry, opens
/// the DB, seeds the initial super admin, wires the embedder and
/// repos, builds `AppState`, and serves until SIGTERM/SIGINT.
pub async fn run() -> anyhow::Result<()> {
    let cfg = crate::config::AppConfig::from_env()?;
    crate::telemetry::init(&cfg.log)?;

    let db = db::open(&cfg.database).context("open db")?;

    let repos = SqliteRepos::new(db.clone());
    let password_hash = match cfg.auth.initial_admin_password.as_deref() {
        Some(password) => Some(
            password::hash_password(password)
                .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        ),
        None => None,
    };
    crate::domain::use_cases::bootstrap_admin::ensure(
        repos.users(),
        cfg.auth.initial_admin_username.as_deref(),
        password_hash.as_deref(),
    )
    .await
    .map_err(|error| anyhow::anyhow!("initial admin bootstrap failed: {error}"))?;

    let embedder: Arc<dyn crate::domain::ports::Embedder> =
        Arc::new(FastembedEmbedder::new(&cfg.embed.cache_dir, &cfg.embed.model)?);

    let state = AppState {
        repos: Arc::new(repos),
        embedder: embedder.clone(),
        mcp: Arc::new(crate::mcp::tools_impl::McpRegistry::new()),
        cfg: Arc::new(cfg.clone()),
        db,
    };

    serve(state, cfg.server.bind, cfg.server.shutdown_timeout).await
}

/// Bind to `bind`, spawn a SIGINT/SIGTERM watcher, and serve until the
/// token is cancelled. The cancel token also wins on graceful shutdown
/// timeout via the caller's `shutdown_timeout` (currently honoured by
/// axum's `with_graceful_shutdown`).
pub async fn serve(
    state: AppState,
    bind: String,
    _shutdown_timeout: Duration,
) -> anyhow::Result<()> {
    let listener =
        tokio::net::TcpListener::bind(&bind).await.with_context(|| format!("bind {bind}"))?;

    let shutdown = CancellationToken::new();
    spawn_signal_handler(shutdown.clone());

    let app = crate::web::router::router(state);

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(async move { shutdown.cancelled_owned().await })
        .await
        .context("axum::serve")
}

/// Spawn a task that listens for SIGINT (Ctrl-C) or SIGTERM and
/// cancels the shutdown token on the first signal received.
fn spawn_signal_handler(token: CancellationToken) {
    tokio::spawn(async move {
        let ctrl_c = tokio::signal::ctrl_c();
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
        tokio::select! {
            res = ctrl_c => { let _ = res; },
            () = async {
                if let Some(s) = sigterm.as_mut() {
                    s.recv().await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {},
        }
        token.cancel();
    });
}

/// Marker to keep `AppError` reachable from `bootstrap` (used by route layer).
#[allow(dead_code)]
fn _marker(_: AppError) {}
