# Changelog

All notable changes to ledgapi will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.2] - 2026-08-28

### Changed

- **Default port changed from `8080` to `18080`.** Port 8080 collides with
  too many other dev servers and reverse proxies already running on local
  machines, which forced users to remap on every first install. The new
  default sits in a less-crowded range while still being easy to remember.
- The default port is now configurable independently for host and
  container via the new `APP_HOST_PORT` and `APP_CONTAINER_PORT` env vars
  read by `docker-compose.yaml`. The container-internal bind address
  (`APP__SERVER__BIND`) and OAuth issuer (`APP__AUTH__ISSUER`) are
  derived from them automatically, so users no longer have to keep four
  values in sync when picking a non-default port.

### Migration

- **No code or schema changes.** v0.0.2 is a default-value change only;
  the binary still accepts `APP__SERVER__BIND=0.0.0.0:8080` and
  `APP__AUTH__ISSUER=http://localhost:8080` for anyone who wants to
  keep the old behaviour. Existing volumes, tokens, and audit logs are
  untouched.
- If you previously set `APP__SERVER__BIND` or `APP__AUTH__ISSUER`
  explicitly to a non-default value, no action is required.
- If you relied on the default and need port 8080, set
  `APP_HOST_PORT=8080 APP_CONTAINER_PORT=8080` in your environment
  before `docker compose up` (and override `APP__AUTH__ISSUER` /
  `APP__SERVER__BIND` if you run the binary directly).

## [0.0.1] - 2026-08-28

The first public, open-source release of ledgapi. The codebase is published as a
self-hosted, agent-native API contract registry with MCP integration.

### Added

- **MCP server** with 11 purpose-built tools: `list_projects`, `create_project`,
  `list_groups`, `create_group`, `list_contracts`, `get_contract_by_id`,
  `create_contract`, `update_contract`, `delete_contract`, `search_contract`,
  `export_openapi`.
- **RAG duplicate detection** on `create_contract` using local sentence
  embeddings (`all-MiniLM-L6-v2` via `fastembed-rs`) and `sqlite-vec` for
  vector search. Threshold configurable via `APP__EMBED__SIMILARITY_THRESHOLD`
  (default `0.85`); `force=true` overrides the warning.
- **Hybrid search** in `search_contract` with `semantic`, `exact`, and `hybrid`
  modes.
- **OAuth 2.1 + PKCE** for MCP clients. Dynamic client registration, no static
  API tokens in `.mcp.json` — the client only carries `type` and `url`.
- **Multi-project, multi-group** registry. Every contract is scoped to a
  project; groups can be nested (`parent_group_name` on `create_group`).
- **Three roles**: `viewer` (read), `editor` (CRUD via MCP),
  `super-admin` (user management + global audit log). Initial super-admin is
  seeded from `APP__AUTH__INITIAL_ADMIN_USERNAME` and
  `APP__AUTH__INITIAL_ADMIN_PASSWORD` on first boot.
- **Append-only audit log** of every create / update / delete with its acting
  user. Visible per-contract in the web UI and globally at `/admin/audit` for
  super-admins. Reads, logins, and failed writes are not logged.
- **OpenAPI 3.x export** per project as `GET /projects/{slug}/openapi.yml` or
  via the `export_openapi` MCP tool.
- **Server-rendered web UI** with login, dashboard, project view, contract
  detail, search, and admin pages. No client JavaScript framework.
- **Single static binary** in a slim Debian image (`~120 MB` on disk,
  `~50 MB` RSS after warm-up). Self-contained, no third-party API calls,
  no telemetry, no `unsafe` in the codebase.
- **Health probes** at `GET /healthz` and `GET /readyz`.

### Security

- Passwords hashed with Argon2id.
- Session cookies are opaque, HttpOnly, hashed at rest in `web_sessions`.
- OAuth tokens (access, refresh, authorization codes) are hashed at rest;
  authorization codes are single-use with a short TTL.
- CSRF token bound to each session.
- `APP__AUTH__COOKIE_SECURE` enforces `Secure` cookies when serving over HTTPS.

### Notes for v0.0.x

- The pre-release development history was tracked in
  `docs/content/changelog.md` (versions v0.1.0–v0.6.0). Those notes describe
  internal milestones leading up to the open-source launch; the public
  changelog starts here at v0.0.1.
- `0.0.x` versions are pre-1.0: anything may change between minor versions
  (database schema, MCP tool signatures, configuration keys). Read the
  diff before upgrading.

[Unreleased]: https://github.com/limaxs-dev/ledgapi/compare/v0.0.2...HEAD
[0.0.2]: https://github.com/limaxs-dev/ledgapi/releases/tag/v0.0.2
[0.0.1]: https://github.com/limaxs-dev/ledgapi/releases/tag/v0.0.1
