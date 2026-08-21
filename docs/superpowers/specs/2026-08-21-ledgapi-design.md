# Ledgapi — Design Spec

**Date:** 2026-08-21
**Status:** Draft for review
**Source PRD:** `../../PRD.md` (v1 draft, 2026-08-21)
**Product name:** Ledgapi — *API contracts, remembered by your agents.*

---

## 1. Purpose

A self-hosted, agent-native **API contract registry**. AI coding agents (Claude Code et al.) interact through MCP to **query** what endpoints already exist before writing code and **register** new endpoints after creating them. A read-only web UI lets humans review. Single Docker container, single binary, offline-capable (local embedding model).

This spec covers **v1**. v2 candidates (audit history, OAuth, multi-user, OpenAPI import, webhooks) are out of scope.

---

## 2. Resolved decisions (from PRD §11 + brainstorming)

| # | Question | Decision |
|---|---|---|
| 1 | UI auth | **No auth on UI** (local-network trust). MCP endpoint requires bearer token. `/setup` is the only stateful web route. |
| 2 | Embedding model & language | **English-only** with `sentence-transformers/all-MiniLM-L6-v2` via fastembed-rs. ~25MB on first boot. |
| 3 | Breaking-change detection on `update_contract` | **Overwrite silently.** No diff, no warning in v1. |
| 4 | Rate limit | **None.** Single Docker container, single admin token, single-user. |
| 5 | Product / tool / Docker name | **Ledgapi** (this spec, binary, container image, MCP server name). |
| — | MCP transport | **Streamable HTTP** (modern 2025-03-26 spec). Legacy HTTP+SSE dropped. |
| — | Create-contract similarity output | Returns top-K matches (PRD §9 K=5). Agent decides: `update_contract` one of them **or** `create_contract` again with `force=true`. |

---

## 3. Architecture

### 3.1 Process topology

```
┌──────────────────────────────────────────────────────────────────┐
│                   ledgapi binary (Axum)                          │
│                                                                    │
│   POST /mcp ──► auth mw ──► Streamable HTTP MCP server             │
│                              ├─ initialize / tools/list            │
│                              └─ tools/call → use_case → port → repo│
│                                                                    │
│   GET  /, /projects/*, /openapi.yml ──► web router (no auth)       │
│   GET  /setup                    ──► setup handler (no auth, TTL)  │
│   GET  /healthz, /readyz         ──► health probes (no auth)        │
│                                                                    │
│   AppState = { pool, embedder, token_repo, cfg }                   │
└──────────────────────────────────────────────────────────────────┘
         │                                       │
         ▼                                       ▼
┌────────────────────┐                  ┌─────────────────────┐
│ SQLite (WAL)       │                  │ sqlite-vec          │
│ + auth_tokens      │                  │ (vec0 virtual table)│
│ + projects/groups/ │                  └─────────────────────┘
│   contracts        │
└────────────────────┘                          │
                                                ▼
                                       ┌─────────────────────┐
                                       │ FastembedEmbedder   │
                                       │ (all-MiniLM-L6-v2,  │
                                       │  spawn_blocking)    │
                                       └─────────────────────┘
```

### 3.2 Module layout (single crate, layered)

