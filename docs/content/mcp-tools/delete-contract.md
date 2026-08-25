---
title: delete_contract
description: Delete a contract. The row is removed and the embedding is dropped. The audit log retains the full record.
method: WRITE
---

Delete a contract. The row is removed from the `contracts` table and the corresponding vector is removed from `contract_embeddings`. The audit log retains the full record of the deleted row.

## Request

| Parameter | Type | Description |
|---|---|---|
| project_slug | string | Required. |
| contract_id | string | Required. |

## Response

```json
{ "status": "deleted", "contract_id": "8a4b..." }
```

## Errors

| Status | When |
|---|---|
| 401 | No bearer token. |
| 403 | The token does not have `ledgapi:write`. |
| 404 | The project or contract does not exist. |

## Notes

Deletion is permanent. There is no soft delete and no undo. To bring a contract back, re-create it with the same fields. The new contract will have a new id, and the audit log will show the original delete followed by the new create.
