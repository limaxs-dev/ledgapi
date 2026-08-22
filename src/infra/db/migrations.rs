//! SQL migration runner. Embedded via `include_str!`. Tracks applied
//! migrations in the `_migrations` table. Full implementation arrives
//! in Task 15.

/// Apply all pending migrations. Stub for Task 14 — no-op until the
/// migration files land in Task 15.
pub fn run(_conn: &rusqlite::Connection) -> anyhow::Result<()> {
    Ok(())
}
