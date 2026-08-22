//! Business domain — entities, value objects, ports, use cases.
//!
//! This module must NOT import `infra`, `mcp`, `web`, `axum`, `rusqlite`,
//! or `fastembed`. Enforced by `tests/architecture.rs`.

pub mod contract;
pub mod errors;
pub mod group;
pub mod ports;
pub mod project;
pub mod use_cases;

pub use contract::{
    AuthType, Contract, ContractCreate, ContractSummary, ContractUpdate, Method, Status,
    normalize_path,
};
pub use errors::DomainError;
pub use group::{Group, GroupRef, GroupSummary};
pub use ports::{
    ContractRepo, Embedder, EmbeddingRepo, GroupRepo, ListContractsFilter, ProjectRepo, Repos,
    SearchMode, SearchResult, TokenRepo,
};
pub use project::{Project, ProjectCreate, ProjectSlug, ProjectSummary};
