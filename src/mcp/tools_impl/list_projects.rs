//! `list_projects` — thin delegate over `manage_project::list`.

use crate::domain::errors::DomainError;
use crate::mcp::tools::{Tool, ToolContext};
use async_trait::async_trait;
use schemars::{JsonSchema, SchemaGenerator, schema::Schema};
use serde::Deserialize;
use serde_json::{Value, json};

/// `list_projects` MCP tool.
pub struct ListProjectsTool;

/// JSON-Schema input for `list_projects`.
#[derive(Deserialize, JsonSchema)]
pub struct Input {}

#[async_trait]
impl Tool for ListProjectsTool {
    fn name(&self) -> &'static str {
        "list_projects"
    }

    fn description(&self) -> &'static str {
        "List all projects with contract counts."
    }

    fn input_schema(&self) -> Schema {
        let mut generator = SchemaGenerator::default();
        Input::json_schema(&mut generator)
    }

    async fn execute(&self, ctx: ToolContext, _input: Value) -> Result<Value, DomainError> {
        let list = crate::domain::use_cases::manage_project::list(ctx.state.repos()).await?;
        let arr: Vec<Value> = list
            .iter()
            .map(|p| {
                json!({
                    "slug": p.slug.as_str(),
                    "name": p.name,
                    "contract_count": p.contract_count,
                })
            })
            .collect();
        Ok(json!({ "projects": arr }))
    }
}