---
title: ledgapi
description: A self-hosted, agent-native API contract registry. AI agents query and register endpoint contracts through MCP; humans review them through a read-only UI.
eyebrow: Self-hosted registry
---

A source of truth for what endpoints already exist. AI coding agents query it through MCP, humans review it through the UI.

## What it is

ledgapi stores the API contracts of a project, not the API traffic. Each contract describes a single endpoint: method, path, request and response schemas, and an example. Agents read the registry before writing new code so they do not duplicate work that already exists, and they register new endpoints after writing them so the rest of the team can see what changed.

The registry is single-tenant, self-hosted, and runs offline. Embedding models run on the host through `fastembed-rs`; nothing is sent to a third party.

## Three surfaces

::: bento
- **Contracts**: one row per endpoint, with method, path, group, status, tags, and an optional OpenAPI export per project.
- **MCP tools**: ten tools over HTTP + SSE. Agents call them through any OAuth 2.1 capable MCP client. Reads and writes go through the same role system the web UI uses.
- **Audit log**: every create, update, and delete is appended with the acting user and timestamp. Reads are not logged.
:::

## Quickstart

```bash
export INITIAL_USERNAME=admin
export INITIAL_PASSWORD=change-this-password   # min 12 characters
docker compose -f docker/docker-compose.yaml up -d
```

Open `http://localhost:8080/` and sign in with those credentials. Connect an MCP client by adding one entry to `.mcp.json`:

```json
{
  "mcpServers": {
    "ledgapi": {
      "type": "http",
      "url": "http://localhost:8080/mcp"
    }
  }
}
```

The client discovers the OAuth metadata, opens a browser for login and consent, and stores the resulting tokens itself. No secrets in config.

## Roles

| Role | Read | Write | Manage users | View audit log |
|---|---|---|---|---|
| viewer | yes | no | no | no |
| editor | yes | yes | no | no |
| super-admin | yes | yes | yes | yes |

Roles are scoped to a single ledgapi instance, not per project. Every user can read every project. The super-admin is seeded from environment variables on first boot and never overwritten on subsequent starts.

## Where to go next

If you are running ledgapi for the first time, the [install guide](/docs/getting-started/install) takes about five minutes. If you are integrating an MCP client, jump to [connect an MCP client](/docs/getting-started/connect-mcp). If you want the mental model first, read [architecture](/docs/concepts/architecture).
