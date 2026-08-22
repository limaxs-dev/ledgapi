//! SQL migration runner. Embedded via `include_str!`. Tracks applied
//! migrations in the `_migrations` table.

use anyhow::Context;
use rusqlite::Connection;
use time::OffsetDateTime;

const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("../../../migrations/0001_init.sql")),
    ("0002_contract_embeddings", include_str!("../../../migrations/0002_contract_embeddings.sql")),
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
    fn all_tables_exist() {
        let db = open_memory().unwrap();
        db.with_conn(|c| {
            for table in [
                "projects",
                "groups",
                "contracts",
                "auth_tokens",
                "contract_embeddings",
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
