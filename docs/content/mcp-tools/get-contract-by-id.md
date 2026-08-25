---
title: get_contract_by_id
description: Retrieve the full record of a single contract, including both schemas, both examples, and the timestamps.
method: READ
---

Retrieve the full record of a contract. Use `list_contracts` to find the id, then call this tool to load the schemas.

## Request

| Parameter | Type | Description |
|---|---|---|
| project_slug | string | Required. The project that owns the contract. |
| contract_id | string | Required. The UUID returned by `create_contract` or `list_contracts`. |

## Response

```json
{
  "id": "8a4b2f12-1c2d-4e9a-b1c2-1f0a2b3c4d5e",
  "project_slug": "acme-billing",
  "group_name": "auth",
  "method": "POST",
  "path": "/api/v1/auth/login",
  "summary": "Exchange username and password for a session token",
  "description": "Used by the web client and the mobile app.",
  "request_headers": null,
  "request_params": null,
  "request_body_schema": { "type": "object", "...": "..." },
  "request_example":    { "username": "alice", "password": "..." },
  "response_schema":    { "type": "object", "...": "..." },
  "response_example":   { "access_token": "eyJ...", "refresh_token": "...", "expires_in": 900 },
  "auth_type": "none",
  "status": "stable",
  "tags": ["public", "rate-limited"],
  "created_at": "2026-08-21T10:00:00Z",
  "updated_at": "2026-08-22T14:30:00Z"
}
```

## Errors

| Status | When |
|---|---|
| 401 | No bearer token. |
| 403 | The token does not have `ledgapi:read`. |
| 404 | The project does not exist, or the contract id is not in this project. |
