---
title: search_contract
description: Hybrid search over the contracts in a project. Combines semantic (RAG) and exact (lexical) matching.
method: READ
---

Search contracts in a project by natural language or by path. The default mode is `hybrid`, which blends a semantic score from the embedding model with a lexical score from the path and summary.

## Request

| Parameter | Type | Description |
|---|---|---|
| project_slug | string | Required. |
| query | string | Required. Natural language or path fragment. |
| search_mode | enum | Optional. `hybrid` (default), `semantic`, or `exact`. |
| group_name | string | Optional. Restrict the search to one group. |
| limit | integer | Optional. 1 to 100. Default 10. |

## Response

```json
{
  "results": [
    { "id": "8a4b...", "method": "POST", "path": "/api/v1/auth/login", "summary": "...", "similarity": 0.91 },
    { "id": "1f0a...", "method": "GET",  "path": "/api/v1/users/{id}",  "summary": "...", "similarity": 0.74 }
  ]
}
```

Results are sorted by score, descending. The `similarity` field is the final score after the mode-specific weighting. In `exact` mode it is 1.0 for a match and 0.0 otherwise.

## Errors

| Status | When |
|---|---|
| 401 | No bearer token. |
| 403 | The token does not have `ledgapi:read`. |
| 404 | The project does not exist. |
| 422 | The `search_mode` value is not one of the allowed values, or `limit` is out of range. |

## Modes

| Mode | When to use |
|---|---|
| `hybrid` | Default. Wins on most queries. The semantic score catches natural-language intent; the lexical score catches exact-path completions. |
| `semantic` | When the query is a description, not a path. Example: "how does the client log in". |
| `exact` | When the user is typing a path and needs completion. Fast and predictable. |

## Notes

The lexical score uses simple case-insensitive substring matching against `method`, `path`, and `summary`. It is not a full-text search engine. For complex queries, prefer `semantic`.
