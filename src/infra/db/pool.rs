//! SQLite connection wrapper. Single `Arc<Mutex<Connection>>` per
//! spec §13 #1. Loads sqlite-vec on construction.

use crate::config::DatabaseConfig;
use crate::infra::db::migrations;
use anyhow::Context;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;

/// Single shared database connection.
///
/// Cheap to clone (`Arc`); every operation acquires the mutex briefly.
/// SQLite WAL allows concurrent reads but a single writer; the mutex
/// makes that serialization explicit.
#[derive(Clone)]
pub struct Db {
    inner: Arc<Mutex<Connection>>,
}

impl Db {
    /// Borrow the underlying connection for the duration of a closure.
    /// Panics on lock poisoning (we never panic across threads; lock
    /// poisoning indicates a `panic!` already happened).
    pub fn with_conn<R>(&self, f: impl FnOnce(&mut Connection) -> R) -> R {
        let mut guard = self.inner.lock();
        f(&mut guard)
    }

    /// Borrow the underlying connection without holding the lock across
    /// an await point. **Do not** use this for multi-statement work.
    /// Prefer [`Self::with_conn`].
    pub fn conn(&self) -> parking_lot::MappedMutexGuard<'_, Connection> {
        parking_lot::MutexGuard::map(self.inner.lock(), |c| c)
    }
}

/// Open the database at `cfg.path`. Auto-creates the parent directory.
/// Loads sqlite-vec. Applies PRAGMAs. Runs migrations.
///
/// # Errors
/// Returns `Err` on I/O failure, sqlite-vec load failure, or migration
/// failure.
pub fn open(cfg: &DatabaseConfig) -> anyhow::Result<Db> {
    let path = Path::new(&cfg.path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create parent dir {}", parent.display()))?;
        }
    }

    // Register sqlite-vec before opening so the extension is loaded
    // by the SQLite runtime for the brand-new connection below.
    register_sqlite_vec()?;

    let conn =
        Connection::open(&cfg.path).with_context(|| format!("open sqlite at {}", cfg.path))?;

    apply_pragmas(&conn, cfg.busy_timeout_ms)?;
    migrations::run(&conn)?;

    Ok(Db { inner: Arc::new(Mutex::new(conn)) })
}

fn apply_pragmas(conn: &Connection, busy_timeout_ms: u64) -> anyhow::Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL").context("PRAGMA journal_mode = WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON").context("PRAGMA foreign_keys = ON")?;
    conn.pragma_update(None, "busy_timeout", busy_timeout_ms as i64)
        .context("PRAGMA busy_timeout")?;
    Ok(())
}

// The sqlite-vec Rust binding (v0.1.x) exposes `sqlite3_vec_init` as a
// C extern symbol; the documented registration path is to transmute the
// symbol into the function-pointer form that `sqlite3_auto_extension`
// expects and call it through FFI. There is no safe wrapper in the
// binding, so this function is the single `unsafe` site in `src/`.
// The architecture test (Task 42) explicitly allows `unsafe` in
// `src/infra/db/pool.rs`.
#[allow(unsafe_code)]
fn register_sqlite_vec() -> anyhow::Result<()> {
    // SQLite keeps auto-extensions in a process-global linked list;
    // re-registering the same pointer is harmless, so it's safe to
    // call before every connection open.
    //
    // SAFETY: registering an extension via `sqlite3_auto_extension` is
    // the documented `sqlite-vec` Rust binding API. The transmute is
    // required because `sqlite3_vec_init` is declared as a void extern
    // symbol but the FFI signature expects a 3-arg SQLite entry-point.
    type SqliteEntryPoint = unsafe extern "C" fn(
        *mut rusqlite::ffi::sqlite3,
        *mut *mut std::os::raw::c_char,
        *const rusqlite::ffi::sqlite3_api_routines,
    ) -> std::os::raw::c_int;
    let entry: SqliteEntryPoint =
        unsafe { std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ()) };
    let rc = unsafe { rusqlite::ffi::sqlite3_auto_extension(Some(entry)) };
    if rc != 0 {
        return Err(anyhow::anyhow!("sqlite3_auto_extension returned {rc} for sqlite-vec"));
    }
    Ok(())
}

/// Open an in-memory database. Used by integration tests.
pub fn open_memory() -> anyhow::Result<Db> {
    // Register sqlite-vec before opening so the extension is loaded
    // by the SQLite runtime for the brand-new connection below.
    register_sqlite_vec()?;
    let conn = Connection::open_in_memory().context("open in-memory sqlite")?;
    apply_pragmas(&conn, default_busy_timeout())?;
    migrations::run(&conn)?;
    Ok(Db { inner: Arc::new(Mutex::new(conn)) })
}

fn default_busy_timeout() -> u64 {
    5000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_memory_works() {
        let db = open_memory().unwrap();
        db.with_conn(|c| {
            // SQLite version sanity check
            let v: String = c.query_row("SELECT sqlite_version()", [], |r| r.get(0)).unwrap();
            assert!(!v.is_empty());
        });
    }

    #[test]
    fn open_memory_loads_sqlite_vec() {
        let db = open_memory().unwrap();
        db.with_conn(|c| {
            // Smoke test that vec0 is available
            c.execute_batch("CREATE VIRTUAL TABLE t USING vec0(x float[4])").unwrap();
            let bytes: Vec<u8> =
                [0.0_f32, 0.0, 0.0, 1.0].iter().flat_map(|f| f.to_le_bytes()).collect();
            c.execute("INSERT INTO t (rowid, x) VALUES (1, ?1)", rusqlite::params![bytes]).unwrap();
        });
    }

    #[test]
    fn open_creates_parent_dir() {
        let tmp = tempdir();
        let path = tmp.join("sub/dir/ledgapi.db");
        let cfg =
            DatabaseConfig { path: path.to_string_lossy().into_owned(), busy_timeout_ms: 1000 };
        let _db = open(&cfg).unwrap();
        assert!(path.parent().unwrap().exists());
        assert!(path.exists());
    }

    // Tiny stand-in for `tempfile::tempdir` to avoid adding a dev-dep.
    fn tempdir() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("ledgapi-test-{}", std::process::id()));
        p.push(format!(
            "{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
