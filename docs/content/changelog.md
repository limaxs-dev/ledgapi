---
title: Changelog
description: Release notes for ledgapi. Newest first.
---

## v0.0.1

The first public, open-source release. See the top-level `CHANGELOG.md` for
the full release notes. This in-app changelog now mirrors the public one;
prior development history is summarized below for context.

## Prior development (pre-public)

The project was developed internally before the open-source release. The
following notes record the cumulative capabilities at the time of the
v0.0.1 public launch. They are not tagged releases.

### OAuth 2.1 with PKCE

Dynamic client registration. The `.mcp.json` file now needs only `type`
and `url`.

The static bootstrap token and the `/setup` endpoint are removed. The
initial super-admin is seeded from `APP__AUTH__INITIAL_ADMIN_USERNAME`
and `APP__AUTH__INITIAL_ADMIN_PASSWORD` on first boot, then never touched
again.

### Audit log

Every create, update, and delete is recorded. The log is visible to
super-admins at `/admin/audit`. Reads are not logged.

Per-contract audit trail is added to the contract detail page.

### RAG duplicate detection

`create_contract` runs a similarity check before insert. Default
threshold `0.85`, configurable through
`APP__EMBED__SIMILARITY_THRESHOLD`. The `force` flag on `create_contract`
bypasses the check.

The `search_contract` tool gains `hybrid` and `exact` modes alongside the
existing `semantic` mode.

### Multi-project support

Every contract is scoped to a project. `list_projects` returns the flat
list. The `project_slug` parameter is required on every other tool.

Nested groups are supported via the `parent_group_name` parameter on
`create_group`.

### OpenAPI export

`export_openapi` returns a YAML string and a download URL. The export is
reachable directly at `/projects/{slug}/openapi.yml`.

### Initial capabilities

Single-binary Rust + Axum server. SQLite (WAL) for relational data,
`sqlite-vec` for vector search, `fastembed-rs` for local embedding
generation. Server-rendered web UI with login, dashboard, project view,
contract detail, search, and admin pages. Three roles: viewer, editor,
super-admin.
