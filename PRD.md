# PRD: API Contract Registry (Self-Hosted, Agent-Native)

**Status:** Draft v1 **Owner:** [your name] **Date:** August 21, 2026

---

## 1. Overview

The API Contract Registry is a self-hosted tool for documenting the API contracts of a project — not an API client (like Postman), but the **source of truth** about which APIs already exist in a codebase/project.

The tool is designed to be **agent-native**: its primary consumers are AI coding agents (Claude Code, etc.) that interact through MCP (Model Context Protocol), rather than humans through a UI. Humans only view/browse through a read-only UI.

### 1.1 Problem Statement

- Postman has API call limitations on the free tier, even though the only need is documentation, not request execution.
- AI agents working on coding tasks often **rebuild endpoints that already exist** because they don't know what API contracts the project already has — leading to duplication & inconsistency.
- There is no lightweight, self-hosted, free tool designed so that agents can **query & register API contracts programmatically** before writing new code.

### 1.2 Solution

An API contract registry that:

1. Can be CRUDed by agents via MCP tools.
2. Performs **RAG-based duplicate checking** every time a new contract is created, so the agent is aware if a similar endpoint already exists.
3. Supports both semantic search (RAG) and exact search (by ID/path).
4. Has a read-only UI for human review.
5. Can export to `openapi.yml` per project.
6. Self-hosted via a single Docker container, with one-time auth setup.

---

## 2. Goals & Non-Goals

### 2.1 Goals (v1)

- [ ] CRUD API contracts via MCP tools (create, read, update, delete, list).
- [ ] Group contracts per feature/module within a single project.
- [ ] Multi-project within a single instance/server.
- [ ] RAG semantic search + duplicate detection on create.
- [ ] Exact search by ID / path / method.
- [ ] Read-only web UI for browsing & searching.
- [ ] Export to OpenAPI 3.x YAML per project.
- [ ] Setup via Docker; initial super-admin seeded from environment on first boot.
- [ ] MCP server via HTTP/SSE transport, accessible by multiple agents concurrently.

### 2.2 Non-Goals (v1)

- ❌ Not an API client / does not execute HTTP requests to real APIs (unlike Postman).
- ❌ No CRUD from the UI (mutations happen through MCP tools under an authenticated actor).
- ❌ No field-level diff/version history per contract (the audit log records who changed what, not diffs).
- ❌ No real-time collaboration/notification between agents.

---

## 3. Target Users


| User                               | Needs                                                                                                                           |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| AI Coding Agent (Claude Code, etc.) | Query contracts before writing code, register new contracts after creating an endpoint, check for duplication                     |
| Developer/Human                    | Browse & review API contracts via UI, cross-check documentation, export OpenAPI for other purposes (e.g., generate client SDKs) |


---

## 4. Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Docker Container                    │
│                                                       │
│  ┌──────────────┐     ┌─────────────────────────┐   │
│  │  MCP Server   │     │      Web UI (HTML)       │   │
│  │  (HTTP/SSE)   │     │   Askama/Tera templates  │   │
│  └───────┬───────┘     └────────────┬────────────┘   │
│          │                          │                 │
│          └──────────┬───────────────┘                 │
│                      │                                 │
│           ┌──────────▼──────────┐                     │
│           │   Core Service      │                     │
│           │  (business logic)   │                     │
│           └──────────┬──────────┘                     │
│                      │                                 │
│      ┌───────────────┼────────────────┐               │
│      │               │                │               │
│ ┌────▼─────┐  ┌──────▼──────┐  ┌──────▼──────┐        │
│ │  SQLite   │  │ sqlite-vec  │  │ fastembed-rs │        │
│ │(relational)│  │  (vector)   │  │ (embedding)  │        │
│ └───────────┘  └─────────────┘  └──────────────┘        │
│                                                       │
└─────────────────────────────────────────────────────┘
         ▲
         │ OAuth 2.1 (authorization code + PKCE via browser)
         │
