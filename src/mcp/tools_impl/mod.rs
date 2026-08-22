//! Concrete tool implementations. One file per tool in submodules.
//!
//! Task 32 ships only the registry skeleton — the actual tool structs
//! land in Task 33 (read-only tools) and Task 34 (write/search tools).

use crate::mcp::tools::Tool;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry of all tools advertised to MCP clients.
///
/// Pre-populated by [`McpRegistry::new`] once the individual tool
/// modules are wired in (Tasks 33 & 34). The skeleton here lets the
/// dispatcher compile before those land.
#[derive(Default)]
pub struct McpRegistry {
    tools: HashMap<&'static str, Arc<dyn Tool>>,
}

impl McpRegistry {
    /// Construct an empty registry. Task 33/34 expand this to register
    /// every v1 tool.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool by its [`Tool::name`]. Used by Task 33 to wire
    /// up the read-only tools and Task 34 to add the write/search tools.
    #[allow(dead_code)]
    fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name(), tool);
    }

    /// List every registered tool (for `tools/list`).
    #[must_use]
    pub fn list(&self) -> Vec<&Arc<dyn Tool>> {
        let mut v: Vec<&Arc<dyn Tool>> = self.tools.values().collect();
        v.sort_by_key(|t| t.name());
        v
    }

    /// Look up a tool by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }
}