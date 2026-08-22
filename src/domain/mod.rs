//! Business domain — entities, value objects, ports, use cases.
//!
//! This module must NOT import `infra`, `mcp`, `web`, `axum`, `rusqlite`,
//! or `fastembed`. Enforced by `tests/architecture.rs`.

pub mod errors;

pub use errors::DomainError;