┌────────┴────────┐
│  AI Agent(s)     │  mcp.json: { type: http, url: http://host:port/mcp }
│  (Claude Code)   │
└──────────────────┘

```

### 4.1 Tech Stack


| Layer         | Choice                                                             |
| ------------- | ------------------------------------------------------------------- |
| Language      | Rust                                                                |
| Web framework | Axum                                                                |
| Database      | SQLite (WAL mode)                                                   |
| Vector search | sqlite-vec extension                                                |
| Embedding     | fastembed-rs (model: `all-MiniLM-L6-v2` or equivalent, local/offline) |
| MCP transport | HTTP + SSE                                                          |
| UI rendering  | Server-rendered HTML (Askama or Tera)                               |
| Auth          | OAuth 2.1 (browser login + PKCE), session cookies, users/roles in SQLite |
| Deployment    | Single Docker container                                             |


### 4.2 Design Principles

- **Storage layer abstracted via trait** (repository pattern) — so that if Postgres is needed in the future (scale-up), the implementation can be swapped without changing business logic.
- **Zero external API dependency** — embedding runs locally, no third-party API key required, truly can run offline/air-gapped.
- **Idempotent by design** — the MCP tool `create_contract` always checks similarity before insert, to prevent duplication without requiring explicit coordination between agents.

---

## 5. Data Model

### 5.1 `projects`


| Column       | Type        | Description                                   |
| ------------ | ----------- | --------------------------------------------- |
| id           | TEXT (UUID) | PK                                            |
| slug         | TEXT        | unique, used in MCP call (`project_slug`)     |
| name         | TEXT        | project name                                  |
| description  | TEXT        | optional                                      |
| created_at   | DATETIME    |                                               |


### 5.2 `groups` (grouping per feature/module)


| Column       | Type        | Description                                 |
| ------------ | ----------- | ------------------------------------------- |
| id           | TEXT (UUID) | PK                                          |
| project_id   | TEXT        | FK → [projects.id](http://projects.id)      |
| name         | TEXT        | e.g., "Auth", "User Management", "Payment"  |
| description  | TEXT        | optional                                    |


### 5.3 `contracts`


| Column               | Type        | Description                                   |
| -------------------- | ----------- | --------------------------------------------- |
| id                   | TEXT (UUID) | PK                                            |
| project_id           | TEXT        | FK → [projects.id](http://projects.id)        |
| group_id             | TEXT        | FK → [groups.id](http://groups.id), nullable  |
| method               | TEXT        | GET/POST/PUT/PATCH/DELETE                     |
| path                 | TEXT        | e.g., `/api/v1/users/{id}`                    |
| summary              | TEXT        | short description                             |
| description          | TEXT        | long description, optional                  |
| request_headers      | JSON        | headers schema                                |
| request_params       | JSON        | path/query params schema                      |
| request_body_schema  | JSON        | JSON Schema for body                          |
| request_example      | JSON        | example payload                               |
| response_schema      | JSON        | JSON Schema for response (per status code)    |
| response_example     | JSON        | example response                              |
| auth_type            | TEXT        | none/bearer/api_key/basic, etc.               |
| status               | TEXT        | draft/stable/deprecated                       |
| tags                 | JSON array  | for additional filtering                      |
| created_at           | DATETIME    |                                               |
| updated_at           | DATETIME    |                                               |


### 5.4 `contract_embeddings` (managed via sqlite-vec)


| Column       | Type    | Description                                 |
| ------------ | ------- | ------------------------------------------- |
| contract_id  | TEXT    | FK → [contracts.id](http://contracts.id)    |
| embedding    | FLOAT[] | vector from `summary + description + path`  |


### 5.5 `users` / `web_sessions` / `oauth_*` / `audit_log`


| Table            | Purpose                                                        |
| ---------------- | -------------------------------------------------------------- |
| `users`          | username, Argon2id password hash, role, active flag            |
| `web_sessions`   | hashed session cookie tokens with CSRF hash and expiry         |
| `oauth_clients`  | dynamically registered MCP clients and their redirect URIs     |
| `oauth_authorization_codes` / `oauth_access_tokens` / `oauth_refresh_tokens` | hashed, one-time or revocable OAuth grants |
| `audit_log`      | append-only record of every create/update/delete with its actor |


---

## 6. MCP Tools Specification

All tools (except `list_projects`) require a `project_slug` parameter.

### 6.1 `create_contract`

Creates a new contract. **Automatically runs a RAG similarity check** before insert.

**Input:**

```json
{
  "project_slug": "string",
  "group_name": "string (optional, auto-create group if it doesn't exist)",
  "method": "GET|POST|PUT|PATCH|DELETE",
  "path": "string",
  "summary": "string",
  "description": "string (optional)",
  "request_headers": "object (optional)",
  "request_params": "object (optional)",
  "request_body_schema": "object (optional)",
  "request_example": "object (optional)",
  "response_schema": "object",
  "response_example": "object (optional)",
  "auth_type": "string (optional)",
  "tags": ["string"],
  "force": "boolean (default false, bypass duplicate warning)"
}

```

**Output (if a similar contract is found, force=false):**

```json
{
  "status": "warning_similar_found",
  "similar_contracts": [
    { "id": "...", "method": "GET", "path": "/api/v1/users/{id}", "similarity": 0.91 }
  ],
  "message": "Similar contracts found. Resend with force=true if you still want to create."
}

```

**Output (success):**

```json
{ "status": "created", "contract_id": "uuid" }

```

### 6.2 `search_contract`

Hybrid search: semantic (RAG) + exact.

**Input:**

```json
{
  "project_slug": "string",
  "query": "string (natural language or path)",
  "search_mode": "semantic|exact|hybrid (default hybrid)",
  "group_name": "string (optional, filter)",
  "limit": "number (default 10)"
}

```

**Output:**

```json
{
  "results": [
    { "id": "...", "method": "GET", "path": "...", "summary": "...", "similarity": 0.87 }
  ]
}

```

### 6.3 `get_contract_by_id`

**Input:** `{ "project_slug": "string", "contract_id": "string" }` **Output:** full contract object.

### 6.4 `update_contract`

**Input:** `{ "project_slug": "string", "contract_id": "string", ...fields to update }` **Output:** `{ "status": "updated", "contract_id": "..." }` *(v1: overwrite directly, no history)*

### 6.5 `delete_contract`

**Input:** `{ "project_slug": "string", "contract_id": "string" }` **Output:** `{ "status": "deleted" }`

### 6.6 `list_groups`

**Input:** `{ "project_slug": "string" }` **Output:** `{ "groups": [{ "id": "...", "name": "...", "contract_count": 12 }] }`

### 6.7 `list_contracts`

**Input:** `{ "project_slug": "string", "group_name": "string (optional)", "status": "string (optional)" }` **Output:** array of contract summaries (without full schema, for compactness).

### 6.8 `list_projects`

**Input:** `{}` **Output:** `{ "projects": [{ "slug": "...", "name": "...", "contract_count": 42 }] }`

### 6.9 `create_project`

**Input:** `{ "slug": "string", "name": "string", "description": "string (optional)" }` **Output:** `{ "status": "created", "project_slug": "..." }`

### 6.10 `export_openapi`

**Input:** `{ "project_slug": "string" }` **Output:** `{ "yaml": "...", "download_url": "/projects/{slug}/openapi.yml" }`

---

## 7. HTTP Endpoints (UI & Misc)


| Endpoint                          | Method   | Description                                          |
| --------------------------------- | -------- | ---------------------------------------------------- |
| `/mcp`                            | POST/SSE | MCP server endpoint (requires OAuth access token)    |
| `/`                               | GET      | Dashboard: list all projects (requires login)        |
| `/projects/{slug}`                | GET      | List groups & contracts within a project             |
| `/projects/{slug}/contracts/{id}` | GET      | Contract details incl. per-contract audit history    |
| `/projects/{slug}/search?q=...`   | GET      | Search UI (uses `search_contract` behind the scenes) |
| `/projects/{slug}/openapi.yml`    | GET      | Download OpenAPI export                              |
| `/login`, `/logout`               | GET/POST | Browser login and session logout                     |
| `/.well-known/oauth-*`            | GET      | MCP OAuth discovery metadata                         |
| `/oauth/register`                 | POST     | Dynamic client registration (public PKCE clients)    |
| `/oauth/authorize`, `/oauth/consent` | GET/POST | Browser authorization and consent screen          |
| `/oauth/token`                    | POST     | Authorization-code / refresh-token exchange          |
| `/admin/users`                    | GET/POST | Super-admin user management (viewer/editor/admin)    |
| `/admin/audit`                    | GET      | Super-admin global audit log viewer                  |
| `/healthz`, `/readyz`             | GET      | Liveness / readiness probes                          |


---

## 8. Auth & Setup Flow

1. First `docker run` → server checks whether the `users` table is empty.
2. If empty → the initial super-admin is created from `APP__AUTH__INITIAL_ADMIN_USERNAME` and `APP__AUTH__INITIAL_ADMIN_PASSWORD`. On an empty database, both variables are **required**; once any user exists they are ignored (existing users are never overwritten).
3. The human logs into the web UI at `/login`; sessions are opaque HttpOnly cookies backed by hashed rows in `web_sessions`.
4. MCP clients use OAuth 2.1: discovery via `/.well-known/oauth-protected-resource`, dynamic client registration, browser login + consent, then authorization-code + PKCE exchange at `/oauth/token`. `.mcp.json` needs only `type` and `url`.
5. Scopes are capped by role (`ledgapi:read` for all, `ledgapi:write` for editor+, `ledgapi:admin` for super-admin). Every successful create/update/delete is appended to `audit_log` with its acting user; reads, logins, and failed writes are not audited.

---

## 9. RAG & Duplicate Detection — Detailed Flow

1. When `create_contract` is called, the system generates an embedding from `method + path + summary + description`.
2. Query the top-K (e.g., K=5) nearest contracts within the same project via `sqlite-vec`.
3. If the highest similarity > threshold (default **0.85**, configurable via env var `SIMILARITY_THRESHOLD`) and `force != true` → return a warning, **do not insert**.
4. If `force == true` or the similarity is below the threshold → insert the contract + its embedding.
5. `search_contract` in semantic mode uses the same query mechanism, without the threshold block (all results are returned with their similarity score, sorted desc).

---

## 10. Roadmap

### v1 (MVP — scope of this PRD)

- CRUD contracts via MCP, grouping, multi-project
- RAG duplicate check + semantic search
- Login-protected web UI + OpenAPI export
- OAuth 2.1 browser login for MCP clients (PKCE, consent)
- Multi-user roles (viewer / editor / super-admin) seeded from env on first boot
- Append-only audit log of all mutations with actor
- Docker single container

### v2 (candidates, not yet scoped in detail)

- Field-level diff/version history per contract
- Import from existing OpenAPI yml (reverse — populate the registry from an existing spec)
- Webhook/notification when a contract changes (e.g., breaking change alert)
- CLI companion tool (alongside MCP) for humans who want to CRUD without going through an agent

---

## 11. Open Questions (need to be decided before development starts)

1. ~~**UI auth**~~ — resolved: all web UI routes require a session; `/admin/*` additionally requires super-admin.
2. **Embedding model & language**: will contracts be dominated by English (technical), or do we also need a model more friendly to Indonesian? This affects the choice of embedding model.
3. **Breaking change detection**: when `update_contract` significantly changes `response_schema`, should there be an automatic warning to the agent (similar to duplicate check), or just overwrite without validation in v1?
4. **Rate limit / resource guard**: since accessed by many agents concurrently via HTTP, do we need a per-token request limit to prevent one agent from "spamming" create/search excessively?
5. **Tool/project name**: we need a product name for branding, docs, and Docker image name.

---