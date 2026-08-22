//! Streamable HTTP transport for MCP.
//!
//! POST /mcp — JSON-RPC request body, JSON-RPC response (or SSE-wrapped).
//! Bearer auth enforced by middleware (set up at router level).

use crate::domain::errors::DomainError;
use crate::mcp::tools::ToolContext;
use crate::state::AppState;
use axum::Json;
use axum::extract::{Extension, Request};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

/// Maximum JSON-RPC body size. Per spec §13 #2.
pub const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

/// JSON-RPC 2.0 request envelope.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 response frame.
#[derive(Debug, serde::Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error frame.
#[derive(Debug, serde::Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Top-level handler. Called by the router after auth middleware.
pub async fn handle(Extension(state): Extension<AppState>, req: Request) -> Response {
    // Body parsing with explicit size limit.
    let Ok(bytes) = axum::body::to_bytes(req.into_body(), MAX_BODY_BYTES).await else {
        return respond_error(
            None,
            StatusCode::BAD_REQUEST,
            json!({"code": -32700, "message": "could not read request body"}),
            &state,
        )
        .await;
    };

    let Ok(request) = serde_json::from_slice::<JsonRpcRequest>(&bytes) else {
        // JSON-RPC convention: parse errors use 200 with error frame.
        return respond_error(
            None,
            StatusCode::OK,
            json!({"code": -32700, "message": "parse error"}),
            &state,
        )
        .await;
    };

    let id = request.id.clone().unwrap_or(Value::Null);

    // Notifications have no id → no JSON-RPC response (HTTP 204).
    if request.id.is_none() {
        // Per spec §13 #14: 204 No Content.
        return StatusCode::NO_CONTENT.into_response();
    }

    let response = dispatch(&state, request).await;
    respond(&state, id, response).await
}

async fn dispatch(
    state: &AppState,
    req: JsonRpcRequest,
) -> Result<Value, (i32, String, Option<Value>)> {
    match req.method.as_str() {
        "initialize" => Ok(initialize_response()),
        "notifications/initialized" => {
            // No-op: we already returned 204 above. This branch only fires
            // if a client sends `id: null` and we somehow get here.
            Ok(Value::Null)
        }
        "tools/list" => Ok(tools_list_response(state)),
        "tools/call" => tools_call_dispatch(state, req.params).await,
        _ => Err((-32601, format!("method not found: {}", req.method), None)),
    }
}

fn initialize_response() -> Value {
    json!({
        "protocolVersion": "2025-06-18",
        "serverInfo": { "name": "ledgapi", "version": env!("CARGO_PKG_VERSION") },
        "capabilities": { "tools": {} }
    })
}

fn tools_list_response(state: &AppState) -> Value {
    let tools = state.mcp_registry().list();
    let arr: Vec<Value> = tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name(),
                "description": t.description(),
                "inputSchema": serde_json::to_value(t.input_schema()).unwrap_or(json!({})),
            })
        })
        .collect();
    json!({ "tools": arr })
}

async fn tools_call_dispatch(
    state: &AppState,
    params: Value,
) -> Result<Value, (i32, String, Option<Value>)> {
    let Some(name) = params.get("name").and_then(|v| v.as_str()) else {
        return Err((-32602, "params.name is required".to_owned(), None));
    };
    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
    let Some(tool) = state.mcp_registry().get(name) else {
        return Err((
            -32602,
            format!("unknown tool: {name}"),
            Some(json!({"code":"validation_failed"})),
        ));
    };

    let ctx = build_tool_context(state, &arguments).await?;

    invoke_tool(tool, ctx, arguments).await
}

async fn build_tool_context(
    state: &AppState,
    arguments: &Value,
) -> Result<ToolContext, (i32, String, Option<Value>)> {
    // Resolve project_slug from arguments (every tool except
    // `list_projects` has it; absent for tools that don't).
    let project_slug = arguments.get("project_slug").and_then(|v| v.as_str()).map(str::to_owned);
    let project_id = if let Some(slug) = &project_slug {
        let parsed = crate::domain::project::ProjectSlug::parse(slug).map_err(|e| {
            (-32602, e.message(), Some(json!({"code":"validation_failed","field":"project_slug"})))
        })?;
        match state.repos().projects().find_by_slug(&parsed).await {
            Ok(Some(p)) => p.id,
            Ok(None) => {
                return Err((
                    -32602,
                    "project not found".to_owned(),
                    Some(json!({"code":"not_found"})),
                ));
            }
            Err(e) => {
                return Err((-32603, e.message(), Some(json!({"code":"internal_error"}))));
            }
        }
    } else {
        crate::core::id::Id::nil()
    };

    Ok(ToolContext {
        project_slug: project_slug.unwrap_or_default(),
        project_id,
        state: Arc::new(state.clone()),
    })
}

