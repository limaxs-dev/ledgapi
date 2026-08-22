# Project: Ledgapi v1

## What
Self-hosted, agent-native API contract registry. Single-crate Rust binary (Axum 0.8 + SQLite + sqlite-vec + fastembed + MCP).

## How it runs
- Local dev: `APP__SERVER__BIND=127.0.0.1:8080 cargo run`
- Docker: `docker compose -f docker/docker-compose.yaml up -d`
- Health: `GET /healthz` (cheap), `GET /readyz` (DB check)
- MCP endpoint: `POST /mcp` (Streamable HTTP, requires Bearer)
- First-run token: stdout (`LEDGAPI_BOOTSTRAP_TOKEN=...`) + `GET /setup` page (returns 410 after consumption or 5min TTL)

## Spec
- `docs/superpowers/specs/2026-08-21-ledgapi-design.md`
- `docs/superpowers/plans/2026-08-21-ledgapi.md`

## Known follow-ups (post-v1, NOT blocking this run)
- `make deny` fails on upstream `paste` (RUSTSEC-2024-0436) + `webpki-roots` license (CDLA-Permissive-2.0) transitive via fastembed
- `make archaven` fails (tool not installed in env)
- Docker build fails on Bookworm (upstream `ort-sys 2.0.0-rc.13` C++ ABI mismatch)

## Environment
- Repo: /home/limaxs/work/Workspace/My Product/ledgapi
- Branch: master (pushed to origin)
- 51 commits, working tree clean
- 126 lib tests + 9 e2e + 4 arch + 4 #[ignore] live = 126 passing
- All cargo test/clippy/fmt --check pass
- No prior `.qa-hunter/` state
