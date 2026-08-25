---
title: list_groups
description: List the groups in a project, with the contract count per group.
method: READ
---

List every group in a project, with the number of contracts in each. Groups are returned in insertion order; there is no stable sort.

## Request

| Parameter | Type | Description |
|---|---|---|
| project_slug | string | Required. The project to list. |

## Response

```json
{
  "groups": [
    { "name": "auth", "contract_count": 6 },
    { "name": "users", "contract_count": 12 },
    { "name": "billing", "contract_count": 24 }
  ]
}
```

`contract_count` is the number of contracts whose `group_name` matches, including drafts and deprecated. Groups that exist but have no contracts still appear in the list, with `contract_count: 0`.

## Errors

| Status | When |
|---|---|
| 401 | No bearer token. |
| 403 | The token does not have `ledgapi:read`. |
| 404 | The project does not exist. |
