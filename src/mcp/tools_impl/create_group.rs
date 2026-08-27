//! `create_group` — create a group (folder) in a project, optionally nested
//! under another group via `parent_id`.

use crate::domain::errors::DomainError;
use crate::domain::group::GroupRef;
use crate::domain::project::ProjectSlug;
use crate::mcp::tools::{Tool, ToolContext};
use async_trait::async_trait;
use schemars::{JsonSchema, SchemaGenerator, schema::Schema};
use serde::Deserialize;
use serde_json::{Value, json};

/// `create_group` MCP tool.
pub struct CreateGroupTool;

/// JSON-Schema input for `create_group`.
#[derive(Deserialize, JsonSchema)]
pub struct Input {
    pub project_slug: String,
    /// Group name (1-100 chars, must be unique within the same parent).
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Optional parent group id. `None` means root-level under the project.
    /// Rendered as a string in JSON Schema because `Id` is not a primitive type.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub parent_id: Option<String>,
}

#[async_trait]
impl Tool for CreateGroupTool {
    fn name(&self) -> &'static str {
        "create_group"
    }

    fn description(&self) -> &'static str {
        "Create a group (folder) in a project, optionally nested under another group. Returns the new group's id. If a group with the same name and parent already exists, returns the existing group."
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
        let project = ctx
            .state
            .repos()
            .projects()
            .find_by_slug(&slug)
            .await?
            .ok_or(DomainError::NotFound { resource: "project" })?;
        let parent_id = match p.parent_id.as_deref() {
            None | Some("") => None,
            Some(s) => Some(crate::core::id::Id::parse(s).ok_or(DomainError::Validation {
                field: "parent_id".to_owned(),
                message: format!("not a valid id: {s}"),
            })?),
        };
        let group_ref = GroupRef { name: p.name, description: p.description, parent_id };
        let group = crate::domain::use_cases::manage_group::resolve(
            ctx.state.repos(),
            project.id,
            group_ref,
        )
        .await?;
        Ok(json!({
            "id": group.id.to_string(),
            "name": group.name,
            "parent_id": group.parent_id.map(|i| i.to_string()),
            "project_id": group.project_id.to_string(),
        }))
    }
}
