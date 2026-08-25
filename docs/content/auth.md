---
title: Authentication and OAuth 2.1
description: Browser sessions for the UI, OAuth 2.1 with PKCE for MCP clients, and the role-to-scope map.
---

There are two authentication paths. The web UI uses session cookies. MCP clients use OAuth 2.1 with PKCE. The two paths share the same user table and the same role map.

## Browser sessions

The login form at `/login` takes a username and password. On success, the server sets a session cookie. The cookie is `HttpOnly`, `SameSite=Lax`, and rotated on every login.

The session token is opaque and random. The server stores a hash of the token in the `web_sessions` table, plus a CSRF hash and the expiry. The plaintext token only ever lives in the cookie.

`Secure` is set when `APP__AUTH__COOKIE_SECURE=true`. In production behind HTTPS, this must be `true`. In local development over plain HTTP, leave it `false` or the cookie will be dropped by the browser.

## OAuth 2.1 for MCP clients

An MCP client connects through OAuth 2.1 with PKCE. There is no client secret; every client is a public client.

### Discovery

The client fetches `/.well-known/oauth-authorization-server` to learn the issuer, the authorization and token endpoints, and the supported PKCE methods. The response also lists the scopes the server understands.

### Dynamic client registration

The client posts to `/oauth/register` with a name and a list of redirect URIs. The server returns a client id. There is no admin approval step; the consent screen is the only gate.

### Authorization

The client generates a PKCE verifier and challenge, then opens the authorization endpoint in a browser. The user signs in (or is already signed in through the session cookie) and approves the requested scopes on the consent screen.

On approval, the server redirects to the client's redirect URI with a short-lived authorization code. On denial, the redirect carries an `error` query parameter and no code.

### Token exchange

The client posts to `/oauth/token` with the code, the PKCE verifier, and the client id. The server validates the PKCE pair, marks the code as used, and returns an access token and a refresh token.

Access tokens are short-lived. Refresh tokens are long-lived and rotated on every use. The old refresh token is invalidated as soon as the new pair is issued.

### Logout

There is no client-initiated logout in v1. A client that wants to disconnect can drop its tokens locally. The server keeps refresh tokens until they expire or until the user revokes them through a future `/oauth/revoke` endpoint.

## Roles and scopes

| Role | Granted scopes |
|---|---|
| viewer | `ledgapi:read` |
| editor | `ledgapi:read`, `ledgapi:write` |
| super-admin | `ledgapi:read`, `ledgapi:write`, `ledgapi:admin` |

A token with `ledgapi:read` cannot call any tool that mutates state, even if the tool name is correct. The server checks the scope before dispatching the tool and returns a 403 with the missing scope listed.

A client that asks for `ledgapi:admin` must be approved by a super-admin. The consent screen reflects this and the approval is recorded in the audit log with the user that pressed the button.

## Initial super-admin

On first boot, the server reads `APP__AUTH__INITIAL_ADMIN_USERNAME` and `APP__AUTH__INITIAL_ADMIN_PASSWORD`. If the `users` table is empty, the server creates a super-admin from these values. Both variables are required on the first start; they are ignored on every subsequent start.

Changing the environment variables after the first boot has no effect. To change the admin password, sign in and rotate it through a future `/admin/users/{id}/password` endpoint, or update the database directly.

## Production checklist

- `APP__AUTH__ISSUER` set to the public URL of the instance.
- `APP__AUTH__COOKIE_SECURE=true` when serving over HTTPS.
- All redirect URIs registered by clients are `https://`. The localhost exemption covers `http://localhost:*` and `http://127.0.0.1:*` only.
- The initial admin password is at least 12 characters.
