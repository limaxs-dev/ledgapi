---
title: Deployment and env vars
description: The complete list of environment variables, what they do, and the recommended values for production.
---

The container reads its configuration from environment variables. The names use the `SECTION__KEY` convention from the `config` crate, so `APP__AUTH__ISSUER` is the `issuer` key under the `auth` section of the `app` namespace.

## Server

| Variable | Default | Description |
|---|---|---|
| `APP__SERVER__BIND` | `0.0.0.0:8080` | The address and port the HTTP server binds to. |
| `APP__SERVER__WORKERS` | `num_cpus::get()` | The number of Tokio worker threads. |

## Database

| Variable | Default | Description |
|---|---|---|
| `APP__DATABASE__URL` | `sqlite:///data/ledgapi.db?mode=rwc` | SQLite connection URL. WAL mode is enabled automatically. The `rwc` flag creates the file if missing. |
| `APP__DATABASE__POOL_SIZE` | `8` | The number of SQLite connections in the pool. |

## Auth

| Variable | Default | Description |
|---|---|---|
| `APP__AUTH__ISSUER` | `http://localhost:8080` | The public URL of the instance. Used in OAuth metadata and as the `iss` claim on tokens. |
| `APP__AUTH__COOKIE_SECURE` | `false` | Set `true` behind HTTPS. Sets the `Secure` flag on session cookies. |
| `APP__AUTH__INITIAL_ADMIN_USERNAME` | none | Read only on first boot. Required if the `users` table is empty. |
| `APP__AUTH__INITIAL_ADMIN_PASSWORD` | none | Read only on first boot. Must be at least 12 characters. |
| `APP__AUTH__SESSION_TTL` | `12h` | How long a session cookie is valid. |
| `APP__AUTH__ACCESS_TOKEN_TTL` | `1h` | How long an OAuth access token is valid. |
| `APP__AUTH__REFRESH_TOKEN_TTL` | `30d` | How long an OAuth refresh token is valid. |

## RAG

| Variable | Default | Description |
|---|---|---|
| `APP__RAG__SIMILARITY_THRESHOLD` | `0.85` | The cosine-similarity cutoff for the duplicate-detection check on `create_contract`. Range 0.0 to 1.0. |
| `APP__RAG__EMBEDDING_MODEL` | `all-MiniLM-L6-v2` | The `fastembed` model to use. Changing the model requires re-embedding every contract. |
| `APP__RAG__CACHE_DIR` | `/data/models` | Where the downloaded model is cached. Mount this as a persistent volume. |

## Logging

| Variable | Default | Description |
|---|---|---|
| `RUST_LOG` | `info,ledgapi=debug` | Standard `tracing-subscriber` filter. |

## Data volume

The data volume must be persistent. It holds:

- `ledgapi.db` and its WAL files
- The `models/` directory with the cached embedding model
- The audit log

A `docker volume` is fine for a single-host setup. For a multi-host setup, mount a network filesystem. SQLite in WAL mode is safe to share across hosts as long as the underlying filesystem supports POSIX locking.

## Backups

Back up the data volume with the server stopped, or with SQLite's `.backup` command while the server is running.

```bash
# While the server is running, from inside the container:
sqlite3 /data/ledgapi.db ".backup /data/backup-$(date +%s).db"
```

Copy the backup file to off-host storage. The `.backup` command takes a consistent snapshot even while writes are in flight.

There is no point-in-time recovery beyond the last backup. If the data volume is lost and the last backup is stale, the registry state is lost. Plan accordingly.

## Migrations

Migrations live in the `migrations/` directory of the repository. They run automatically at startup, in lexical order, inside a transaction. A failed migration aborts startup. The previous image is still on disk, so `docker compose down && docker compose up -d` with the previous image restores the old behavior.

New migrations are added by appending a numbered file to the directory. Never edit a migration that has already run on a deployed instance; the checksum mismatch will block startup.

## Resource sizing

The default container is fine for a single team. For larger deployments:

- Increase `APP__SERVER__WORKERS` to the number of cores.
- Increase `APP__DATABASE__POOL_SIZE` to two times the worker count.
- Mount the data volume on SSD. SQLite is sensitive to fsync latency.
- Expect the embedding model to use about 400 MB of RAM. A 1 GB container is tight; 2 GB is comfortable.