```
src/
├── main.rs                        # composition root entry
├── lib.rs                         # re-exports + run()
├── config.rs                      # AppConfig (env-driven)
├── state.rs                       # AppState { pool, embedder, token_repo, cfg }
├── errors.rs                      # AppError + IntoResponse
├── telemetry.rs                   # tracing-subscriber init
│
├── core/                          # pure types — no I/O, no async
│   ├── id.rs                      # UUIDv7 Id + IdGenerator + SystemIdGenerator
│   ├── clock.rs                   # Clock trait + SystemClock / FixedClock
│   ├── observability.rs           # MetricsRecorder / RequestSpan facade traits
│   └── envelope.rs                # ApiResponse, ApiError JSON envelope
│
├── domain/                        # business types + use cases (no infra deps)
│   ├── contract.rs                # Contract, Method, Status enums + DTOs
│   ├── project.rs                 # Project, ProjectSlug
│   ├── group.rs                   # Group
│   ├── ports.rs                   # Repository traits + Embedder trait
│   ├── errors.rs                  # DomainError taxonomy
│   └── use_cases/
│       ├── create_contract.rs     # dup-check → write
│       ├── search_contract.rs     # hybrid RRF
│       ├── update_contract.rs
│       ├── delete_contract.rs
│       ├── manage_project.rs
│       ├── manage_group.rs
│       ├── bootstrap_token.rs     # first-run token generation
│       └── export_openapi.rs
│
├── infra/                         # adapters — no business rules
│   ├── db/
│   │   ├── pool.rs                # rusqlite + sqlite-vec loader
│   │   └── migrations.rs          # apply migrations/*.sql via include_str!
│   ├── repos/
│   │   ├── project_repo_sqlite.rs
│   │   ├── group_repo_sqlite.rs
│   │   ├── contract_repo_sqlite.rs
│   │   ├── embedding_repo_sqlite_vec.rs
│   │   └── token_repo_sqlite.rs
│   └── embeddings/
│       └── fastembed_impl.rs      # FastembedEmbedder impls domain::ports::Embedder (spawn_blocking)
│   └── auth/
│       ├── token.rs               # generate, sha256-hex
│       └── middleware.rs          # Bearer enforcement for /mcp only
│
├── mcp/                           # Streamable HTTP MCP server
│   ├── server.rs                  # POST /mcp handler + JSON-RPC dispatch
│   ├── tools.rs                   # Tool trait + registry
│   └── tools/                     # one file per tool (delegate to use_cases)
│       ├── create_project.rs
│       ├── list_projects.rs
│       ├── create_contract.rs
│       ├── get_contract_by_id.rs
│       ├── update_contract.rs
│       ├── delete_contract.rs
│       ├── list_groups.rs
│       ├── list_contracts.rs
│       ├── search_contract.rs
│       └── export_openapi.rs
│
└── web/                           # askama templates + axum handlers
    ├── router.rs
    ├── handlers.rs                # thin: parse → use_case → render
    ├── openapi_export.rs          # Contract → OpenAPI 3.1 YAML
    └── templates/                 # see §6
```

### 3.3 Layering rules (enforced by `tests/architecture.rs`)

| Module | May import | Must NOT import |
|---|---|---|
| `core/` | std, serde, uuid, time, thiserror | anything else |
| `domain/` | `core/`, serde | `infra/`, `mcp/`, `web/`, axum, rusqlite, fastembed |
| `infra/` | `core/`, `domain/`, rusqlite, fastembed, axum (middleware only) | `mcp/`, `web/`, business rules |
| `mcp/` | `core/`, `domain/` (and `infra/` only via `domain::ports` traits) | direct SQL, fastembed, axum types |
| `web/` | `core/`, `domain/`, `infra/`, askama, axum | direct fastembed, direct SQL |
| `main.rs` / `lib.rs` | everything (composition root) | business logic |

`archaven.toml` enforces workspace-wide forbidden deps (no `unsafe`, no wildcard deps, no `lazy_static`/`OnceCell`, ban specific transitive crates). `tests/architecture.rs` is the fast smoke for module-level boundaries.

---

## 4. Data model

### 4.1 Schema (SQLite + WAL, applied on boot)

```sql
-- 0001_init.sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;

CREATE TABLE projects (
    id          TEXT PRIMARY KEY,        -- UUIDv7
    slug        TEXT NOT NULL UNIQUE,    -- MCP project_slug
    name        TEXT NOT NULL,
    description TEXT,
    created_at  INTEGER NOT NULL
);

CREATE TABLE groups (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT,
    UNIQUE(project_id, name)
);

CREATE TABLE contracts (
    id                  TEXT PRIMARY KEY,
    project_id          TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    group_id            TEXT     REFERENCES groups(id) ON DELETE SET NULL,
    method              TEXT NOT NULL CHECK (method IN ('GET','POST','PUT','PATCH','DELETE')),
    path                TEXT NOT NULL,
    summary             TEXT NOT NULL,
    description         TEXT,
    request_headers     TEXT,                  -- JSON
    request_params      TEXT,                  -- JSON
    request_body_schema TEXT,                  -- JSON
    request_example     TEXT,                  -- JSON
    response_schema     TEXT NOT NULL,         -- JSON
    response_example    TEXT,                  -- JSON
    auth_type           TEXT,
    status              TEXT NOT NULL DEFAULT 'draft'
                        CHECK (status IN ('draft','stable','deprecated')),
    tags                TEXT NOT NULL DEFAULT '[]',
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    UNIQUE (project_id, method, path)
);

CREATE INDEX contracts_project_idx ON contracts(project_id);
CREATE INDEX contracts_group_idx   ON contracts(group_id);
CREATE INDEX contracts_status_idx  ON contracts(project_id, status);

CREATE TABLE auth_tokens (
    token_hash  TEXT PRIMARY KEY,        -- sha256 hex
    label       TEXT,
    created_at  INTEGER NOT NULL
);

-- 0002_contract_embeddings.sql
CREATE VIRTUAL TABLE contract_embeddings USING vec0(
    contract_id TEXT PRIMARY KEY,
    project_id  TEXT,                     -- filtered KNN
    embedding   float[384]                -- MiniLM dim
);
```

