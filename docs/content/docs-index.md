---
title: Documentation
description: Index of every page in the ledgapi docs.
---

The docs are organised into four sections. The first is for setup, the second for the mental model, the third for the ten MCP tools, and the fourth for HTTP, auth, and deployment reference.

## Sections

**Getting started**: install the container, log in for the first time, connect an MCP client, and register your first contract.

**Concepts**: the architecture, how projects and groups fit together, how duplicate detection works, and what the audit log records.

**MCP tools**: one page per tool. Each page lists every parameter, the response shape, and the error codes the tool can return.

**Reference**: the HTTP and UI endpoints, the OAuth 2.1 flow, and the deployment environment variables.

## Conventions

Code blocks in the docs are tested. The `docker compose` snippet on the install page is the same one used in CI. The `.mcp.json` block is the minimum needed to make an OAuth-capable client open the browser for consent.

Inline identifiers use the same names as the source: `project_slug`, `group_name`, `contract_id`. Words in this style are literal values, not descriptive text.

Method badges match the HTTP method colors used in the product UI: GET in blue, POST in green, PUT in amber, PATCH in violet, DELETE in red. The MCP tool pages use READ and WRITE in the same colors to mean viewer and editor scope.
