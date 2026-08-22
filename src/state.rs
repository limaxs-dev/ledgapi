//! `AppState` — placeholder until Task 40 (Composition).
//!
//! The real `AppState` is constructed in Task 40. This stub exists so
//! that code that depends on `crate::state::AppState` (e.g. the auth
//! middleware and the MCP dispatcher) compiles in isolation. Task 32
//! adds the `mcp_registry()` accessor; Task 33/34 will eventually
//! extend it as more tools need accessors.

use crate::domain::ports::Repos;
use crate::infra::repos::SqliteRepos;
use crate::mcp::tools_impl::McpRegistry;
use std::sync::Arc;

/// Shared application state. Cloned (via `Arc`s) into every handler.
#[derive(Clone)]
pub struct AppState {
    repos: Arc<SqliteRepos>,
    mcp: Arc<McpRegistry>,
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

    /// Mark the setup page as consumed (first valid MCP call).
    pub fn mark_setup_consumed(&self) {}
}