---
title: Changelog
description: Release notes for ledgapi. Newest first.
---

## v0.6.0

OAuth 2.1 with PKCE. Dynamic client registration. The `.mcp.json` file now needs only `type` and `url`.

The static bootstrap token and the `/setup` endpoint are removed. The initial super-admin is seeded from `APP__AUTH__INITIAL_ADMIN_USERNAME` and `APP__AUTH__INITIAL_ADMIN_PASSWORD` on first boot, then never touched again.

## v0.5.0

Audit log of every create, update, and delete. The log is visible to super-admins at `/admin/audit`. Reads are not logged.

Per-contract audit trail is added to the contract detail page.

## v0.4.0

RAG duplicate detection on `create_contract`. Default threshold 0.85, configurable through `APP__RAG__SIMILARITY_THRESHOLD`. The `force` flag on `create_contract` bypasses the check.

The `search_contract` tool gains `hybrid` and `exact` modes alongside the existing `semantic` mode.

## v0.3.0

Multi-project support. Every contract is scoped to a project. `list_projects` returns the flat list. The `project_slug` parameter is required on every other tool.

## v0.2.0

OpenAPI export. `export_openapi` returns a YAML string and a download URL. The export is reachable directly at `/projects/{slug}/openapi.yml`.

## v0.1.0

Initial release. Single-project registry, MCP server, web UI, login, session cookies, three roles.
