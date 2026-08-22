//! `AppState` — placeholder until Task 40 (Composition).
//!
//! The real `AppState` is constructed in Task 40. This stub exists so
//! that code that depends on `crate::state::AppState` (e.g. the auth
//! middleware and the MCP dispatcher) compiles in isolation. Task 32
//! adds the `mcp_registry()` accessor; Task 34 adds the embedder and
//! embed-config accessors used by the write/search MCP tools.
//!
//! Task 37 adds the `setup`/`bootstrap_token_plaintext` accessors that
//! the `/setup` page uses. Task 40 finalises the struct shape.

use crate::config::{AppConfig, EmbedConfig};
use crate::domain::ports::{Embedder, Repos};
use crate::infra::repos::SqliteRepos;
use crate::mcp::tools_impl::McpRegistry;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// First-run bootstrap state. Cleared by [`AppState::mark_setup_consumed`]
/// once the operator finishes setup (either by calling `/mcp` with the
/// token or by the TTL elapsing).
#[derive(Debug)]
pub struct SetupState {
    /// True while setup is still pending.
    pub active: bool,
    /// Wall-clock instant when the bootstrap window expires.
    pub expires_at: Instant,
    /// Plaintext token; held only while `active` so the `/setup` page
    /// can render it before the first MCP call.
    pub plaintext: Option<String>,
}

/// Shared application state. Cloned (via `Arc`s) into every handler.
#[derive(Clone)]
pub struct AppState {
    repos: Arc<SqliteRepos>,
    mcp: Arc<McpRegistry>,
    cfg: Arc<AppConfig>,
    embedder: Arc<dyn Embedder>,
    /// Atomic "is the setup window still open?" toggle. Flipped by the
    /// auth middleware on the first valid bearer token, and by the
    /// `/setup` handler when its TTL elapses.
    setup_active: Arc<AtomicBool>,
    setup_state: Arc<SetupState>,
}

impl AppState {
    /// Borrow the repos bundle.
    #[must_use]
    pub fn repos(&self) -> &dyn Repos {
        self.repos.as_ref()
    }

    /// Borrow the concrete [`SqliteRepos`] handle. Used by health
    /// probes that need raw access to the underlying connection.
    #[must_use]
    pub fn sqlite_repos(&self) -> &crate::infra::repos::SqliteRepos {
        &self.repos
    }

    /// Borrow the MCP tool registry.
    #[must_use]
    pub fn mcp_registry(&self) -> &McpRegistry {
        self.mcp.as_ref()
    }

    /// Borrow the full application configuration.
    #[must_use]
    pub fn config(&self) -> &AppConfig {
        self.cfg.as_ref()
    }

    /// Borrow the embed config (for use cases that need threshold/K).
    #[must_use]
    pub fn embed_cfg(&self) -> &EmbedConfig {
        &self.cfg.embed
    }

    /// Clone the embedder handle for use cases that need to compute
    /// embeddings off the use-case thread.
    #[must_use]
    pub fn embedder(&self) -> Arc<dyn Embedder> {
        self.embedder.clone()
    }

    /// Mark the setup page as consumed (first valid MCP call).
    pub fn mark_setup_consumed(&self) {
        self.setup_active.store(false, Ordering::Relaxed);
    }

    /// Borrow the first-run setup state.
    #[must_use]
    pub fn setup(&self) -> &SetupState {
        &self.setup_state
    }

    /// Borrow the plaintext bootstrap token if still active, else `None`.
    #[must_use]
    pub fn bootstrap_token_plaintext(&self) -> Option<&str> {
        self.setup_state.plaintext.as_deref()
    }

    /// Build an [`AppState`] wired to a fresh in-memory database and a
    /// [`StubEmbedder`](crate::infra::embeddings::fastembed_impl::StubEmbedder).
    /// Used exclusively by integration tests under `src/web/`. Production
    /// construction happens in `bootstrap::run` (Task 40).
    #[must_use]
    pub fn for_tests() -> Self {
        use crate::infra::db::pool::open_memory;
        use crate::infra::embeddings::fastembed_impl::StubEmbedder;

        let cfg = Arc::new(crate::config::AppConfig {
            server: crate::config::ServerConfig {
                bind: "127.0.0.1:0".to_owned(),
                shutdown_timeout: std::time::Duration::from_secs(30),
            },
            database: crate::config::DatabaseConfig {
                path: ":memory:".to_owned(),
                busy_timeout_ms: 5000,
            },
            embed: crate::config::EmbedConfig {
                cache_dir: String::new(),
                model: String::new(),
                similarity_threshold: 0.85,
                knn_top_k: 5,
                hybrid_limit: 10,
            },
            log: crate::config::LogConfig {
                format: crate::config::LogFormat::Pretty,
                level: "info".to_owned(),
            },
        });
        Self {
            repos: Arc::new(SqliteRepos::new(open_memory().expect("open in-memory db"))),
            mcp: Arc::new(McpRegistry::new()),
            cfg,
            embedder: Arc::new(StubEmbedder::new()),
            setup_active: Arc::new(AtomicBool::new(false)),
            setup_state: Arc::new(SetupState {
                active: false,
                expires_at: Instant::now(),
                plaintext: None,
            }),
        }
    }
}
