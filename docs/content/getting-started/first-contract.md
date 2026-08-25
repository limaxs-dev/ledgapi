---
title: Your first contract
description: Create a project, add a group, register a contract, and search for it. Five minutes from empty database to a working RAG search.
---

This page walks the whole flow once. If you have an MCP client connected, the tool calls below are copy-pasteable. If you are using the UI, the same actions are available as form fields under `/admin/projects` and `/projects/{slug}/contracts/new`.

## 1. Create a project

A project is the top-level container. Every contract belongs to one project.

```json
{
  "slug": "acme-billing",
  "name": "Acme Billing API",
  "description": "Internal billing service for Acme customers"
}
```

The slug is what every other tool call refers to. Pick something short and stable. You cannot rename a slug after creation; you can only change the human-readable `name`.

The response is the project id and a confirmation. The new project now appears in the projects list.

## 2. Add a group

Groups are folders inside a project. Most projects have a few: `auth`, `users`, `billing`, `reports`. The group name is required when creating a contract, so adding them up front keeps the registry tidy.

```json
{
  "project_slug": "acme-billing",
  "name": "auth"
}
```

Groups are auto-created if you skip this step and pass `group_name` to `create_contract` directly. Auto-created groups have no description and cannot be edited later, so it is worth adding them explicitly.

## 3. Register a contract

This is the main write. A contract describes one endpoint: method, path, schemas, an example, and a few flags.

```json
{
  "project_slug": "acme-billing",
  "group_name": "auth",
  "method": "POST",
  "path": "/api/v1/auth/login",
  "summary": "Exchange username and password for a session token",
  "description": "Used by the web client and the mobile app. Returns a short-lived bearer token and a longer-lived refresh token.",
  "request_body_schema": {
    "type": "object",
    "required": ["username", "password"],
    "properties": {
      "username": { "type": "string" },
      "password": { "type": "string", "format": "password" }
    }
  },
  "request_example": {
    "username": "alice",
    "password": "correct horse battery staple"
  },
  "response_schema": {
    "type": "object",
    "required": ["access_token", "refresh_token", "expires_in"],
    "properties": {
      "access_token":  { "type": "string" },
      "refresh_token": { "type": "string" },
      "expires_in":    { "type": "integer" }
    }
  },
  "response_example": {
    "access_token": "eyJhbGciOi...",
    "refresh_token": "GEvxJ9...",
    "expires_in": 900
  },
  "auth_type": "none",
  "tags": ["public", "rate-limited"]
}
```

The server runs a RAG similarity check before insert. If a similar contract is found and `force` is not set, the response is `warning_similar_found` with the top five matches and their similarity scores. Resend with `"force": true` to create anyway.

The success response is `{ "status": "created", "contract_id": "uuid" }`.

## 4. List and retrieve

`list_contracts` returns compact summaries for a project. Use it to drive a dropdown in your editor or to populate a sidebar.

```json
{
  "project_slug": "acme-billing",
  "group_name": "auth"
}
```

`get_contract_by_id` returns the full record, including both schemas and the examples.

```json
{
  "project_slug": "acme-billing",
  "contract_id": "8a4b...-c1d2"
}
```

## 5. Search

`search_contract` is the read path agents will hit most. Pass a natural language query and the server returns the closest matches, sorted by similarity.

```json
{
  "project_slug": "acme-billing",
  "query": "how does the client get a session token",
  "search_mode": "semantic",
  "limit": 5
}
```

`hybrid` is the default. It blends the semantic score with a lexical score, which usually wins on exact-path queries. `exact` matches the query against the path only and is fast for completion-style UIs.

## What just happened

The registry now holds one project, one group, and one contract. The contract's embedding is in the `contract_embeddings` vector table. The audit log has three rows: one for the project create, one for the group create, one for the contract create. A super-admin can see all three at `/admin/audit`.
