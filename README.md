# ledgapi

<p align="center">
  <img src="logo.png" alt="ledgapi logo" width="320">
</p>

> **API contracts, remembered by your agents.**

A self-hosted, agent-native API contract registry. AI coding agents (Claude Code, Cursor, etc.) interact through MCP to query what endpoints already exist before writing code, and to register new endpoints after creating them. A login-protected web UI lets humans review contracts and audit history.

## Quickstart

```bash
export INITIAL_USERNAME=admin
export INITIAL_PASSWORD=change-this-password   # min 12 characters
docker compose -f docker/docker-compose.yaml up -d
```

On first boot the database is empty, so these two environment variables seed the **initial super-admin**. They are ignored on every subsequent start (existing users are never overwritten).

Open <http://localhost:8080/> and sign in with those credentials to use the web UI. Create additional users (viewer/editor/super-admin) under `/admin/users`.

### Connecting an MCP client

No tokens or headers needed — `.mcp.json` only carries `type` and `url`:

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

On first connect, an OAuth-capable MCP client discovers ledgapi's authorization metadata, opens your browser, and asks you to log in and approve access (consent screen). The client stores the resulting access/refresh token itself; nothing is pasted into config files.

Roles map to capabilities: **viewer** can search/read, **editor** can also create/update/delete via MCP, **super-admin** additionally manages users and views the global audit log (`/admin/audit`). Every successful create/update/delete is recorded in the audit log with its acting user; reads are not audited.

### Production notes

- Set `APP__AUTH__ISSUER=https://your-host` (and `APP__AUTH__COOKIE_SECURE=true`) when running behind HTTPS; OAuth redirect URIs must be `https://` except for localhost.
- Session cookies, web sessions, and OAuth tokens are stored hashed and expire per the TTLs documented in `.env.example`.
- The legacy static bootstrap token (`LEDGAPI_BOOTSTRAP_TOKEN`, `/setup`) has been removed.

## Manual smoke

```bash
curl -s http://localhost:8080/.well-known/oauth-authorization-server | head
```

## Development

```bash
make ci   # fmt-check + clippy + test + architecture + deny + archaven
```

See `docs/superpowers/specs/2026-08-21-ledgapi-design.md` for the original design and `docs/superpowers/plans/2026-08-23-ledgapi-oauth-users-audit.md` for this auth rework plan.
