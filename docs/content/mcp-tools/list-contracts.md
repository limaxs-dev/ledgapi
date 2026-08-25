---
title: list_contracts
description: List the contracts in a project, optionally filtered by group and status. Returns compact summaries, not full schemas.
method: READ
---

List contracts in a project. The response is a flat array of compact summaries. Use `get_contract_by_id` to retrieve the full record for a specific contract.

## Request

| Parameter | Type | Description |
|---|---|---|
| project_slug | string | Required. The project to list. |
| group_name | string | Optional. Filter by group. |
| status | string | Optional. One of `draft`, `stable`, `deprecated`. |

## Response

```json
[
  { "id": "8a4b...", "method": "POST", "path": "/api/v1/auth/login", "summary": "...", "status": "stable" },
  { "id": "1f0a...", "method": "GET",  "path": "/api/v1/users/{id}",  "summary": "...", "status": "stable" }
]
```

The response is the array directly, not wrapped in an object. Compact summaries exclude the schemas and examples to keep the payload small for editor integrations.

## Errors

| Status | When |
|---|---|
| 401 | No bearer token. |
| 403 | The token does not have `ledgapi:read`. |
| 404 | The project does not exist. |
| 422 | The `status` value is not one of the allowed values. |

## Notes

The list is not paginated in v1. A project with thousands of contracts will return the whole list in one response. The workaround for now is to filter by `group_name` and call the tool once per group.
