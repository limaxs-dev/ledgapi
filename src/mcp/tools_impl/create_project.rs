//! `create_project` — thin delegate over `manage_project::create`.

use crate::domain::errors::DomainError;
use crate::domain::project::{ProjectCreate, ProjectSlug};
use crate::mcp::tools::{Tool, ToolContext};
use async_trait::async_trait;
use schemars::{JsonSchema, SchemaGenerator, schema::Schema};
use serde::Deserialize;
use serde_json::{Value, json};

/// `create_project` MCP tool.
pub struct CreateProjectTool;

/// JSON-Schema input for `create_project`.
#[derive(Deserialize, JsonSchema)]
pub struct Input {
    /// URL-safe project slug (lowercase a-z, 0-9, `-`, `_`).
    pub slug: String,
    /// Human-readable project name.
    pub name: String,
    /// Optional longer description.
    #[serde(default)]
    pub description: Option<String>,
}

#[async_trait]
impl Tool for CreateProjectTool {
    fn name(&self) -> &'static str {
        "create_project"
    }

    fn description(&self) -> &'static str {
        "Create a new API contract project."
    }

    fn input_schema(&self) -> Schema {
        let mut generator = SchemaGenerator::default();
        Input::json_schema(&mut generator)
    }

    async fn execute(&self, ctx: ToolContext, input: Value) -> Result<Value, DomainError> {
        ctx.require_scope("ledgapi:write")?;
        let p: Input = serde_json::from_value(input).map_err(|e| DomainError::Validation {
            field: "args".into(),
            message: e.to_string(),
        })?;
        let slug = ProjectSlug::parse(&p.slug)?;
        let out = crate::domain::use_cases::manage_project::create_with_actor(
            ctx.state.repos(),
            &ctx.principal,
            ProjectCreate { slug, name: p.name, description: p.description },
        )
        .await?;
        Ok(json!({ "status": "created", "project_slug": out.slug }))
    }
}
