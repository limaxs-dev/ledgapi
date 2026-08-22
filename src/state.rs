//! `AppState` — placeholder until Task 40 (Composition).
//!
//! The real `AppState` is constructed in Task 40. This stub exists so
//! that code that depends on `crate::state::AppState` (e.g. the auth
//! middleware and the MCP dispatcher) compiles in isolation. Task 32
//! adds the `mcp_registry()` accessor; Task 34 adds the embedder and
//! embed-config accessors used by the write/search MCP tools.

use crate::config::{AppConfig, EmbedConfig};
use crate::domain::ports::{Embedder, Repos};
use crate::infra::repos::SqliteRepos;
use crate::mcp::tools_impl::McpRegistry;
use std::sync::Arc;

/// Shared application state. Cloned (via `Arc`s) into every handler.
#[derive(Clone)]
pub struct AppState {
    repos: Arc<SqliteRepos>,
    mcp: Arc<McpRegistry>,
    cfg: Arc<AppConfig>,
    embedder: Arc<dyn Embedder>,
}

impl AppState {
    /// Borrow the repos bundle.
    #[must_use]
    pub fn repos(&self) -> &dyn Repos {
        self.repos.as_ref()
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
    pub fn mark_setup_consumed(&self) {}
}