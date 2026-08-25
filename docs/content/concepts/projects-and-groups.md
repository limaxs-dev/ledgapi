---
title: Projects and groups
description: How projects, groups, and contracts nest. The slug is the public identifier; the id is internal.
---

The data model is three levels deep. A project holds many groups. A group holds many contracts. There is no fourth level in v1.

## Project

A project is a registry of its own. The project slug is the namespace every other tool call uses, so pick something short and stable. The slug is unique across the instance.

```json
{
  "slug": "acme-billing",
  "name": "Acme Billing API",
  "description": "Internal billing service for Acme customers"
}
```

The slug cannot be renamed after creation. The `name` and `description` can be updated through `update_project` (planned v2). For now, treat the slug as a permanent identifier.

## Group

A group is a folder inside a project. Most projects have a few: `auth`, `users`, `billing`, `reports`. The group id is internal; the group `name` is the public identifier inside the project, and it is unique within that project.

```json
{
  "project_slug": "acme-billing",
  "name": "auth",
  "description": "Authentication and session endpoints"
}
```

Two groups in the same project cannot share a name. Two groups in different projects can. Groups can be created explicitly through `create_group` (planned v2) or implicitly through `create_contract` by passing `group_name`.

## Contract

A contract describes one endpoint. The path is a public identifier within the project. The id is a UUID assigned at insert time.

```json
{
  "id": "8a4b2f12-1c2d-4e9a-b1c2-1f0a2b3c4d5e",
  "project_slug": "acme-billing",
  "group_name": "auth",
  "method": "POST",
  "path": "/api/v1/auth/login"
}
```

The `id` is what `get_contract_by_id` and `update_contract` and `delete_contract` take. The `(method, path, project_slug)` triple is what `search_contract` matches against.

## Cross-project queries

There is no tool that spans projects. Every read and every write is scoped to a `project_slug`. The exception is `list_projects`, which returns a flat list with the contract count per project.

A super-admin who wants a global view of recent writes reads the audit log at `/admin/audit`. The audit log is the only place where the project boundary is invisible.

## Renaming and deleting

v1 has no `update_project`, no `update_group`, and no `delete_project`. To rename a project, the workaround is to create a new one and re-register every contract. To delete a project, the workaround is to drop the rows from the database directly. Both are planned for v2.

Renaming a group is not supported in v1. Deleting a group with contracts in it cascades and removes every contract inside. The audit log records the cascade as one logical event.
