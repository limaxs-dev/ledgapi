//! Business domain — entities, value objects, ports, use cases.
//!
//! This module must NOT import `infra`, `mcp`, `web`, `axum`, `rusqlite`,
//! or `fastembed`. Enforced by `tests/architecture.rs`.

pub mod audit;
pub mod auth;
pub mod contract;
pub mod errors;
pub mod group;
pub mod ports;
pub mod project;
pub mod use_cases;

pub use audit::{AuditAction, AuditEntry, AuditEvent, AuditFilter, AuditResource};
pub use auth::{
    AuthorizationCode, OAuthClient, OAuthToken, Principal, RefreshToken, Role, Session, User,
    UserCreate,
};
pub use contract::{
    AuthType, Contract, ContractCreate, ContractExample, ContractExampleInput, ContractSummary,
    ContractUpdate, ExampleKind, Method, Status, normalize_path,
};
pub use errors::DomainError;
pub use group::{Group, GroupRef, GroupSummary};
pub use ports::{
    AuditRepo, ContractRepo, Embedder, EmbeddingRepo, GroupRepo, ListContractsFilter, OAuthRepo,
    ProjectRepo, Repos, SearchMode, SearchResult, SessionRepo, UserRepo,
};
pub use project::{Project, ProjectCreate, ProjectSlug, ProjectSummary};
