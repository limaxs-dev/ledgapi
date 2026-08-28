---
title: Connect an MCP client
description: Add one entry to .mcp.json. The client discovers the OAuth metadata, opens a browser for consent, and stores the resulting tokens itself.
---

An MCP client connects to ledgapi through OAuth 2.1. There are no API keys to copy and no static tokens to paste into config files. The client does the discovery, the browser dance, and the token storage on its own.

## The minimum config

Add one entry to `.mcp.json` at the root of your project.

```json
{
  "mcpServers": {
    "ledgapi": {
      "type": "http",
      "url": "http://localhost:18080/mcp"
    }
  }
}
```

In production, swap `localhost` for your host. The URL must match the `APP__AUTH__ISSUER` environment variable on the server.

## What happens on first connect

1. The client fetches `http://localhost:18080/.well-known/oauth-authorization-server` to discover the issuer, authorization endpoint, token endpoint, and supported PKCE methods.
2. The client generates a PKCE verifier and a state value, then opens the authorization endpoint in a browser. The URL looks like `http://localhost:18080/oauth/authorize?response_type=code&client_id=...&code_challenge=...&state=...`.
3. You sign in to ledgapi in the browser. The same session cookie is reused if you are already signed in.
4. The consent screen lists the scopes the client is asking for. Approve or deny.
5. The server redirects back to the client's redirect URI with a short-lived authorization code.
6. The client exchanges the code for an access token and a refresh token at `/oauth/token`, using the PKCE verifier to prove it owns the request.
7. The client stores both tokens. From now on, every MCP request carries the access token in the `Authorization` header.

## Reconnect after a server restart

Nothing to do. The refresh token survives restarts. When the access token expires, the client uses the refresh token to mint a new one without opening the browser again.

If you change `APP__AUTH__ISSUER`, existing refresh tokens are invalidated. Every connected client has to redo the consent dance.

## Multiple clients

Each MCP client registers itself dynamically through `/oauth/register`. There is no admin step to pre-approve a client. The consent screen is the only gate.

A client that asks for `ledgapi:admin` must be approved by a super-admin. The consent screen reflects this and the approval is logged in the audit log with the user that pressed the button.

## Localhost exemption

When the issuer is `http://localhost:*` or `http://127.0.0.1:*`, redirect URIs do not need to be `https://`. Outside localhost, every redirect URI must be `https://`, and `APP__AUTH__COOKIE_SECURE` must be `true`.

```bash
# Production environment variables
export APP__AUTH__ISSUER=https://ledgapi.example.com
export APP__AUTH__COOKIE_SECURE=true
```

## Smoke check

From a terminal, with a valid access token in `$TOKEN`:

```bash
curl -s http://localhost:18080/.well-known/oauth-protected-resource | head
curl -s -H "Authorization: Bearer $TOKEN" http://localhost:18080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

The first request returns the protected resource metadata. The second returns the list of MCP tools the token has access to.