SQLite timestamps are **unix epoch seconds** (integer). JSON columns are stored as `TEXT` and serialized via `serde_json`.

### 4.2 Embedding pipeline

- Embedding input: `format!("{method} {path} {summary} {description}")`.
- `Embedder` trait in `domain/ports.rs`:
  ```rust
  #[async_trait]
  pub trait Embedder: Send + Sync {
      async fn embed(&self, text: &str) -> Result<Vec<f32>, DomainError>;
      fn dimension(&self) -> usize; // 384
  }
  ```
- `FastembedEmbedder` (infra) wraps fastembed-rs (sync). All `embed()` calls go through `tokio::task::spawn_blocking` internally.
- Model cache: `APP__EMBED__CACHE_DIR` (default `/data/.cache/fastembed` in Docker). Lazy-loaded on first `embed()` call (~10–30s on cold start).
- ONNX runtime uses 1–2 CPU threads; container should be sized accordingly.

### 4.3 `create_contract` flow

```
1. Validate input (method enum, path starts with /, summary non-empty,
   response_schema parses as JSON)
2. Resolve project_id from project_slug (404 if missing)
3. Resolve-or-create group_id from group_name (if provided)
4. Compute embedding for (method, path, summary, description)
   └─ on Embedder error → return DomainError::EmbeddingUnavailable (503)
5. semantic_search(project_id, embedding, K=APP__EMBED__KNN_TOP_K)
   → top-K candidates with similarity in [0,1], EXCLUDING any exact
     (method, path) match (the candidate wouldn't exist yet, but the
     branch filters for safety in concurrent insert races)
6. If max_similarity >= APP__EMBED__SIMILARITY_THRESHOLD (default 0.85)
   AND force != true:
       return { status: "warning_similar_found",
                similar_contracts: top-K (with similarity, sorted desc),
                message: "Similar contracts found. Call update_contract on
                          a match, or resend with force=true to create anyway." }
       ⚠ No row is inserted.
7. else: INSERT contract + embedding in a single transaction
   ├─ UNIQUE(project_id, method, path) violation (rare race with a
   │   concurrent insert that won the semantic check first) →
   │   rollback, return DomainError::DuplicateKey { existing_id } (409)
   └─ success → return { status: "created", contract_id }
```

`force=true` skips step 6 but **still returns** the similar_contracts array so the agent has full context.

### 4.4 Hybrid search (RRF, k=60)

`search_contract(project_slug, query, mode, group_name, limit)`:

| `mode` | Behavior |
|---|---|
| `exact` | Branch A only |
| `semantic` | Branch B only |
| `hybrid` (default) | A + B → RRF merge |

- **Branch A (exact):**
  ```sql
  SELECT id, method, path, summary, status
    FROM contracts
   WHERE project_id = ?1
     AND (path LIKE '%' || ?2 || '%' OR summary LIKE '%' || ?2 || '%')
     AND (?3 IS NULL OR group_id = ?3)
   ORDER BY CASE WHEN path = ?2 THEN 0 ELSE 1 END ASC,
            updated_at DESC
   LIMIT 50;
  ```
- **Branch B (semantic):** embed query, then KNN via sqlite-vec, joined back to `contracts` to filter by `project_id` and `group_id`:
  ```sql
  SELECT ce.contract_id, ce.distance
    FROM contract_embeddings AS ce
    JOIN contracts      AS c  ON c.id = ce.contract_id
   WHERE ce.embedding MATCH ?1
     AND c.project_id = ?2
     AND (?3 IS NULL OR c.group_id = ?3)
   ORDER BY ce.distance
   LIMIT 50;
  ```
  Similarity = `1.0 - distance`.
