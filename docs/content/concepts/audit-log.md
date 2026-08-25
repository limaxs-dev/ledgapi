---
title: Audit log
description: What gets recorded, what does not, and how to read it through the UI.
---

The audit log is an append-only record of every successful mutation. Reads, logins, and failed writes are not logged. The log lives in the same SQLite database, so a backup of the data volume is a backup of the log.

## Schema

| Column | Type | Description |
|---|---|---|
| id | integer | Monotonically increasing primary key. |
| timestamp | datetime | UTC at the moment the transaction committed. |
| actor_user_id | text | The id of the user that issued the request. |
| actor_username | text | Snapshot of the username at log time. |
| action | text | `create`, `update`, or `delete`. |
| entity_type | text | `project`, `group`, `contract`, `user`, or `oauth_client`. |
| entity_id | text | The id of the affected row. |
| project_slug | text | The project the action is scoped to, or `null` for global actions. |
| details | text | A short JSON blob with the before/after diff for `update` and the deleted row for `delete`. |

## What gets recorded

Every successful:

- `create_project`
- `create_group` (planned v2; current auto-create is not logged)
- `create_contract`
- `update_contract`
- `delete_contract`
- `create_user`
- `deactivate_user` (planned v2)
- `oauth_consent_grant` for `ledgapi:admin` scope

## What does not get recorded

- Reads of any kind, including `get_contract_by_id`, `list_contracts`, and `search_contract`.
- Logins, including failed logins.
- Failed writes. The server returns an error and the audit log is not appended.
- The contents of `request_body` or `request_example` on update. The audit row records only the diff between the old and new values, not the full payload.
- MCP `tools/list` and `resources/list` calls.

## Reading the log

A super-admin opens `/admin/audit` in the UI. The default view is the most recent 200 rows, newest first. The filter bar accepts a username, an action, an entity type, and a date range.

```bash
# Direct database query, with a copy of the data volume:
sqlite3 ledgapi.db "SELECT timestamp, actor_username, action, entity_type, entity_id, project_slug FROM audit_log ORDER BY id DESC LIMIT 50"
```

The `details` column is JSON. For an update, it looks like:

```json
{
  "before": { "summary": "old text", "tags": ["a"] },
  "after":  { "summary": "new text", "tags": ["a", "b"] }
}
```

For a delete, it is the full row as it was before the delete.

## Retention

There is no automatic pruning. The audit log grows for the lifetime of the database. For a busy instance, expect a few rows per write; for a quiet one, near zero.

Pruning is a manual operation. A super-admin runs a `DELETE FROM audit_log WHERE timestamp < ?` query with their preferred cutoff. The deletion is not itself logged.

## Why so much is not logged

The goal is accountability for mutations, not surveillance of reads. Logging every search would multiply the storage requirement by orders of magnitude, would slow down reads, and would not add useful information. If a super-admin needs to know who read what, the answer is the session log at the reverse proxy.
