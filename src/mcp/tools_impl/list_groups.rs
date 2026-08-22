//! `list_groups` — thin delegate over `manage_group::list`.

use crate::domain::errors::DomainError;
use crate::domain::project::ProjectSlug;
use crate::mcp::tools::{Tool, ToolContext};
use async_trait::async_trait;
use schemars::{JsonSchema, SchemaGenerator, schema::Schema};
use serde::Deserialize;
use serde_json::{Value, json};

/// `list_groups` MCP tool.
pub struct ListGroupsTool;

/// JSON-Schema input for `list_groups`.
#[derive(Deserialize, JsonSchema)]
pub struct Input {
    pub project_slug: String,
}

#[async_trait]
impl Tool for ListGroupsTool {
    fn name(&self) -> &'static str {
        "list_groups"
    }

    fn description(&self) -> &'static str {
        "List all groups in a project with contract counts."
    }

    fn input_schema(&self) -> Schema {
        let mut generator = SchemaGenerator::default();
        Input::json_schema(&mut generator)
    }

    async fn execute(&self, ctx: ToolContext, input: Value) -> Result<Value, DomainError> {
        let p: Input = serde_json::from_value(input).map_err(|e| DomainError::Validation {
            field: "args".into(),
            message: e.to_string(),
        })?;
        let slug = ProjectSlug::parse(&p.project_slug)?;
        // Re-resolve via slug: `ctx.project_id` is a nil sentinel for
        // tools that don't take a project_slug.
        let project = ctx
            .state
            .repos()
            .projects()
            .find_by_slug(&slug)
            .await?
            .ok_or(DomainError::NotFound { resource: "project" })?;
        let list =
            crate::domain::use_cases::manage_group::list(ctx.state.repos(), project.id).await?;
        let arr: Vec<Value> = list
            .iter()
            .map(|g| {
                json!({
                    "id": g.id.to_string(),
                    "name": g.name,
                    "contract_count": g.contract_count,
                })
            })
            .collect();
        Ok(json!({ "groups": arr }))
    }
}