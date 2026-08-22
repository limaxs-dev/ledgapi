//! Infrastructure adapters — no business rules, no use cases. Only
//! implements `domain::ports::*` against concrete backends.
//!
//! `infra` may import `core`, `domain`, `rusqlite`, `fastembed`, and
//! `axum` (for middleware). It must NOT import `mcp` or `web`.

pub mod db;
pub mod embeddings;
pub mod repos;
