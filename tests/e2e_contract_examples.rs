mod common;

use common::TestApp;
use http_body_util::BodyExt;
use ledgapi::domain::contract::{ContractExampleInput, ExampleKind, Method};
use ledgapi::domain::ports::Repos;
use ledgapi::domain::project::{ProjectCreate, ProjectSlug};
use serde_json::{Value, json};

fn with_bearer(
    mut req: axum::http::Request<axum::body::Body>,
    plaintext: &str,
) -> axum::http::Request<axum::body::Body> {
    use axum::http::header::AUTHORIZATION;
    req.headers_mut().insert(AUTHORIZATION, format!("Bearer {plaintext}").parse().unwrap());
    req
}

async fn setup_app() -> (TestApp, String, ledgapi::core::id::Id) {
    let app = TestApp::new();
    let access_token = app.seed_admin_access_token().await;
    let project = app
        .state
        .repos
        .projects()
        .create(&ProjectCreate {
            slug: ProjectSlug::parse("api").unwrap(),
            name: "API".to_owned(),
            description: None,
        })
        .await
        .unwrap();
    (app, access_token, project.id)
}

fn example_args() -> Value {
    json!({
        "project_slug": "api",
        "method": "POST",
        "path": "/users",
        "summary": "Create user",
        "response_schema": {"type": "object"},
        "examples": [
            {
                "name": "happy-path",
                "kind": "positive",
                "status_code": 201,
                "request": {"name": "Ada"},
                "response": {"id": 1}
            },
            {
                "name": "validation-error",
                "kind": "negative",
                "status_code": 422,
                "request": {"name": ""},
                "response": {"error": "invalid name"}
            }
        ],
        "force": true
    })
}

async fn response_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn mcp_contract_examples_round_trip_and_replace() {
    let (app, plaintext, _) = setup_app().await;
    let create = app
        .oneshot(with_bearer(
            TestApp::mcp_request(
                "tools/call",
                json!({
                    "name": "create_contract",
                    "arguments": example_args()
                }),
            ),
            &plaintext,
        ))
        .await;
    let create_body = response_json(create).await;
    let contract_id =
        create_body["result"]["content"][0]["json"]["contract_id"].as_str().unwrap().to_owned();

    let get = app
        .oneshot(with_bearer(
            TestApp::mcp_request(
                "tools/call",
                json!({
                    "name": "get_contract_by_id",
                    "arguments": {"project_slug": "api", "contract_id": contract_id}
                }),
            ),
            &plaintext,
        ))
        .await;
    let get_body = response_json(get).await;
    let contract = &get_body["result"]["content"][0]["json"];
    assert_eq!(contract["examples"].as_array().unwrap().len(), 2);
    assert_eq!(contract["examples"][0]["name"], "happy-path");
    assert_eq!(contract["examples"][1]["status_code"], 422);

    let update = app
        .oneshot(with_bearer(
            TestApp::mcp_request(
                "tools/call",
                json!({
                    "name": "update_contract",
                    "arguments": {
                        "project_slug": "api",
                        "contract_id": contract_id,
                        "examples": []
                    }
                }),
            ),
            &plaintext,
        ))
        .await;
    let update_body = response_json(update).await;
    assert_eq!(update_body["result"]["content"][0]["json"]["status"], "updated");

    let get_after_update = app
        .oneshot(with_bearer(
            TestApp::mcp_request(
                "tools/call",
                json!({
                    "name": "get_contract_by_id",
                    "arguments": {"project_slug": "api", "contract_id": contract_id}
                }),
            ),
            &plaintext,
        ))
        .await;
    let get_after_update_body = response_json(get_after_update).await;
    assert!(
        get_after_update_body["result"]["content"][0]["json"]["examples"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn contract_detail_renders_multiple_examples() {
    let (app, _, project_id) = setup_app().await;
    let contract = app
        .state
        .repos
        .contracts()
        .create(
            project_id,
            None,
            &ledgapi::domain::contract::ContractCreate {
                method: Method::Post,
                path: "/users".to_owned(),
                summary: "Create user".to_owned(),
                description: None,
                request_headers: None,
                request_params: None,
                request_body_schema: Some(json!({"type": "object"})),
                request_example: None,
                response_schema: json!({"type": "object"}),
                response_example: None,
                examples: Some(vec![
                    ContractExampleInput {
                        name: "happy-path".to_owned(),
                        kind: ExampleKind::Positive,
                        status_code: 201,
                        request: json!({"name": "Ada"}),
                        response: json!({"id": 1}),
                    },
                    ContractExampleInput {
                        name: "validation-error".to_owned(),
                        kind: ExampleKind::Negative,
                        status_code: 422,
                        request: json!({"name": ""}),
                        response: json!({"error": "invalid name"}),
                    },
                ]),
                auth_type: None,
                status: None,
                tags: None,
                group_name: None,
                group_parent_id: None,
                force: false,
            },
        )
        .await
        .unwrap();
    let (session, csrf) = app.seed_admin_session().await;
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri(format!("/projects/api/contracts/{}", contract.id))
                .header("cookie", format!("ledgapi_session={session}; ledgapi_csrf={csrf}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), 200);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("Examples"));
    assert!(body.contains("happy-path"));
    assert!(body.contains("validation-error"));
    assert!(body.contains("201"));
    assert!(body.contains("422"));
    assert!(body.contains("invalid name"));
}

#[test]
fn example_input_validation_rejects_invalid_status() {
    let input = ContractExampleInput {
        name: "bad".to_owned(),
        kind: ExampleKind::Negative,
        status_code: 99,
        request: json!({}),
        response: json!({}),
    };
    assert!(input.validate("examples").is_err());
}
