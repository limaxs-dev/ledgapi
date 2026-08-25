---
title: export_openapi
description: Export a project as an OpenAPI 3.1 YAML document. The response includes a download URL.
method: READ
---

Export a project as an OpenAPI 3.1 YAML document. The response includes the YAML string and a download URL for the same payload.

## Request

| Parameter | Type | Description |
|---|---|---|
| project_slug | string | Required. |

## Response

```json
{
  "yaml": "openapi: 3.1.0\ninfo:\n  title: Acme Billing API\n  version: 1.0.0\npaths:\n  /api/v1/auth/login:\n    post:\n      summary: ...\n      ...\n",
  "download_url": "/projects/acme-billing/openapi.yml"
}
```

The download URL is also reachable directly through a logged-in browser session at `/projects/{slug}/openapi.yml`. The content is the same YAML, served with the right `Content-Type` and `Content-Disposition` headers.

## Errors

| Status | When |
|---|---|
| 401 | No bearer token. |
| 403 | The token does not have `ledgapi:read`. |
| 404 | The project does not exist. |

## Notes

The export includes every contract, regardless of `status`. There is no flag to omit `draft` or `deprecated` contracts. A consumer that wants only `stable` contracts can filter the YAML client-side or filter the list before exporting.

Group names are translated to OpenAPI tags. A contract in `group_name: "auth"` ends up with `tags: ["auth"]` in the export.
