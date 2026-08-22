//! `create_contract` — thin delegate over `create_contract::execute`.

use crate::domain::contract::{ContractCreate, Method};
use crate::domain::errors::DomainError;
use crate::domain::project::ProjectSlug;
use crate::mcp::tools::{Tool, ToolContext};
use async_trait::async_trait;
use schemars::{JsonSchema, SchemaGenerator, schema::Schema};
use serde::Deserialize;
use serde_json::{Value, json};

/// `create_contract` MCP tool.
pub struct CreateContractTool;

/// JSON-Schema input for `create_contract`.
#[derive(Deserialize, JsonSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct Input {
    pub project_slug: String,
    /// HTTP method (case-insensitive: GET/POST/PUT/PATCH/DELETE).
    pub method: String,
    /// Path template (e.g. `/api/v1/users/{id}`).
    pub path: String,
    /// Short summary (1-300 chars).
    pub summary: String,
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
    /// Required response JSON Schema.
    pub response_schema: Value,
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
    /// Bypass the similarity warning. Without `true`, a warning_similar_found
    /// result is returned when a too-similar contract already exists.
    #[serde(default)]
    pub force: bool,
}

#[async_trait]
impl Tool for CreateContractTool {
    fn name(&self) -> &'static str {
        "create_contract"
    }

    fn description(&self) -> &'static str {
        "Create a contract. Returns warning_similar_found if a too-similar contract exists; pass force=true to bypass."
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
        let method = Method::parse(&p.method).ok_or_else(|| DomainError::Validation {
            field: "method".into(),
            message: "must be GET/POST/PUT/PATCH/DELETE".into(),
        })?;
        let mut cc = ContractCreate {
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
            force: p.force,
        };
        // Drop empty-string group_name so we don't resolve an empty group.
        if cc.group_name.as_deref() == Some("") {
            cc.group_name = None;
        }

        let out = crate::domain::use_cases::create_contract::execute(
            ctx.state.repos(),
            ctx.state.embedder(),
            ctx.state.embed_cfg(),
            slug,
            cc,
        )
        .await?;
        Ok(json!({ "status": out.status, "contract_id": out.contract_id }))
    }
}