- **RRF merge:** for each contract_id in union, `score = 1/(60 + rank_a) + 1/(60 + rank_b)` (missing list → term omitted). Sort desc, take top `limit`.
- Hydrate summaries from `contracts` table.

---

## 5. MCP server

### 5.1 Wire format

```
POST /mcp
Authorization: Bearer <token>            (required)
Content-Type:   application/json
Accept:         application/json, text/event-stream

Body: JSON-RPC 2.0 request

Response (default, our tools don't stream):
    Content-Type: application/json
    { jsonrpc: "2.0", id, result }  or  { jsonrpc: "2.0", id, error }

Response (only when client prefers text/event-stream):
    Content-Type: text/event-stream
    event: message
    data: { jsonrpc: "2.0", id, result }
```

No `Mcp-Session-Id` — every tool call is stateless. The transport still respects `Accept: text/event-stream` by wrapping the JSON response in a single SSE event.

### 5.2 JSON-RPC methods

| method | Direction | Behavior |
|---|---|---|
| `initialize` | client → server | Returns `{ protocolVersion: "2025-06-18", serverInfo: { name: "ledgapi", version }, capabilities: { tools: {} } }` |
| `notifications/initialized` | client → server | 202 Accepted, no body |
| `tools/list` | client → server | All 10 tools with inputSchema (JSON Schema via schemars) |
| `tools/call` | client → server | Dispatch by `params.name`, validate, execute |
| _other_ | client → server | -32601 Method not found |

### 5.3 Tool surface

Each tool is a thin struct (`CreateContractTool`, etc.) implementing `mcp::tools::Tool`. Tools delegate to `domain::use_cases::*` — they never touch SQL.

| Tool | Use case | Notes |
|---|---|---|
| `create_project` | `manage_project::create` | |
| `list_projects` | `manage_project::list` | |
| `create_contract` | `create_contract::execute` | Returns warning on dup |
| `get_contract_by_id` | `create_contract::get` | |
| `update_contract` | `update_contract::execute` | Silent overwrite |
| `delete_contract` | `delete_contract::execute` | |
| `list_groups` | `manage_group::list` | |
| `list_contracts` | `create_contract::list` | Optional `group_name`, `status` filter; `limit` (default 100, max 500) |
| `search_contract` | `search_contract::execute` | `mode = exact|semantic|hybrid` |
| `export_openapi` | `export_openapi::execute` | Returns YAML string + download URL |

### 5.4 Error mapping

| DomainError | JSON-RPC code | `data.code` | HTTP |
|---|---|---|---|
| `Validation { field, message }` | -32602 | `validation_failed` | 400 |
| `NotFound` | -32602 | `not_found` | 404 |
| `DuplicateKey` | -32602 | `duplicate_key` | 409 |
| `SimilarFound` | (success result, status `warning_similar_found`) | n/a | 200 |
| `Auth(Missing)` | (middleware rejects) | — | 401 |
| `Auth(Invalid)` | (middleware rejects) | — | 403 |
| `Internal` | -32603 | `internal_error` | 500 |
| `EmbeddingUnavailable` | -32603 | `service_unavailable` | 503 |

`SimilarFound` is **not** a JSON-RPC error — the agent reads the candidates and decides.

---

## 6. Web UI

### 6.1 Routes (all unauthenticated)

| Method | Path | Template |
|---|---|---|
| GET | `/` | `dashboard.html` |
| GET | `/projects/{slug}` | `project.html` |
| GET | `/projects/{slug}/contracts/{id}` | `contract.html` |
| GET | `/projects/{slug}/search?q=&mode=&group=` | `search.html` |
| GET | `/projects/{slug}/openapi.yml` | (none, attachment download) |
| GET | `/setup` | `setup.html` (or 410 Gone) |
| GET | `/healthz` | (none, 200 OK) |
| GET | `/readyz` | (none, 200 OK or 503) |
| GET | `/*` | `404.html` |

### 6.2 Visual philosophy

