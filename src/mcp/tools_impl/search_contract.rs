//! `search_contract` — thin delegate over `search_contract::execute`.

use crate::domain::errors::DomainError;
use crate::domain::ports::SearchMode;
use crate::domain::project::ProjectSlug;
use crate::mcp::tools::{Tool, ToolContext};
use async_trait::async_trait;
use schemars::{JsonSchema, SchemaGenerator, schema::Schema};
use serde::Deserialize;
use serde_json::{Value, json};

/// `search_contract` MCP tool.
pub struct SearchContractTool;

/// JSON-Schema input for `search_contract`.
#[derive(Deserialize, JsonSchema)]
pub struct Input {
    pub project_slug: String,
    /// Free-text query.
    pub query: String,
    /// `exact`, `semantic`, or `hybrid` (default).
    #[serde(default = "default_mode")]
    pub search_mode: String,
    /// Optional group name filter.
    #[serde(default)]
    pub group_name: Option<String>,
    /// Optional max results.
    #[serde(default)]
    pub limit: Option<i64>,
}

fn default_mode() -> String {
    "hybrid".into()
}

#[async_trait]
impl Tool for SearchContractTool {
    fn name(&self) -> &'static str {
        "search_contract"
    }

    fn description(&self) -> &'static str {
        "Hybrid search: exact + semantic with RRF merge. Modes: exact, semantic, hybrid (default). Empty query rejected."
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
        let mode = SearchMode::parse(&p.search_mode)?;
        let results = crate::domain::use_cases::search_contract::execute(
            ctx.state.repos(),
            ctx.state.embedder(),
            ctx.state.embed_cfg(),
            slug,
            &p.query,
            mode,
            p.group_name.as_deref(),
            p.limit,
        )
        .await?;
        let arr: Vec<Value> = results
            .iter()
            .map(|r| {
                json!({
                    "id": r.id.to_string(),
                    "method": r.method.as_str(),
                    "path": r.path,
                    "summary": r.summary,
                    "status": r.status.as_str(),
                    "tags": r.tags,
                    "similarity": r.similarity,
                })
            })
            .collect();
        Ok(json!({ "results": arr }))
    }
}