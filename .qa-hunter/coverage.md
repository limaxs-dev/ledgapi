# QA Coverage

All recorded segments are explored. Iteration 3 covered API, UI functional,
integration, and edge-case behavior through live localhost probes. Iteration 4
covered regression verification. Iteration 5 verified the loaded UI, MCP API,
and authentication through Chrome DevTools Protocol. No segments are blocked.

Iteration 7 closed the regression-backfill gap: all 15 historical findings now
have live automated tests (`tests/regression_backfill.rs` plus existing unit
tests in `src/domain/use_cases/update_contract.rs` and `tests/e2e_mcp.rs`).
Full suite green: 121 unit + all integration/e2e crates pass.

Iteration 8 deep-dived MCP: new `tests/e2e_mcp_corner_cases.rs` (28 tests)
covers JSON-RPC envelope edge cases, transport body limit, every tool's
validation error paths, viewer-scope enforcement, token lifecycle
(expired/revoked/inactive), cross-project isolation, SimilarFound
semantics, and a full CRUD cycle across all 10 tools. All segments remain
explored; no blocked segments.

Iteration 9 performed the git-staleness recheck (HEAD moved past
`last_tested_commit` 3b63031 since iteration 7). The full suite (186
tests pre-fix, 188 after) passed locally, clippy and fmt are clean, and
the new docs surface (`/docs/*`), the design refresh, the auth-hardening
flash flow, and the nested-group tree all serve correctly under a live
`cargo run`. No segments are blocked. UI-functional target
`POST /admin/users` was freshly explored — its flash-based error flow
(flash=created/duplicate/invalid) now works for all three validation
paths, fixing the one regression found.
