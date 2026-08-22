//! `export_openapi` — build OpenAPI 3.0.3 YAML from a project's contracts.
//! See spec §13 #7/#10/#11/#12/#20.

use crate::domain::contract::AuthType;
use crate::domain::errors::DomainError;
use crate::domain::ports::{ListContractsFilter, Repos};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

pub async fn execute(repos: &dyn Repos, project_slug: crate::domain::project::ProjectSlug)
    -> Result<ExportResult, DomainError>
{
    let project = repos
        .projects()
        .find_by_slug(&project_slug)
        .await?
        .ok_or(DomainError::NotFound { resource: "project" })?;

    let contracts = repos
        .contracts()
        .list(project.id, &ListContractsFilter { limit: 100_000, ..Default::default() })
        .await?;

    // We need full contract bodies, not summaries. Re-fetch each.
    let mut full = Vec::with_capacity(contracts.len());
    for c in &contracts {
        let con = repos.contracts().find_by_id(project.id, c.id).await?;
        full.push(con);
    }

    let doc = build_doc(&project.name, project.description.as_deref(), &full);
    let yaml = serde_yaml::to_string(&doc).map_err(|e| DomainError::Internal(format!("yaml: {e}")))?;

    Ok(ExportResult {
        yaml,
        download_url: format!("/projects/{}/openapi.yml", project.slug),
    })
}

pub struct ExportResult {
    pub yaml: String,
    pub download_url: String,
}

fn build_doc(title: &str, description: Option<&str>, contracts: &[crate::domain::contract::Contract]) -> Value {
    let mut paths: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
    let mut security_schemes: Map<String, Value> = Map::new();
    let mut tags: Vec<Value> = Vec::new();

    for c in contracts {
        let op = build_operation(c);
        paths
            .entry(c.path.clone())
            .or_default()
            .insert(c.method.as_str().to_lowercase(), op);

        if let Some(a) = c.auth_type {
            add_security_scheme(&mut security_schemes, a);
        }
        for t in &c.tags {
            let v = Value::String(t.clone());
            if !tags.contains(&v) {
                tags.push(v);
            }
        }
    }

    let mut components = Map::new();
    if !security_schemes.is_empty() {
        components.insert("securitySchemes".to_owned(), Value::Object(security_schemes));
    }

    let paths_json: Map<String, Value> = paths
        .into_iter()
        .map(|(p, ops)| (p, Value::Object(ops.into_iter().collect())))
        .collect();

    let mut info = Map::new();
    info.insert("title".into(), Value::String(title.to_owned()));
    info.insert("version".into(), Value::String("1.0.0".into()));
    if let Some(d) = description {
        info.insert("description".into(), Value::String(d.to_owned()));
    }

    let mut root = Map::new();
    root.insert("openapi".into(), Value::String("3.0.3".into()));
    root.insert("info".into(), Value::Object(info));
    root.insert("paths".into(), Value::Object(paths_json));
    if !components.is_empty() {
        root.insert("components".into(), Value::Object(components));
    }
    if !tags.is_empty() {
        root.insert("tags".into(), Value::Array(tags));
    }
    Value::Object(root)
}

fn build_operation(c: &crate::domain::contract::Contract) -> Value {
    let mut op = Map::new();
    op.insert("summary".into(), Value::String(c.summary.clone()));
    if let Some(d) = &c.description { op.insert("description".into(), Value::String(d.clone())); }

    // Parameters from path template + request_params JSON Schema.
    let mut parameters: Vec<Value> = Vec::new();
    for cap in path_param_names(&c.path) {
        parameters.push(json!({
            "name": cap,
            "in": "path",
            "required": true,
            "schema": {"type": "string"}
        }));
    }
    if let Some(params) = &c.request_params {
        extend_params_from_schema(&mut parameters, params);
    }
    if !parameters.is_empty() {
        op.insert("parameters".into(), Value::Array(parameters));
    }

    // Body
    if let Some(body) = &c.request_body_schema {
        op.insert("requestBody".into(), json!({
            "required": true,
            "content": {"application/json": {"schema": body}}
        }));
    }

    // Response
    let mut resp_content = Map::new();
    resp_content.insert("application/json".into(), json!({"schema": c.response_schema}));
    op.insert("responses".into(), json!({
        "200": {
            "description": "Successful response",
            "content": resp_content
        }
    }));

    if let Some(a) = c.auth_type {
        if a != AuthType::None {
            op.insert("security".into(), Value::Array(vec![json!({a.as_str(): []})]));
        }
    }

    if !c.tags.is_empty() {
        op.insert("tags".into(), Value::Array(c.tags.iter().map(|t| Value::String(t.clone())).collect()));
    }

    Value::Object(op)
}

