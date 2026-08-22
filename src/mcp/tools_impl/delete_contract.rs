//! `delete_contract` — thin delegate over `delete_contract::execute`.

use crate::domain::errors::DomainError;
use crate::domain::project::ProjectSlug;
use crate::mcp::tools::{Tool, ToolContext};
use async_trait::async_trait;
use schemars::{JsonSchema, SchemaGenerator, schema::Schema};
use serde::Deserialize;
use serde_json::{Value, json};

/// `delete_contract` MCP tool.
pub struct DeleteContractTool;

/// JSON-Schema input for `delete_contract`.
#[derive(Deserialize, JsonSchema)]
pub struct Input {
    pub project_slug: String,
    /// Contract id (UUIDv7).
    pub contract_id: String,
}

#[async_trait]
impl Tool for DeleteContractTool {
    fn name(&self) -> &'static str {
        "delete_contract"
    }

    fn description(&self) -> &'static str {
        "Delete a contract by id."
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
        let id = crate::core::id::Id::parse(&p.contract_id)
            .ok_or(DomainError::NotFound { resource: "contract" })?;
        crate::domain::use_cases::delete_contract::execute(ctx.state.repos(), slug, id).await?;
        Ok(json!({ "status": "deleted" }))
    }
}