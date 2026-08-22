//! Pure types — no I/O, no async, no env access.
//!
//! Anything imported here must be in the workspace deps as a "core-only"
//! dep: `serde`, `time`, `uuid`, `thiserror`.

pub mod id;
