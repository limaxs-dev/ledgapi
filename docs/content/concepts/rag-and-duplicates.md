---
title: RAG and duplicate detection
description: How ledgapi decides whether a new contract is a near-duplicate of an existing one, and how to override the check.
---

Every contract insert runs a similarity check before the row is written. The check uses a local embedding model, no third-party API, and the threshold is configurable.

## What is embedded

The vector is built from the concatenation of four fields, in this order:

```text
method + " " + path + " " + summary + " " + description
```

`tags` and `request_example` are not part of the embedding. The model is `all-MiniLM-L6-v2`, which produces 384-dimensional vectors. The first call downloads the model into the data volume; subsequent calls use the cache.

## How similarity is measured

The server queries the top five nearest contracts in the same project through `sqlite-vec`, using cosine distance. The five hits are sorted by score, and the highest score is compared to a threshold.

| `search_mode` | Used by | Meaning |
|---|---|---|
| semantic | search_contract | Cosine similarity over the embedding only. |
| exact | search_contract | Lexical match on the path. |
| hybrid | default for search_contract | Weighted blend of semantic and exact. |
| duplicate | create_contract, internal | The same as semantic, but with a hard threshold. |

## The threshold

The default threshold is **0.85**. It is read from the environment variable `SIMILARITY_THRESHOLD` at startup. A change requires a container restart.

A score above the threshold is treated as a duplicate. The handler returns `warning_similar_found` with the top five matches and their scores, and the database is not touched.

## Overriding the check

Set `force: true` in the request body.

```json
{
  "project_slug": "acme-billing",
  "method": "POST",
  "path": "/api/v1/auth/login",
  "summary": "...",
  "force": true
}
```

The contract is inserted without the similarity check. The audit log records the override with the highest similarity score, so a super-admin can spot agents that are routinely forcing past the check.

## Why RAG and not just exact path matching

Two endpoints with different paths can describe the same behavior. The RAG layer catches the case where an agent is about to write a new endpoint that does the same thing as an existing one, even when the URLs do not match.

A pure exact match on `(method, path)` would miss the case where one agent writes `/users/{id}/avatar` and another writes `/users/{id}/profile-picture`. The semantic check puts both close together in vector space and flags the second as a likely duplicate.

## Limits

- The embedding model is English-dominant. Non-English summaries will match less reliably.
- The threshold is global, not per-project. Some teams may want a stricter or looser bar; that is a v2 feature.
- There is no incremental update. Renaming a contract does not re-embed it. If a high-similarity false positive is annoying, the workaround is to rewrite the summary to be more specific, which triggers a re-embed on the next update.
