//! `export_openapi` — thin delegate over `export_openapi::execute`.

use crate::domain::errors::DomainError;
use crate::domain::project::ProjectSlug;
use crate::mcp::tools::{Tool, ToolContext};
use async_trait::async_trait;
use schemars::{JsonSchema, SchemaGenerator, schema::Schema};
use serde::Deserialize;
use serde_json::{Value, json};

/// `export_openapi` MCP tool.
pub struct ExportOpenApiTool;

/// JSON-Schema input for `export_openapi`.
#[derive(Deserialize, JsonSchema)]
pub struct Input {
    pub project_slug: String,
}

#[async_trait]
impl Tool for ExportOpenApiTool {
    fn name(&self) -> &'static str {
        "export_openapi"
    }

    fn description(&self) -> &'static str {
        "Export a project's contracts as OpenAPI 3.0.3 YAML. Returns the YAML string and a download URL."
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
        let r = crate::domain::use_cases::export_openapi::execute(ctx.state.repos(), slug).await?;
        Ok(json!({ "yaml": r.yaml, "download_url": r.download_url }))
    }
}