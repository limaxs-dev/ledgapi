//! Business domain — entities, value objects, ports, use cases.
//!
//! This module must NOT import `infra`, `mcp`, `web`, `axum`, `rusqlite`,
//! or `fastembed`. Enforced by `tests/architecture.rs`.

pub mod errors;
pub mod group;
pub mod project;

pub use errors::DomainError;
pub use group::{Group, GroupRef, GroupSummary};
pub use project::{Project, ProjectCreate, ProjectSlug, ProjectSummary};
