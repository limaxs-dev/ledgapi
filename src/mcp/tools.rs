//! Tool trait + per-tool context.

use crate::domain::errors::DomainError;
use async_trait::async_trait;
use schemars::schema::Schema;
use serde_json::Value;

/// Context passed to every tool. Bundles the resolved project_slug and
/// the resolved project_id (so tools don't need to look it up again).
#[derive(Clone)]
pub struct ToolContext {
    /// Project slug from the tool's `arguments` (or empty string for
    /// tools that don't take one).
    pub project_slug: String,
    /// Resolved project id. Use [`Id::nil()`](crate::core::id::Id::nil)
    /// for tools that don't need a project.
    pub project_id: crate::core::id::Id,
    /// Shared application state (repos, embedder, config).
    pub state: std::sync::Arc<crate::state::AppState>,
}

/// A single MCP tool. Tools are stateless — they take input JSON, return
/// output JSON. Project lookup is done by the dispatcher.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name as advertised to clients.
    fn name(&self) -> &'static str;

    /// One-line description.
    fn description(&self) -> &'static str;

    /// JSON Schema for the tool input. Generated from `schemars`.
    fn input_schema(&self) -> Schema;

    /// Execute the tool. Errors returned here map to JSON-RPC error
    /// frames unless `is_mcp_error()` returns false (e.g., `SimilarFound`).
    async fn execute(&self, ctx: ToolContext, input: Value) -> Result<Value, DomainError>;
}
