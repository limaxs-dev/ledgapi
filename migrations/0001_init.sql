-- 0001_init.sql — relational schema + indexes.
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE projects (
    id          TEXT PRIMARY KEY,
    slug        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    description TEXT,
    created_at  INTEGER NOT NULL
);

CREATE TABLE groups (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT,
    UNIQUE(project_id, name)
);

CREATE TABLE contracts (
    id                  TEXT PRIMARY KEY,
    project_id          TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    group_id            TEXT     REFERENCES groups(id) ON DELETE SET NULL,
    method              TEXT NOT NULL CHECK (method IN ('GET','POST','PUT','PATCH','DELETE')),
    path                TEXT NOT NULL,
    summary             TEXT NOT NULL,
    description         TEXT,
    request_headers     TEXT,
    request_params      TEXT,
    request_body_schema TEXT,
    request_example     TEXT,
    response_schema     TEXT NOT NULL,
    response_example    TEXT,
    auth_type           TEXT,
    status              TEXT NOT NULL DEFAULT 'draft'
                        CHECK (status IN ('draft','stable','deprecated')),
    tags                TEXT NOT NULL DEFAULT '[]',
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    UNIQUE (project_id, method, path)
);

CREATE INDEX contracts_project_idx ON contracts(project_id);
CREATE INDEX contracts_group_idx   ON contracts(group_id);
CREATE INDEX contracts_status_idx  ON contracts(project_id, status);

CREATE TABLE auth_tokens (
    token_hash  TEXT PRIMARY KEY,
    label       TEXT,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS _migrations (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    applied_at  INTEGER NOT NULL
);
