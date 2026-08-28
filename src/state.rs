//! `AppState` — every dependency held by the composition root and
//! cloned (via `Arc`) into every handler.

use crate::config::{AppConfig, EmbedConfig};
use crate::domain::ports::{Embedder, Repos};
use crate::infra::db::Db;
use crate::infra::repos::SqliteRepos;
use crate::mcp::tools_impl::McpRegistry;
use std::sync::Arc;
use std::time::Duration;

/// Shared application state. Every handler receives a clone.
///
/// All fields are public so `bootstrap::run` (and tests) can construct
/// the struct directly via a literal. Handlers always go through the
/// accessor methods.
#[derive(Clone)]
pub struct AppState {
    pub repos: Arc<SqliteRepos>,
    pub embedder: Arc<dyn Embedder>,
    pub mcp: Arc<McpRegistry>,
    pub cfg: Arc<AppConfig>,
    pub db: Db,
}

impl AppState {
    /// Borrow the repos bundle as a trait object.
    #[must_use]
    pub fn repos(&self) -> &dyn Repos {
        self.repos.as_ref()
    }

    /// Borrow the concrete [`SqliteRepos`] handle. Used by health
    /// probes that need raw access to the underlying connection.
    #[must_use]
    pub fn sqlite_repos(&self) -> &SqliteRepos {
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

    /// Build a state suitable for tests with `StubEmbedder` and an
    /// in-memory DB. `plaintext_token` is the token returned by
    /// bootstrap (for setup-page tests).
    #[must_use]
    pub fn for_tests(repos: SqliteRepos, embedder: Arc<dyn Embedder>) -> Self {
        let cfg = Arc::new(AppConfig {
            server: crate::config::ServerConfig {
                bind: "127.0.0.1:0".to_owned(),
                shutdown_timeout: Duration::from_secs(1),
            },
            database: crate::config::DatabaseConfig {
                path: ":memory:".to_owned(),
                busy_timeout_ms: 1000,
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
                level: "warn".to_owned(),
            },
            auth: crate::config::AuthConfig {
                initial_admin_username: None,
                initial_admin_password: None,
                issuer: "http://localhost:18080".to_owned(),
                session_ttl: Duration::from_hours(1),
                access_token_ttl: Duration::from_hours(1),
                refresh_token_ttl: Duration::from_hours(24),
                authorization_code_ttl: Duration::from_mins(1),
                cookie_secure: false,
            },
        });
        let db = repos.db.clone();
        Self { repos: Arc::new(repos), embedder, mcp: Arc::new(McpRegistry::new()), cfg, db }
    }

    /// Build an [`AppState`] wired to a fresh in-memory database and a
    /// [`StubEmbedder`](crate::infra::embeddings::fastembed_impl::StubEmbedder).
    /// Used by `src/web/` tests that need a fully wired state without
    /// external I/O. New code should prefer
    /// [`AppState::for_tests`](Self::for_tests) with explicit repos.
    #[must_use]
    pub fn for_tests_default() -> Self {
        use crate::infra::db::pool::open_memory;
        use crate::infra::embeddings::fastembed_impl::StubEmbedder;

        let db = open_memory().expect("open in-memory db");
        let repos = SqliteRepos::new(db);
        Self::for_tests(repos, Arc::new(StubEmbedder::new()))
    }
}
