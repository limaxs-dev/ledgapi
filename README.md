# ledgapi

> **API contracts, remembered by your agents.**

A self-hosted, agent-native API contract registry. AI coding agents (Claude Code, Cursor, etc.) interact through MCP to query what endpoints already exist before writing code, and to register new endpoints after creating them. A read-only web UI lets humans review.

## Quickstart

```bash
docker compose -f docker/docker-compose.yaml up -d
docker compose logs ledgapi | grep LEDGAPI_BOOTSTRAP_TOKEN
# copy the token, then put it into your MCP config:
```

`~/.claude/mcp.json`:

```json
{
  "mcpServers": {
    "ledgapi": {
      "type": "http",
      "url": "http://localhost:8080/mcp",
      "headers": { "Authorization": "Bearer <token-from-logs>" }
    }
  }
}
```

## Manual smoke

```bash
curl -s -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
```

Open <http://localhost:8080/> to browse the dashboard.

## Development

```bash
make ci   # fmt-check + clippy + test + architecture + deny + archaven
```

See `docs/superpowers/specs/2026-08-21-ledgapi-design.md` for the full design.