fn add_security_scheme(schemes: &mut Map<String, Value>, auth: AuthType) {
    let key = auth.as_str().to_owned() + "Auth";
    let val = match auth {
        AuthType::Bearer => json!({"type":"http","scheme":"bearer"}),
        AuthType::ApiKey => json!({"type":"apiKey","in":"header","name":"X-API-Key"}),
        AuthType::Basic => json!({"type":"http","scheme":"basic"}),
        AuthType::None => return,
    };
    schemes.entry(key).or_insert(val);
}

fn path_param_names(path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = path.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(end) = path[i+1..].find('}') {
                let name = &path[i+1..i+1+end];
                out.push(name.to_owned());
                i = i + 1 + end + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn extend_params_from_schema(parameters: &mut Vec<Value>, schema: &Value) {
    // If schema is a JSON Schema object with `properties`, treat each
    // property as a query parameter unless it has an explicit `"in"` field.
    if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
        for (name, sch) in props {
            let location = sch.get("in").and_then(|i| i.as_str()).unwrap_or("query");
            let mut sch_clone = sch.clone();
            if let Some(obj) = sch_clone.as_object_mut() {
                obj.remove("in");
            }
            parameters.push(json!({
                "name": name,
                "in": location,
                "schema": sch_clone
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::contract::{Contract, ContractCreate, Method, Status};
    use crate::core::id::Id;
    use crate::domain::project::{ProjectCreate, ProjectSlug};
    use crate::infra::db::pool::open_memory;
    use crate::infra::repos::SqliteRepos;

    #[tokio::test]
    async fn export_with_no_contracts_produces_valid_yaml() {
        let db = open_memory().unwrap();
        let repos = SqliteRepos::new(db);
        repos.projects().create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "My API".to_owned(),
            description: Some("desc".to_owned()),
        }).await.unwrap();
        let r = execute(&repos, ProjectSlug::parse("api").unwrap()).await.unwrap();
        assert!(r.yaml.contains("openapi: 3.0.3"));
        assert!(r.yaml.contains("title: My API"));
        assert!(r.yaml.contains("version: 1.0.0"));
        assert!(r.yaml.contains("description: desc"));
        // Round-trip via yaml parser
        let _: serde_yaml::Value = serde_yaml::from_str(&r.yaml).unwrap();
    }

    #[test]
    fn path_param_names_extracts_braces() {
        assert_eq!(path_param_names("/users/{id}/posts/{post_id}"), vec!["id", "post_id"]);
        assert_eq!(path_param_names("/users"), Vec::<String>::new());
    }

    #[test]
    fn operation_includes_path_param() {
        let c = Contract {
            id: Id::new(),
            project_id: Id::new(),
            group_id: None,
            method: Method::Get,
            path: "/users/{id}".to_owned(),
            summary: "Get user".to_owned(),
            description: None,
            request_headers: None,
            request_params: None,
            request_body_schema: None,
            request_example: None,
            response_schema: json!({"type":"object"}),
            response_example: None,
            auth_type: None,
            status: Status::Draft,
            tags: vec![],
            created_at: time::OffsetDateTime::UNIX_EPOCH,
            updated_at: time::OffsetDateTime::UNIX_EPOCH,
        };
        let op = build_operation(&c);
        let params = op["parameters"].as_array().unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0]["name"], "id");
        assert_eq!(params[0]["in"], "path");
    }

    #[allow(dead_code)]
    fn _cc_marker(_: ContractCreate) {}
}
