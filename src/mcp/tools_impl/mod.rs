//! Concrete tool implementations. One file per tool in submodules.
//!
//! Task 33 ships the 5 read-only tools. Task 34 adds the 5 write/search
//! tools.

pub mod create_project;
pub mod get_contract_by_id;
pub mod list_contracts;
pub mod list_groups;
pub mod list_projects;

use crate::mcp::tools::Tool;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry of all tools advertised to MCP clients.
///
/// Pre-populated by [`McpRegistry::new`] with every tool that has been
/// implemented so far. Callers list every tool for `tools/list` and
/// resolve a tool by name for `tools/call`.
#[derive(Default)]
pub struct McpRegistry {
    tools: HashMap<&'static str, Arc<dyn Tool>>,
}

impl McpRegistry {
    /// Construct a registry pre-populated with every v1 read-only tool.
    /// Task 34 extends this with the 5 write/search tools.
    #[must_use]
    pub fn new() -> Self {
        let mut r = Self::default();
        r.register(Arc::new(create_project::CreateProjectTool));
        r.register(Arc::new(list_projects::ListProjectsTool));
        r.register(Arc::new(get_contract_by_id::GetContractByIdTool));
        r.register(Arc::new(list_groups::ListGroupsTool));
        r.register(Arc::new(list_contracts::ListContractsTool));
        r
    }

    /// Register a tool by its [`Tool::name`].
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_registers_five_read_only_tools() {
        let r = McpRegistry::new();
        assert_eq!(r.list().len(), 5);
    }

    #[test]
    fn list_is_sorted_alphabetically() {
        let r = McpRegistry::new();
        let names: Vec<&'static str> = r.list().iter().map(|t| t.name()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
    }

    #[test]
    fn get_returns_tool_by_name() {
        let r = McpRegistry::new();
        assert!(r.get("create_project").is_some());
        assert!(r.get("list_projects").is_some());
        assert!(r.get("nonexistent").is_none());
    }
}