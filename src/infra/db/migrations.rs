//! SQL migration runner. Embedded via `include_str!`. Tracks applied
//! migrations in the `_migrations` table.

use anyhow::Context;
use rusqlite::Connection;
use time::OffsetDateTime;

const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("../../../migrations/0001_init.sql")),
    ("0002_contract_embeddings", include_str!("../../../migrations/0002_contract_embeddings.sql")),
    ("0003_contract_examples", include_str!("../../../migrations/0003_contract_examples.sql")),
    (
        "0004_auth_users_oauth_audit",
        include_str!("../../../migrations/0004_auth_users_oauth_audit.sql"),
    ),
];

/// Apply all pending migrations. Idempotent.
pub fn run(conn: &Connection) -> anyhow::Result<()> {
    // Ensure the bookkeeping table exists. `0001_init.sql` creates it,
    // but we may run against a pre-bootstrap DB; create it first.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL UNIQUE,
            applied_at  INTEGER NOT NULL
        );",
    )
    .context("create _migrations table")?;

    for (name, sql) in MIGRATIONS {
        let already: bool = conn
            .query_row("SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = ?1)", [name], |r| {
                r.get(0)
            })
            .unwrap_or(false);

        if already {
            tracing::debug!(migration = name, "skipping already-applied");
            continue;
        }

        tracing::info!(migration = name, "applying");
        conn.execute_batch(sql).with_context(|| format!("apply migration {name}"))?;

        let now = OffsetDateTime::now_utc().unix_timestamp();
        conn.execute(
            "INSERT INTO _migrations (name, applied_at) VALUES (?1, ?2)",
            rusqlite::params![name, now],
        )
        .with_context(|| format!("record migration {name}"))?;
    }

    backfill_legacy_examples(conn)?;
    Ok(())
}

fn backfill_legacy_examples(conn: &Connection) -> anyhow::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.request_example, c.response_example, c.created_at, c.updated_at
         FROM contracts c
         WHERE c.request_example IS NOT NULL
           AND c.response_example IS NOT NULL
           AND NOT EXISTS (
               SELECT 1 FROM contract_examples e WHERE e.contract_id = c.id
           )",
    )?;
    let rows: Vec<(String, String, String, i64, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    for (contract_id, request, response, created_at, updated_at) in rows {
        conn.execute(
            "INSERT INTO contract_examples
                (id, contract_id, name, kind, status_code, request, response, ordinal, created_at, updated_at)
             VALUES (?1, ?2, 'default', 'positive', 200, ?3, ?4, 0, ?5, ?6)",
            rusqlite::params![
                uuid::Uuid::now_v7().to_string(),
                contract_id,
                request,
                response,
                created_at,
                updated_at,
            ],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::db::pool::open_memory;

    #[test]
    fn migrations_are_idempotent() {
        let db = open_memory().unwrap();
        // Open again — should not re-apply or error.
        let conn = db.conn();
        run(&conn).unwrap();
        run(&conn).unwrap();

        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM _migrations", [], |r| r.get(0)).unwrap();
        assert_eq!(count, MIGRATIONS.len() as i64);
    }

    #[test]
    fn backfills_complete_legacy_examples_idempotently() {
        let db = open_memory().unwrap();
        db.with_conn(|conn| {
            let project_id = uuid::Uuid::now_v7().to_string();
            let contract_id = uuid::Uuid::now_v7().to_string();
            conn.execute(
                "INSERT INTO projects (id, slug, name, created_at) VALUES (?1, 'legacy', 'Legacy', 1)",
                [&project_id],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO contracts (
                    id, project_id, method, path, summary, request_example,
                    response_schema, response_example, status, tags, created_at, updated_at
                ) VALUES (?1, ?2, 'GET', '/legacy', 'Legacy endpoint', ?3, ?4, ?5, 'draft', '[]', 1, 1)",
                rusqlite::params![
                    &contract_id,
                    &project_id,
                    r#"{"input":1}"#,
                    r#"{"type":"object"}"#,
                    r#"{"output":true}"#,
                ],
            )
            .unwrap();

            backfill_legacy_examples(conn).unwrap();
            backfill_legacy_examples(conn).unwrap();
            let row: (String, String, i64, String, String) = conn
                .query_row(
                    "SELECT name, kind, status_code, request, response
                     FROM contract_examples WHERE contract_id = ?1",
                    [&contract_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )
                .unwrap();
            assert_eq!(row, ("default".to_owned(), "positive".to_owned(), 200, r#"{"input":1}"#.to_owned(), r#"{"output":true}"#.to_owned()));
        });
    }

    #[test]
    fn all_tables_exist() {
        let db = open_memory().unwrap();
        db.with_conn(|c| {
            for table in [
                "projects",
                "groups",
                "contracts",
                "auth_tokens",
                "contract_embeddings",
                "contract_examples",
                "_migrations",
            ] {
                let exists: bool = c
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type IN ('table','view') AND name = ?1)",
                        [table],
                        |r| r.get(0),
                    )
                    .unwrap();
                assert!(exists, "table {table} should exist");
            }
        });
    }
}
