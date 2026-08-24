PRAGMA foreign_keys = ON;

CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL CHECK (role IN ('super_admin', 'editor', 'viewer')),
    active        INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE INDEX users_active_idx ON users(active);

CREATE TABLE web_sessions (
    token_hash      TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    csrf_token_hash TEXT NOT NULL,
    expires_at      INTEGER NOT NULL,
    created_at      INTEGER NOT NULL,
    last_seen_at    INTEGER NOT NULL,
    revoked_at      INTEGER
);

CREATE INDEX web_sessions_user_idx ON web_sessions(user_id);
CREATE INDEX web_sessions_expiry_idx ON web_sessions(expires_at);

CREATE TABLE oauth_clients (
    client_id                 TEXT PRIMARY KEY,
    client_name               TEXT NOT NULL,
    redirect_uris             TEXT NOT NULL,
    token_endpoint_auth_method TEXT NOT NULL DEFAULT 'none',
    created_at                INTEGER NOT NULL
);

CREATE TABLE oauth_authorization_codes (
    code_hash              TEXT PRIMARY KEY,
    client_id              TEXT NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    user_id                TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    redirect_uri           TEXT NOT NULL,
    scope                  TEXT NOT NULL,
    code_challenge         TEXT NOT NULL,
    code_challenge_method  TEXT NOT NULL,
    expires_at             INTEGER NOT NULL,
    consumed_at            INTEGER
);

CREATE INDEX oauth_codes_expiry_idx ON oauth_authorization_codes(expires_at);

CREATE TABLE oauth_access_tokens (
    token_hash  TEXT PRIMARY KEY,
    client_id   TEXT NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope       TEXT NOT NULL,
    expires_at  INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    revoked_at  INTEGER
);

CREATE INDEX oauth_access_user_idx ON oauth_access_tokens(user_id);
CREATE INDEX oauth_access_expiry_idx ON oauth_access_tokens(expires_at);

CREATE TABLE oauth_refresh_tokens (
    token_hash      TEXT PRIMARY KEY,
    client_id       TEXT NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope           TEXT NOT NULL,
    expires_at      INTEGER NOT NULL,
    created_at      INTEGER NOT NULL,
    revoked_at      INTEGER,
    replaced_by_hash TEXT
);

CREATE INDEX oauth_refresh_user_idx ON oauth_refresh_tokens(user_id);
CREATE INDEX oauth_refresh_expiry_idx ON oauth_refresh_tokens(expires_at);

CREATE TABLE audit_log (
    id            TEXT PRIMARY KEY,
    actor_user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    action        TEXT NOT NULL CHECK (action IN ('create', 'update', 'delete')),
    resource_type TEXT NOT NULL CHECK (resource_type IN ('user', 'project', 'group', 'contract')),
    resource_id   TEXT,
    metadata      TEXT NOT NULL,
    created_at    INTEGER NOT NULL
);

CREATE INDEX audit_actor_idx ON audit_log(actor_user_id, created_at DESC);
CREATE INDEX audit_resource_idx ON audit_log(resource_type, resource_id, created_at DESC);
CREATE INDEX audit_created_idx ON audit_log(created_at DESC);
