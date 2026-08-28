---
title: Install
description: Run ledgapi from a single Docker container. Seed the initial super-admin from environment variables on first boot.
---

ledgapi ships as one Docker image. SQLite, the vector index, and the embedding model are all inside the container, so a single `docker run` is enough for most setups.

## Requirements

- Docker 24 or later
- 1 GB of free RAM for the embedding model (`all-MiniLM-L6-v2` loads into memory at startup)
- A persistent volume for SQLite and the embedded model cache

## Quickstart with docker compose

Save the snippet below as `docker-compose.yaml` in an empty directory.

```yaml
services:
  ledgapi:
    image: ghcr.io/your-org/ledgapi:latest
    ports:
      - "${APP_HOST_PORT:-18080}:${APP_CONTAINER_PORT:-18080}"
    environment:
      APP__AUTH__INITIAL_ADMIN_USERNAME: admin
      APP__AUTH__INITIAL_ADMIN_PASSWORD: change-this-password
      APP__AUTH__ISSUER: ${APP_ISSUER:-http://localhost:18080}
    volumes:
      - ledgapi-data:/data
    restart: unless-stopped

volumes:
  ledgapi-data:
```

Then start the container.

```bash
docker compose up -d
```

::: warning
The `INITIAL_PASSWORD` is read only on the first boot, when the `users` table is empty. After that, changing the environment variable has no effect. Pick a password of at least 12 characters.
:::

## What runs on first boot

1. The container reads the `users` table. If empty, it creates the initial super-admin from `APP__AUTH__INITIAL_ADMIN_USERNAME` and `APP__AUTH__INITIAL_ADMIN_PASSWORD`. Both variables are required on the first start; they are ignored on every subsequent start.
2. SQLite is created at the data volume path. The `sqlite-vec` extension is loaded, and the `contract_embeddings` virtual table is initialised.
3. The embedding model is downloaded once into the data volume. Subsequent restarts use the cached copy.

## Verifying the install

Open `http://localhost:18080/` and sign in with the credentials you set. You should see an empty projects list. To check the OAuth discovery endpoint from a terminal:

```bash
curl -s http://localhost:18080/.well-known/oauth-authorization-server | head
```

The response is a JSON document with `issuer`, `authorization_endpoint`, `token_endpoint`, and the supported scopes.

## Updating

Pull the new image and restart the container. The data volume is preserved across updates, so SQLite, the vector index, and the embedding cache all survive.

```bash
docker compose pull ledgapi
docker compose up -d
```

Migrations run automatically at startup. The `migrations/` directory in the repository lists every migration in order. If a migration fails, the container exits and the previous image is still on disk.
