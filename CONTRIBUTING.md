# Contributing

## Commit conventions

[Conventional Commits](https://www.conventionalcommits.org/):

- `feat: …` — new feature
- `fix: …` — bug fix
- `refactor: …` — no behaviour change
- `test: …` — tests only
- `docs: …` — documentation only
- `chore: …` — tooling, deps, CI
- `ci: …` — CI changes
- `perf: …` — performance

Scope optional, e.g. `feat(mcp): …`.

## Local checks

Before pushing:

```bash
make fmt
make clippy
make test
make architecture
make deny
make archaven
```

Every gate must be green. `make ci` runs them all.

## Code style

- 4-space indent, max width 100, `edition = "2024"`.
- Doc comments on every public item.
- No `unsafe` (`unsafe_code = "forbid"` at workspace level).
- No `lazy_static`/`OnceCell`/global mutable state. All shared state lives in `AppState`.
