//! `get_contract_by_id` — thin delegate over `create_contract::get`.

use crate::domain::errors::DomainError;
use crate::domain::project::ProjectSlug;
use crate::mcp::tools::{Tool, ToolContext};
use async_trait::async_trait;
use schemars::{JsonSchema, SchemaGenerator, schema::Schema};
use serde::Deserialize;
use serde_json::Value;

/// `get_contract_by_id` MCP tool.
pub struct GetContractByIdTool;

/// JSON-Schema input for `get_contract_by_id`.
#[derive(Deserialize, JsonSchema)]
pub struct Input {
    pub project_slug: String,
    /// Contract id (UUIDv7).
    pub contract_id: String,
}

#[async_trait]
impl Tool for GetContractByIdTool {
    fn name(&self) -> &'static str {
        "get_contract_by_id"
    }

    fn description(&self) -> &'static str {
        "Fetch a single contract by id (UUIDv7)."
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
        let c = crate::domain::use_cases::create_contract::get(ctx.state.repos(), slug, id).await?;
        serde_json::to_value(&c).map_err(|e| DomainError::Internal(e.to_string()))
    }
}