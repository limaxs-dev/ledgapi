---
title: create_project
description: Create a new project. The slug is the public identifier and cannot be renamed.
method: WRITE
---

Create a new project. The slug is the namespace every other tool call uses to scope requests.

## Request

| Parameter | Type | Description |
|---|---|---|
| slug | string | Required. Lowercase, hyphens allowed. Must be unique across the instance. |
| name | string | Required. Human-readable name. Can be changed later. |
| description | string | Optional. Free text, up to 1 KB. |

## Response

```json
{ "status": "created", "project_slug": "acme-billing" }
```

The new project appears in `list_projects` immediately.

## Errors

| Status | When |
|---|---|
| 401 | No bearer token. |
| 403 | The token does not have `ledgapi:write`. |
| 409 | A project with this slug already exists. |
| 422 | The slug has invalid characters, or a required field is missing. |

The slug must match `^[a-z0-9][a-z0-9-]{0,62}$`. Slugs are case-insensitive on input and stored lowercase.

## Notes

There is no `update_project` in v1. To rename a project, the workaround is to create a new one and re-register every contract. There is no `delete_project` either; the workaround is a direct `DELETE FROM projects WHERE slug = ?` followed by a manual cleanup of the audit log if desired.
