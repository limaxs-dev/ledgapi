---
title: update_contract
description: Update one or more fields of an existing contract. The RAG embedding is regenerated on every update.
method: WRITE
---

Update one or more fields of an existing contract. The RAG embedding is regenerated every time the `summary` or `description` changes; other field changes do not re-embed.

## Request

| Parameter | Type | Description |
|---|---|---|
| project_slug | string | Required. |
| contract_id | string | Required. |
| group_name | string | Optional. New group; auto-created if it does not exist. |
| summary | string | Optional. |
| description | string | Optional. |
| request_headers | object | Optional. |
| request_params | object | Optional. |
| request_body_schema | object | Optional. |
| request_example | object | Optional. |
| response_schema | object | Optional. |
| response_example | object | Optional. |
| auth_type | string | Optional. |
| status | string | Optional. |
| tags | array of strings | Optional. |

The `method` and `path` cannot be updated. To change either, delete the contract and create a new one. The id is a UUID, the `(method, path)` pair is the public identity.

## Response

```json
{ "status": "updated", "contract_id": "8a4b..." }
```

The audit log records the diff between the old and new values.

## Errors

| Status | When |
|---|---|
| 401 | No bearer token. |
| 403 | The token does not have `ledgapi:write`. |
| 404 | The project or contract does not exist. |
| 422 | An attempted field is `method` or `path`. |

## Notes

There is no field-level version history in v1. The audit log records who changed what and when, but the previous value is overwritten. A future version may keep per-field history.
