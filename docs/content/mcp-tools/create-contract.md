---
title: create_contract
description: Register a new endpoint. Runs a RAG duplicate check before insert; resend with force=true to override.
method: WRITE
---

Register a new contract. The server runs a RAG similarity check against the existing contracts in the same project. If a similar contract is found and `force` is not set, the contract is not created and the response is `warning_similar_found`.

## Request

| Parameter | Type | Description |
|---|---|---|
| project_slug | string | Required. |
| group_name | string | Optional. Auto-creates the group if it does not exist. |
| method | string | Required. One of `GET`, `POST`, `PUT`, `PATCH`, `DELETE`. |
| path | string | Required. Must start with `/`. Path parameters in `{name}` form. |
| summary | string | Required. One short sentence. Used in lists and the RAG embedding. |
| description | string | Optional. Longer prose, up to 4 KB. |
| request_headers | object | Optional. JSON Schema for headers. |
| request_params | object | Optional. JSON Schema for path and query parameters. |
| request_body_schema | object | Optional. JSON Schema for the body. |
| request_example | object | Optional. Example body payload. |
| response_schema | object | Required. JSON Schema for the response. |
| response_example | object | Optional. Example response body. |
| auth_type | string | Optional. One of `none`, `bearer`, `api_key`, `basic`, `cookie`. |
| status | string | Optional. One of `draft`, `stable`, `deprecated`. Default `draft`. |
| tags | array of strings | Optional. Free-form labels. |
| force | boolean | Optional. Default `false`. Bypass the duplicate check. |

## Response (created)

```json
{ "status": "created", "contract_id": "8a4b2f12-1c2d-4e9a-b1c2-1f0a2b3c4d5e" }
```

## Response (duplicate)

```json
{
  "status": "warning_similar_found",
  "similar_contracts": [
    { "id": "1f0a...", "method": "POST", "path": "/api/v1/auth/login", "similarity": 0.91 }
  ],
  "message": "Similar contracts found. Resend with force=true if you still want to create."
}
```

## Errors

| Status | When |
|---|---|
| 401 | No bearer token. |
| 403 | The token does not have `ledgapi:write`. |
| 404 | The project does not exist. |
| 409 | A contract with the same `(method, path)` already exists in the project. Path collision is checked exactly; the RAG layer is a softer check. |
| 422 | A required field is missing, or a field has an invalid value. |
