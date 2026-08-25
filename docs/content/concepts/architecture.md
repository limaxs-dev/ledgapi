---
title: Architecture
description: The single-container layout, the request lifecycle, and the storage traits that keep the core service decoupled from SQLite.
---

ledgapi is one process inside one container. There is no queue, no external cache, and no third-party API call. SQLite handles both the relational data and the vector index, and `fastembed-rs` runs the embedding model in-process.

## Stack

| Layer | Choice |
|---|---|
| Language | Rust |
| Web framework | Axum |
| Database | SQLite (WAL mode) |
| Vector search | `sqlite-vec` extension |
| Embedding | `fastembed-rs`, model `all-MiniLM-L6-v2` |
| MCP transport | HTTP + SSE |
| UI rendering | Server-rendered HTML through Askama |
| Auth | OAuth 2.1 (browser login + PKCE), session cookies, users in SQLite |
| Deployment | Single Docker container |

## Layout

```text
+-----------------------------------------------+
|  Docker container                             |
|                                               |
|  +---------------+  +-----------------------+ |
|  |  MCP server   |  |   Web UI (HTML)       | |
|  |  HTTP + SSE   |  |   Askama templates    | |
|  +-------+-------+  +-----------+-----------+ |
|          |                      |             |
|          +---------+------------+             |
|                    |                          |
|            +-------v--------+                 |
|            |  Core service  |                 |
|            |  business logic|                 |
|            +-------+--------+                 |
|                    |                          |
|       +------------+-------------+            |
|       |            |             |            |
|  +----v-----+ +-----v-----+ +-----v-----+     |
|  |  SQLite  | | sqlite-vec| | fastembed |     |
|  | relational| |  vectors  | |  on-host  |     |
|  +----------+ +-----------+ +-----------+     |
+-----------------------------------------------+
                  ^
                  |  OAuth 2.1 (authorization code + PKCE)
                  |
        +---------+-----------+
        |  AI agent           |
        |  Claude Code, etc.  |
        +---------------------+
```

## Request lifecycle

For a write through MCP, the steps are:

1. The agent sends a request to `/mcp` with a bearer token. The OAuth middleware validates the token and loads the user and their role.
2. The MCP router dispatches the call to the matching tool handler.
3. The handler validates the input, then calls the core service.
4. For `create_contract`, the core service first asks the embedding layer for a vector. The vector is built from `method + path + summary + description` through `fastembed-rs`.
5. The service queries the top five similar contracts in the same project through `sqlite-vec`.
6. If the highest similarity is above the threshold and `force` is not set, the handler returns `warning_similar_found` and the database is not touched.
7. Otherwise, the contract row is inserted and its embedding is written to the `contract_embeddings` table. The audit log is appended.
8. The handler returns the contract id.

Reads skip steps 4 to 7 and are not logged.

## Storage trait

The core service does not touch SQLite directly. Every call goes through a trait so the implementation can be swapped without changing the business logic.

```rust
#[async_trait]
pub trait ContractRepository: Send + Sync {
    async fn create(&self, contract: NewContract) -> Result<ContractId, RepoError>;
    async fn get(&self, id: ContractId) -> Result<Contract, RepoError>;
    async fn list(&self, project: ProjectSlug, filter: ListFilter) -> Result<Vec<ContractSummary>, RepoError>;
    async fn update(&self, id: ContractId, patch: ContractPatch) -> Result<(), RepoError>;
    async fn delete(&self, id: ContractId) -> Result<(), RepoError>;
    async fn search(&self, project: ProjectSlug, query: SearchQuery) -> Result<Vec<SearchHit>, RepoError>;
}
```

The trait is the only seam between the core service and storage. A Postgres implementation can be added later by writing a second implementation behind the same trait, without touching any handler or tool.

## Why one container

The product is meant to run on a small box next to the code it describes. Adding Postgres, a separate vector database, or a managed embedding service would force every user to operate three things instead of one. The tradeoff is that horizontal scale is harder in v1, and that is acceptable for the planned use case.
