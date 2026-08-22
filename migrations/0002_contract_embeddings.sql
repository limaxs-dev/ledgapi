-- 0002_contract_embeddings.sql — sqlite-vec virtual table.
-- Note: this depends on sqlite-vec having been loaded.

CREATE VIRTUAL TABLE IF NOT EXISTS contract_embeddings USING vec0(
    contract_id TEXT PRIMARY KEY,
    project_id  TEXT,
    embedding   float[384]
);