- Server-rendered HTML via **Askama**, one CSS file (`templates/style.css`).
- System font stack, no web fonts.
- Single accent color (oklch 60% 0.15 250), method/status color badges.
- Max-width 1100px, generous spacing, no nested-card soup.
- Vanilla JS only for: search debounce, copy-to-clipboard on `/setup`, collapsible schema sections.
- Mobile-friendly, desktop-first.

### 6.3 `/setup` lifecycle

State lives in `AppState.setup` (atomic bool + `Instant`):

```
First boot:
  - generate 32-byte token → 64 hex chars
  - insert sha256(token_hash) into auth_tokens
  - log "LEDGAPI_BOOTSTRAP_TOKEN=<token>" to stdout (Docker captures this)
  - setup.active = true; setup.expires_at = Instant::now() + 5min
  - GET /setup renders token + "save this now" instructions

Subsequent boot:
  - setup.active = false from the start
  - GET /setup → 410 Gone with explanation

Clear conditions (whichever fires first):
  - First valid POST /mcp call: setup.active → false
  - On every GET /setup request, if Instant::now() >= setup.expires_at:
    setup.active → false, then return 410 Gone (TTL is checked lazily,
    no background task needed)
```

---

## 7. Auth & configuration

### 7.1 Token format

- 32 random bytes from `rand::thread_rng()` → 64 lowercase hex chars.
- Stored as `sha256(token)` hex (never plaintext in DB or logs).
- Header parse: `Authorization: Bearer <64-hex>` — `strip_prefix("Bearer ")` then hex-length check.
- DB-side compare is **constant-time** (`subtle::ConstantTimeEq` or equivalent) against the stored hash; SQL query is parameterized.

### 7.2 Env vars (loaded via `config` crate, prefix `APP__`)

```bash
APP__SERVER__BIND=0.0.0.0:8080
APP__SERVER__SHUTDOWN_TIMEOUT=30s

APP__DATABASE__PATH=/data/ledgapi.db
APP__DATABASE__BUSY_TIMEOUT_MS=5000

APP__EMBED__CACHE_DIR=/data/.cache/fastembed
APP__EMBED__MODEL=sentence-transformers/all-MiniLM-L6-v2
APP__EMBED__SIMILARITY_THRESHOLD=0.85
APP__EMBED__KNN_TOP_K=5
APP__EMBED__HYBRID_LIMIT=10

APP__LOG__FORMAT=pretty                # pretty | json
APP__LOG__LEVEL=info,ledgapi=debug
```

`.env.example` ships these; `docker-compose.yaml` overrides the runtime-relevant ones.

---

## 8. Docker

### 8.1 Dockerfile (multi-stage)

- Stage 1: `rust:1.97-bookworm`. Cache deps by faking `main.rs` first, then real source, then `cargo build --release --locked --bin ledgapi && strip`.
- Stage 2: `debian:bookworm-slim`. Non-root user `app` (uid 10001), `/data` volume, `tini` as PID 1, `HEALTHCHECK` on `/healthz` with `--start-period=40s` (covers first-boot model download).
- Binary + `migrations/` + `templates/` + `docker/entrypoint.sh` copied in.

### 8.2 docker-compose.yaml

```yaml
services:
  ledgapi:
    build: .
    image: limaxs/ledgapi:0.1.0
    container_name: ledgapi
    restart: unless-stopped
    ports: ["8080:8080"]
    volumes: ["ledgapi-data:/data"]
    environment:
      APP__DATABASE__PATH: /data/ledgapi.db
      APP__EMBED__CACHE_DIR: /data/.cache/fastembed
      APP__LOG__FORMAT: json
volumes:
  ledgapi-data:
```

`entrypoint.sh`: `mkdir -p /data && chown app:app /data && exec /usr/local/bin/ledgapi`.

---

## 9. Tooling inherited from rust-starter

