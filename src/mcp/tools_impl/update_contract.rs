//! `update_contract` — thin delegate over `update_contract::execute`.

use crate::domain::contract::{ContractUpdate, Method};
use crate::domain::errors::DomainError;
use crate::domain::project::ProjectSlug;
use crate::mcp::tools::{Tool, ToolContext};
use async_trait::async_trait;
use schemars::{JsonSchema, SchemaGenerator, schema::Schema};
use serde::Deserialize;
use serde_json::{Value, json};

/// `update_contract` MCP tool.
pub struct UpdateContractTool;

/// JSON-Schema input for `update_contract`.
#[derive(Deserialize, JsonSchema, Default)]
pub struct Input {
    pub project_slug: String,
    /// Contract id (UUIDv7).
    pub contract_id: String,
    /// HTTP method (case-insensitive).
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub request_headers: Option<Value>,
    #[serde(default)]
    pub request_params: Option<Value>,
    #[serde(default)]
    pub request_body_schema: Option<Value>,
    #[serde(default)]
    pub request_example: Option<Value>,
    #[serde(default)]
    pub response_schema: Option<Value>,
    #[serde(default)]
    pub response_example: Option<Value>,
    #[serde(default)]
    pub auth_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub group_name: Option<String>,
}

#[async_trait]
impl Tool for UpdateContractTool {
    fn name(&self) -> &'static str {
        "update_contract"
    }

    fn description(&self) -> &'static str {
        "Overwrite fields on a contract. Silent overwrite in v1 — no diff, no warning. Regenerates the embedding if method/path/summary/description change."
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
        let method = p.method.as_deref().and_then(Method::parse);

        let patch = ContractUpdate {
            method,
            path: p.path,
            summary: p.summary,
            description: p.description,
            request_headers: p.request_headers,
            request_params: p.request_params,
            request_body_schema: p.request_body_schema,
            request_example: p.request_example,
            response_schema: p.response_schema,
            response_example: p.response_example,
            auth_type: p.auth_type,
            status: p.status,
            tags: p.tags,
            group_name: p.group_name,
        };
        let out = crate::domain::use_cases::update_contract::execute(
            ctx.state.repos(),
            ctx.state.embedder(),
            ctx.state.embed_cfg(),
            slug,
            id,
            patch,
        )
        .await?;
        Ok(json!({ "status": "updated", "contract_id": out.id }))
    }
}
