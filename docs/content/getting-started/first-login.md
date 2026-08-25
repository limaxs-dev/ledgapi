---
title: First login
description: Sign in with the initial super-admin, then create additional users with the right role for their job.
---

The first time you open `http://localhost:8080/`, sign in with the credentials from the install step. The browser stores a session cookie, and the home page shows an empty projects list.

## Sign in

1. Open the URL where ledgapi is running.
2. Enter the initial admin username and password.
3. The redirect lands on the projects dashboard.

The session cookie is `HttpOnly` and is rotated on every successful login. Old cookies are invalidated.

## Roles

ledgapi has three roles. Pick the smallest one that fits the job.

| Role | What they can do |
|---|---|
| viewer | Read projects, groups, and contracts through the UI and through MCP. Cannot create, update, or delete. |
| editor | Everything a viewer can do, plus create, update, and delete through MCP. |
| super-admin | Everything an editor can do, plus user management and the global audit log. |

Roles are instance-wide, not per project. Every user can read every project. There is no per-project ACL in v1.

## Create additional users

Only a super-admin can create users. Open `/admin/users` from the top nav.

```bash
# From a terminal, with a logged-in super-admin session cookie:
curl -X POST http://localhost:8080/admin/users \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{
    "username": "alice",
    "password": "another-strong-password",
    "role": "editor"
  }'
```

The response is the new user's id and a confirmation. Passwords are hashed with Argon2id before they touch the database. Plaintext passwords never leave the request.

## Disable a user

Set the user to inactive from the same admin page. Inactive users cannot sign in through the UI and cannot obtain OAuth tokens. Existing sessions are invalidated on the next request.

There is no soft delete in v1. To remove a user record entirely, drop the row from the `users` table directly. This is a planned v2 feature.

## OAuth scopes

The same role map applies to MCP clients. When a client requests a token, ledgapi checks the user's role and grants the matching scopes.

| Role | Granted scopes |
|---|---|
| viewer | `ledgapi:read` |
| editor | `ledgapi:read`, `ledgapi:write` |
| super-admin | `ledgapi:read`, `ledgapi:write`, `ledgapi:admin` |

A client with `ledgapi:read` cannot call `create_contract` even if it guesses the tool name. The server returns a 403 with the missing scope listed in the error body.