| File | Status |
|---|---|
| `rust-toolchain.toml` (1.97, edition 2024) | Adopted |
| `rustfmt.toml` | Adopted |
| `.editorconfig` | Adopted |
| Workspace lints (`unsafe_code = "forbid"`, pedantic + allows) | Adopted |
| `Cargo.toml` `[workspace.package]` + `[workspace.dependencies]` | Adapted (single crate) |
| Release/dev/test profiles | Adopted |
| `deny.toml` (license allow-list, registry rules) | Adopted |
| `Makefile` (`fmt`, `fmt-check`, `clippy`, `test`, `deny`, `ci`, `archaven`) | Adapted (drop sqlx, compose, e2e targets) |
| `CONTRIBUTING.md` (Conventional Commits, code style) | Adopted |
| `archaven.toml` | Adapted (workspace-wide forbidden deps) |
| `Id` (UUIDv7) + `IdGenerator` + `SystemIdGenerator` | Adopted |
| `Clock` trait + `SystemClock` / `FixedClock` | Adopted |
| `MetricsRecorder` / `RequestSpan` facade traits | Adopted (no Prometheus exporter in v1) |
| Response envelope `{success, code, message, data, errors[]}` | Adopted |
| `CoreError` / `CoreErrorCode` taxonomy | Adapted (drop tenant codes; add auth + duplicate + similar) |
| `AppError` + `IntoResponse` shape | Adopted |
| `TestApp` builder (in-memory router for tests) | Adopted |
| `tests/architecture.rs` pattern | Adopted |

**Dropped** (Postgres/Redis/PASETO/multi-crate world): sqlx, redis, rusty_paseto, argon2, sqlx-cli, multi-crate workspace, CRUD kernel.

---

## 10. Testing strategy

### 10.1 Four layers

1. **Unit tests** in each module: domain validation, error mapping tables, pure logic (RRF math, threshold compare).
2. **Architecture tests** (`tests/architecture.rs`): `cargo_metadata`-based module-boundary assertions. ~2s.
3. **Integration tests** via `TestApp` + `StubEmbedder` (deterministic vector from text hash): every MCP tool happy + failure path, dup-check threshold cases, `/setup` lifecycle, auth middleware, OpenAPI export roundtrip.
4. **Live tests** (`#[ignore]`-gated, real SQLite + real fastembed MiniLM): first-run token, embedding roundtrip, sqlite-vec KNN correctness, golden-file OpenAPI export. CI nightly only.

### 10.2 Must-cover list

- Every `DomainError` variant → unit test
- All 10 MCP tools → at least one happy-path + one failure-path test
- Dup-check flow: 0.84 (pass), 0.86 (warn), exact-dup UNIQUE conflict, `force=true`
- Hybrid search: each mode exercised separately
- `/setup`: first-run shows token, after-setup returns 410
- Auth middleware: missing/invalid/valid tokens
- OpenAPI export: YAML roundtrips through a parser
- Architecture: `tests/architecture.rs` asserts all module boundaries
- `make clippy` + `make deny` clean

### 10.3 Test fixtures

```
tests/fixtures/
├── contracts_users_api.json     # 4 contracts
├── contracts_auth_api.json      # 3 contracts
├── golden_openapi.yml           # expected export for users fixture
└── README.md
```

---

## 11. CI / local gates

```makefile
ci: fmt-check clippy test architecture deny archaven
```

- `fmt-check` — `cargo fmt --all -- --check`
- `clippy` — `cargo clippy --all-targets --all-features -- -D warnings`
- `test` — `cargo test --all-features`
- `architecture` — `cargo test --test architecture`
- `deny` — `cargo deny check`
- `archaven` — `archaven check`

Live tests are opt-in: `cargo test -- --ignored`.

---

## 12. Open follow-ups (not blockers)

- **OpenAPI export viewer:** a tiny YAML pretty-printer in the web UI is a nice-to-have, not in v1.
- **Metrics endpoint:** tracing-only in v1. `metrics-exporter-prometheus` is already a workspace dep option; add when there's a need.
- **Token rotation:** v2 per PRD §10. Manual workaround is to drop the row from `auth_tokens` (or wipe the volume).
- **`list_contracts` pagination:** v1 caps at `limit=500`; offset-based pagination can come in v2 if needed.

---

## 13. Implementation defaults (locked during self-review)

These are decisions that the spec leaves implicit. Locked here so the implementation plan can reference them without re-asking.