async fn invoke_tool(
    tool: Arc<dyn crate::mcp::tools::Tool>,
    ctx: ToolContext,
    arguments: Value,
) -> Result<Value, (i32, String, Option<Value>)> {
    match tool.execute(ctx, arguments).await {
        Ok(out) => {
            // Wrap result as MCP structuredContent.
            Ok(json!({
                "content": [{"type": "json", "json": out}],
                "isError": false,
            }))
        }
        Err(e) if !e.is_mcp_error() => {
            // SimilarFound — return as successful tool result.
            let payload = match e {
                DomainError::SimilarFound { candidates } => json!({
                    "status": "warning_similar_found",
                    "similar_contracts": candidates,
                    "message": "Similar contracts found. Call update_contract on a match, or resend with force=true to create anyway."
                }),
                _ => unreachable!(),
            };
            Ok(json!({
                "content": [{"type": "json", "json": payload}],
                "isError": false,
            }))
        }
        Err(e) => {
            let mut data = json!({"code": e.code().as_symbol()});
            if let Some(field) = e.field() {
                data["field"] = json!(field);
            }
            Err((
                match e.code().as_symbol() {
                    "validation_failed" | "not_found" | "duplicate_key" => -32602,
                    _ => -32603,
                },
                e.message(),
                Some(data),
            ))
        }
    }
}

/// Build the JSON-RPC response, wrapping in SSE if the client prefers it.
async fn respond(
    state: &AppState,
    id: Value,
    result: Result<Value, (i32, String, Option<Value>)>,
) -> Response {
    let response = match result {
        Ok(value) => JsonRpcResponse { jsonrpc: "2.0", id, result: Some(value), error: None },
        Err((code, message, data)) => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError { code, message, data }),
        },
    };
    let body = serde_json::to_vec(&response).unwrap_or_default();
    respond_bytes(state, body).await
}

async fn respond_error(
    id: Option<Value>,
    status: StatusCode,
    err_body: Value,
    _state: &AppState,
) -> Response {
    let id = id.unwrap_or(Value::Null);
    let frame = JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(JsonRpcError {
            code: err_body["code"].as_i64().unwrap_or(-32603) as i32,
            message: err_body["message"].as_str().unwrap_or("").to_owned(),
            data: None,
        }),
    };
    let body = serde_json::to_vec(&frame).unwrap_or_default();
    let mut resp = (status, body).into_response();
    resp.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    resp
}

async fn respond_bytes(_state: &AppState, body: Vec<u8>) -> Response {
    // Per spec §5.1: respond JSON by default; SSE-wrapped if client
    // requested `Accept: text/event-stream`. We do not currently inspect
    // the Accept header — clients that prefer JSON work without SSE.
    // If SSE is required, the simplest wrapper is below.
    let mut resp = (StatusCode::OK, body).into_response();
    resp.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    resp
}

/// SSE variant. Used only when the client sends `Accept: text/event-stream`.
#[allow(dead_code)]
pub async fn handle_sse(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    Json(req): Json<JsonRpcRequest>,
) -> Response {
    let id = req.id.clone().unwrap_or(Value::Null);
    if req.id.is_none() {
        return StatusCode::NO_CONTENT.into_response();
    }
    let value = dispatch(&state, req).await;
    let response = match value {
        Ok(v) => JsonRpcResponse { jsonrpc: "2.0", id, result: Some(v), error: None },
        Err((code, message, data)) => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError { code, message, data }),
        },
    };
    let body = serde_json::to_string(&response).unwrap_or_default();
    let sse = format!("event: message\ndata: {body}\n\n");
    let mut resp = (StatusCode::OK, sse).into_response();
    if headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.contains("text/event-stream"))
    {
        resp.headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
    } else {
        resp.headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    }
    resp
}

/// Silence unused-import warning while we keep `Next` for future middleware composition.
#[allow(dead_code)]
fn _next_marker(_: Next) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_rpc_response_serializes() {
        let r = JsonRpcResponse {
            jsonrpc: "2.0",
            id: json!(1),
            result: Some(json!({"ok": true})),
            error: None,
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["jsonrpc"], json!("2.0"));
        assert_eq!(v["id"], json!(1));
        assert_eq!(v["result"]["ok"], json!(true));
        assert!(v.get("error").is_none());
    }

    #[test]
    fn json_rpc_error_carries_code_and_data() {
        let e = JsonRpcError {
            code: -32602,
            message: "x".into(),
            data: Some(json!({"code":"validation_failed"})),
        };
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["code"], json!(-32602));
        assert_eq!(v["message"], json!("x"));
        assert_eq!(v["data"]["code"], json!("validation_failed"));
    }
}
