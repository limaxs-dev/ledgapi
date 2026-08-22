//! Database infrastructure — connection wrapper, migrations, and
//! (later) repository implementations.

pub mod migrations;
pub mod pool;

pub use pool::{Db, open, open_memory};