| # | Topic | Decision | Rationale |
|---|---|---|---|
| 1 | DB connection | **`Arc<Mutex<rusqlite::Connection>>`** (no pool) | SQLite WAL already serializes writes. A pool adds no concurrency benefit at our scale and complicates the code. Each request acquires the mutex briefly. |
| 2 | MCP body size limit | **4 MiB** (`axum::extract::DefaultBodyLimit::max(4 * 1024 * 1024)`) | API contracts with large JSON schemas can run ~100 KiB – 1 MiB. 4 MiB gives headroom without DoS risk. |
| 3 | `update_contract` embedding | **Regenerate on every update** that touches `method`, `path`, `summary`, or `description` | Otherwise semantic search returns stale matches after a meaningful edit. One extra embed call is cheap. |
| 4 | Concurrent insert DuplicateKey payload | **Return `"DuplicateKey"` with the conflicting `(method, path)` only — do not expose `existing_id`** | Agent calls `list_contracts` or `search_contract` to discover the existing contract. Smaller error surface. |
| 5 | Empty `query` in `search_contract` | **Reject with `Validation { field: "query", message: "must be non-empty" }`** | Empty query is meaningless; fail fast. |
| 6 | Path normalization | **Trim trailing slash except for root `/`. Case-sensitive. No other normalization.** | REST convention; predictable. |
| 7 | OpenAPI version | **3.0.3** (not 3.1) | Wider tool compatibility; most client generators default to 3.0. Migration to 3.1 is a v2 concern. |
| 8 | Group name comparison | **Case-sensitive** (e.g., `Auth` ≠ `auth`) | Predictable. Agents normalize client-side if they want. |
| 9 | `APP__DATABASE__PATH` parent dir | **Auto-create on boot if missing** (with `app:app` ownership in Docker) | Smoother Docker volume-mount experience. |
| 10 | OpenAPI `info` fields | **Per-project.** `title=project.name`, `description=project.description`, `version="1.0.0"` | No project versioning in v1; a single static version is honest. |
| 11 | OpenAPI auth mapping | **Map `auth_type` to OpenAPI securitySchemes:** `none` → no security; `bearer` → `bearerAuth` (`type: http, scheme: bearer`); `api_key` → `apiKeyAuth` (`in: header, name: X-API-Key`); `basic` → `basicAuth` (`type: http, scheme: basic`). Unknown → treated as none. | Documented static map. |
| 12 | OpenAPI parameter location inference | **`{name}` patterns in `path` ⇒ path param. Everything in `request_params` JSON Schema ⇒ query param. Body ⇒ from `request_body_schema`. Headers ⇒ from `request_headers`.** If `request_params` items include an explicit `"in"` field, that overrides. | Heuristic that matches 95% of REST APIs without forcing agents to author OpenAPI-specific metadata. |
| 13 | Dashboard sort order | **`projects.created_at DESC`** | Most recent first; matches the "what did I just create?" workflow. |
| 14 | JSON-RPC notification response status | **`204 No Content` with empty body** | HTTP-idiomatic; per JSON-RPC spec notifications get no JSON-RPC response. |
| 15 | Embedding cache (repeated text) | **None in v1.** fastembed-rs re-encodes each call. | Avoids cache-key complexity for negligible savings at our scale. |
| 16 | Tag filtering | **Not exposed in v1** (tags stored on contracts but no `tag` filter on `list_contracts` or `search_contract`). | Out of PRD §6.7/6.2; deferred to v2. |
| 17 | `/setup` page parent dir auto-create | **Auto-create `/data` on boot** (entrypoint.sh handles Docker; the binary creates `APP__DATABASE__PATH`'s parent dir). | Matches #9. |
| 18 | Malformed JSON in MCP request body | **Return JSON-RPC `-32700` (Parse error)** with no body. | Standard JSON-RPC behavior. |
| 19 | Malformed UUID in path | **Return HTTP 404 with 404.html.** | Treat as "doesn't exist" rather than a 400 — keeps URL enumeration behavior consistent. |
| 20 | `export_openapi` per-contract `tags` | **Promoted to OpenAPI `tags: [...]` on the operation**, not the top-level `tags:`. | Lets SDK generators group operations correctly. |

---

## 14. Out of scope (v1)

- Auditing / version history
- OAuth / multi-user
- OpenAPI import (reverse direction)
- Webhooks / breaking-change notifications
- CLI companion tool
- Non-MiniLM embedding models
- Rate limiting
- Per-token access (single global admin token only)
- PostgreSQL backend (storage trait is in place but no adapter)
- HA / multi-instance (SQLite is single-writer; one process at a time)