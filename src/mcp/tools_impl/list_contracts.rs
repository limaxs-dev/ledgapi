//! `list_contracts` — thin delegate over `create_contract::list`.

use crate::domain::contract::Status;
use crate::domain::errors::DomainError;
use crate::domain::ports::ListContractsFilter;
use crate::domain::project::ProjectSlug;
use crate::mcp::tools::{Tool, ToolContext};
use async_trait::async_trait;
use schemars::{JsonSchema, SchemaGenerator, schema::Schema};
use serde::Deserialize;
use serde_json::{Value, json};

/// `list_contracts` MCP tool.
pub struct ListContractsTool;

/// JSON-Schema input for `list_contracts`.
#[derive(Deserialize, JsonSchema)]
pub struct Input {
    pub project_slug: String,
    /// Optional group name filter.
    #[serde(default)]
    pub group_name: Option<String>,
    /// Optional status filter (`draft`, `stable`, `deprecated`).
    #[serde(default)]
    pub status: Option<String>,
    /// Optional max results. Default 100, max 500.
    #[serde(default)]
    pub limit: Option<i64>,
}

#[async_trait]
impl Tool for ListContractsTool {
    fn name(&self) -> &'static str {
        "list_contracts"
    }

    fn description(&self) -> &'static str {
        "List contracts in a project. Optional group_name and status filters; limit default 100, max 500."
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

        let mut filter = ListContractsFilter::default();
        if let Some(s) = &p.status {
            filter.status = Some(Status::parse(s).ok_or_else(|| DomainError::Validation {
                field: "status".into(),
                message: "must be one of: draft, stable, deprecated".into(),
            })?);
        }
        if let Some(name) = &p.group_name {
            // Resolve the group via use case (resolves-or-creates).
            let group = crate::domain::use_cases::manage_group::resolve(
                ctx.state.repos(),
                ctx.project_id,
                crate::domain::group::GroupRef { name: name.clone(), description: None },
            )
            .await?;
            filter.group_id = Some(group.id);
        }
        filter.limit = p.limit.unwrap_or(100).clamp(1, 500);

        let list = crate::domain::use_cases::create_contract::list(ctx.state.repos(), slug, filter)
            .await?;
        let arr: Vec<Value> = list
            .iter()
            .map(|c| {
                json!({
                    "id": c.id.to_string(),
                    "method": c.method.as_str(),
                    "path": c.path,
                    "summary": c.summary,
                    "status": c.status.as_str(),
                    "tags": c.tags,
                })
            })
            .collect();
        Ok(json!({ "contracts": arr }))
    }
}
