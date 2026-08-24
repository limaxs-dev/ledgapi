-- 0003_contract_examples.sql — named request/response examples per contract.
CREATE TABLE contract_examples (
    id          TEXT PRIMARY KEY,
    contract_id TEXT NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('positive', 'negative')),
    status_code INTEGER NOT NULL CHECK (status_code BETWEEN 100 AND 599),
    request     TEXT NOT NULL,
    response    TEXT NOT NULL,
    ordinal     INTEGER NOT NULL DEFAULT 0 CHECK (ordinal >= 0),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    UNIQUE (contract_id, name)
);

CREATE INDEX contract_examples_contract_idx ON contract_examples(contract_id, ordinal);
