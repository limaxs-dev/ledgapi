# Confirmed QA Environment

- Environment: disposable local SQLite database and embedding cache under the session scratchpad.
- Server: `APP__SERVER__BIND=127.0.0.1:8080 cargo run` with database/cache paths overridden to the scratchpad.
- Browser verification: disposable headless Google Chrome with Chrome DevTools Protocol on `127.0.0.1:9222`.
- Audited evidence artifact: `.qa-hunter/evidence/qa-recheck-cdp.json`.
- Chrome DevTools verified `Page.loadEventFired`, `document.readyState=complete`, HTML doctype, `Projects · ledgapi` title, `Projects` heading, stylesheet HTTP 200, dashboard HTTP 200 `text/html`, and zero console exceptions.
- Chrome DevTools page-context fetch verified MCP no-auth and invalid-token responses as HTTP 401, valid initialize as HTTP 200 JSON-RPC `ledgapi`, and `tools/list` as HTTP 200 with 10 tools.
- Chrome DevTools page-context fetch verified `/healthz` HTTP 200 with `ok`.
- Live testing completed; server and disposable browser are stopped afterward.
- Earlier probe results using a mistyped bearer token or `/health` instead of documented `/healthz` were excluded from findings.
