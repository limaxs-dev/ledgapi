//! Port traits — abstract interfaces between domain and infrastructure.
//!
//! Every `use_case` calls these traits via `&dyn` or generics; concrete
//! adapters in `infra::repos::*` implement them against SQLite / sqlite-vec.

use crate::core::id::Id;
use crate::domain::contract::{
    Contract, ContractCreate, ContractSummary, ContractUpdate, Method, Status,
};
use crate::domain::errors::DomainError;
use crate::domain::group::{Group, GroupRef, GroupSummary};
use crate::domain::project::{Project, ProjectCreate, ProjectSlug, ProjectSummary};
use async_trait::async_trait;

/// Search-mode flag for [`ContractRepo::search`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Exact,
    Semantic,
    Hybrid,
}

impl SearchMode {
    /// Parse from the MCP tool's `search_mode` argument.
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        match s {
            "exact" => Ok(Self::Exact),
            "semantic" => Ok(Self::Semantic),
            "hybrid" => Ok(Self::Hybrid),
            _ => Err(DomainError::Validation {
                field: "search_mode".to_owned(),
                message: "must be one of: exact, semantic, hybrid".to_owned(),
            }),
        }
    }
}

/// Result entry returned by the hybrid-search merge.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub id: Id,
    pub method: Method,
    pub path: String,
    pub summary: String,
    pub status: Status,
    pub tags: Vec<String>,
    /// Group name (populated by `search_contract::execute` after the
    /// post-hydration step; None until the use case runs `find_by_id`).
    pub group_name: Option<String>,
    /// Cosine similarity in `[0.0, 1.0]`. `None` for exact-only hits.
    pub similarity: Option<f32>,
}

/// Filter set for listing contracts.
#[derive(Debug, Clone, Default)]
pub struct ListContractsFilter {
    pub group_id: Option<Id>,
    pub status: Option<Status>,
    pub limit: i64,
}

/// Project repository.
#[async_trait]
pub trait ProjectRepo: Send + Sync {
    async fn create(&self, input: &ProjectCreate) -> Result<Project, DomainError>;
    async fn find_by_slug(&self, slug: &ProjectSlug) -> Result<Option<Project>, DomainError>;
    async fn find_by_id(&self, id: Id) -> Result<Option<Project>, DomainError>;
    async fn list_with_counts(&self) -> Result<Vec<ProjectSummary>, DomainError>;
}

/// Group repository.
#[async_trait]
pub trait GroupRepo: Send + Sync {
    /// Find a group by `(project_id, name)` or create it if absent.
    async fn resolve(&self, project_id: Id, input: &GroupRef) -> Result<Group, DomainError>;
    /// Look up a group by `(project_id, name)` without creating. Returns
    /// `None` if the group does not exist. Use this for read-side
    /// filters (`list_contracts`, `search_contract`) where side effects
    /// would be surprising.
    async fn find_by_name(
        &self,
        project_id: Id,
        name: &str,
    ) -> Result<Option<Group>, DomainError>;
    async fn list_with_counts(&self, project_id: Id) -> Result<Vec<GroupSummary>, DomainError>;
}

/// Contract repository.
#[async_trait]
pub trait ContractRepo: Send + Sync {
    async fn create(
        &self,
        project_id: Id,
        group_id: Option<Id>,
        input: &ContractCreate,
    ) -> Result<Contract, DomainError>;

    async fn find_by_id(&self, project_id: Id, contract_id: Id) -> Result<Contract, DomainError>;

    async fn update(
        &self,
        project_id: Id,
        contract_id: Id,
        patch: &ContractUpdate,
        group_id: Option<Id>,
    ) -> Result<Contract, DomainError>;

    async fn delete(&self, project_id: Id, contract_id: Id) -> Result<(), DomainError>;

    async fn list(
        &self,
        project_id: Id,
        filter: &ListContractsFilter,
    ) -> Result<Vec<ContractSummary>, DomainError>;

    /// Search by method/path substring (exact branch).
    async fn search_exact(
        &self,
        project_id: Id,
        group_id: Option<Id>,
        query: &str,
        limit: i64,
    ) -> Result<Vec<SearchResult>, DomainError>;

    /// Top-K semantic neighbors (semantic branch).
    async fn search_semantic(
        &self,
        project_id: Id,
        group_id: Option<Id>,
        query_embedding: &[f32],
        k: i64,
    ) -> Result<Vec<(Id, f32)>, DomainError>;

    /// All contracts in the project whose embedding exists. Used by the
    /// dup-check on create. `k` is the candidate count.
    async fn top_k_similar(
        &self,
        project_id: Id,
        query_embedding: &[f32],
        k: i64,
    ) -> Result<Vec<(Id, f32)>, DomainError>;
}

/// Embedding index (sqlite-vec virtual table).
#[async_trait]
pub trait EmbeddingRepo: Send + Sync {
    /// Insert (or replace) a contract's embedding.
    async fn upsert(
        &self,
        contract_id: Id,
        project_id: Id,
        embedding: &[f32],
    ) -> Result<(), DomainError>;
    /// Remove an embedding by contract id.
    async fn delete(&self, contract_id: Id) -> Result<(), DomainError>;
}

/// Token repository.
#[async_trait]
pub trait TokenRepo: Send + Sync {
    /// True if any row exists with the given sha256 hex.
    async fn exists(&self, hash_hex: &str) -> Result<bool, DomainError>;
    /// Insert a new token row. `label` is for human reference only.
    async fn insert(&self, hash_hex: &str, label: Option<&str>) -> Result<(), DomainError>;
    /// Number of tokens in the table (used by first-run check).
    async fn count(&self) -> Result<i64, DomainError>;
}

/// Embedder (fastembed-rs in infra, StubEmbedder in tests).
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Embed a single text. Returns a vector of `dimension()` floats.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, DomainError>;
    /// Embedding dimensionality (384 for all-MiniLM-L6-v2).
    fn dimension(&self) -> usize;
}

/// Marker trait for repo bundles. Concrete bundles in `infra::repos::*`.
pub trait Repos: Send + Sync {
    fn projects(&self) -> &dyn ProjectRepo;
    fn groups(&self) -> &dyn GroupRepo;
    fn contracts(&self) -> &dyn ContractRepo;
    fn embeddings(&self) -> &dyn EmbeddingRepo;
    fn tokens(&self) -> &dyn TokenRepo;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_mode_parses_all_values() {
        assert_eq!(SearchMode::parse("exact").unwrap(), SearchMode::Exact);
        assert_eq!(SearchMode::parse("semantic").unwrap(), SearchMode::Semantic);
        assert_eq!(SearchMode::parse("hybrid").unwrap(), SearchMode::Hybrid);
        assert!(SearchMode::parse("OTHER").is_err());
    }

    #[test]
    fn list_filter_default_has_unbounded_limit() {
        let f = ListContractsFilter::default();
        assert_eq!(f.limit, 0);
        assert!(f.group_id.is_none());
        assert!(f.status.is_none());
    }
}
