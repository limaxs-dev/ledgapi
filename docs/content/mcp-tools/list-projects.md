---
title: list_projects
description: List every project in the instance, with the contract count per project.
method: READ
---

List every project the calling user can see. Every authenticated user sees every project; there is no per-project ACL in v1.

## Request

No input. Pass an empty JSON object.

## Response

```json
{
  "projects": [
    { "slug": "acme-billing", "name": "Acme Billing API", "contract_count": 42 },
    { "slug": "acme-storefront", "name": "Acme Storefront", "contract_count": 18 }
  ]
}
```

`contract_count` is the number of contracts in the project, including drafts and deprecated. Use `list_contracts` with a `status` filter to narrow it.

## Errors

| Status | When |
|---|---|
| 401 | No bearer token, or the token is expired or revoked. |
| 403 | The token does not have `ledgapi:read`. |

This tool is the only one that does not require a `project_slug`. Every other tool starts by scoping to a project.
