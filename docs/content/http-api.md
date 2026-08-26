---
title: HTTP and UI endpoints
description: Every HTTP route the server exposes, the method, who can call it, and what it returns.
---

The HTTP surface splits into three groups: the MCP endpoint, the web UI, and the OAuth flow. Most of the time you call the MCP tools, not the HTTP endpoints; the HTTP layer is here for the browser and for direct integration.

## MCP and discovery

| Endpoint | Method | Auth | Description |
|---|---|---|---|
| `/mcp` | POST or GET (SSE) | OAuth bearer | The MCP transport. POST for normal JSON-RPC, GET to hold an SSE stream open. |
| `/.well-known/oauth-authorization-server` | GET | none | OAuth 2.1 discovery metadata. Returns `issuer`, `authorization_endpoint`, `token_endpoint`, supported PKCE methods, and the list of scopes. |
| `/.well-known/oauth-protected-resource` | GET | none | Protected resource metadata. Lists the supported scopes and the authorization servers that can mint tokens for them. |

## Web UI

All UI routes require a logged-in session cookie. There is no anonymous read.

| Endpoint | Method | Auth | Description |
|---|---|---|---|
| `/` | GET | session | Dashboard. Lists every project with the contract count. |
| `/projects/{slug}` | GET | session | The project view. Lists every group and the contracts inside. |
| `/projects/{slug}/contracts/{id}` | GET | session | Contract detail. Full record, both schemas, both examples, and the per-contract audit trail. |
| `/projects/{slug}/search?q=...` | GET | session | Search UI. The query is forwarded to `search_contract` in `hybrid` mode. |
| `/projects/{slug}/openapi.yml` | GET | session | OpenAPI export. Same YAML that `export_openapi` returns, served with the right headers. |
| `/login` | GET, POST | none | Sign in. GET shows the form, POST submits it. |
| `/logout` | POST | session | Sign out. Invalidates the session cookie. |

## OAuth flow

| Endpoint | Method | Auth | Description |
|---|---|---|---|
| `/oauth/register` | POST | none | Dynamic client registration. A client posts its name and redirect URIs; the server returns a client id. |
| `/oauth/authorize` | GET | session | Authorization endpoint. Renders the consent screen with the requested scopes. |
| `/oauth/consent` | POST | session | Consent submission. On approve, redirects to the client's redirect URI with a short-lived code. |
| `/oauth/token` | POST | client id + secret (or PKCE) | Token endpoint. Exchanges the code for an access token and a refresh token. |

## Admin

| Endpoint | Method | Auth | Description |
|---|---|---|---|
| `/admin/users` | GET, POST | super-admin | User management. GET lists users, POST creates one. |
| `/admin/audit` | GET | super-admin | Global audit log. Newest first, paginated, filterable. |

## Health

| Endpoint | Method | Auth | Description |
|---|---|---|---|
| `/healthz` | GET | none | Liveness probe. Returns 200 if the process is alive. |
| `/readyz` | GET | none | Readiness probe. Returns 200 if SQLite is reachable and the embedding model is loaded. |

## Error format

Every JSON error from the HTTP layer uses the same shape:

```json
{ "error": "not_found", "message": "Project 'acme-billing' does not exist." }
```

The `error` field is a stable machine-readable code. The `message` field is for humans. MCP errors follow the JSON-RPC 2.0 error format and include the same `error` code under `data.code`.

## Static files

| Path | Description |
|---|---|
| `/static/logo.png` | The brand logo (transparent PNG, embedded at compile time). |
| `/static/style.css` | The product UI stylesheet. The docs site has its own stylesheet appended to this file. |
