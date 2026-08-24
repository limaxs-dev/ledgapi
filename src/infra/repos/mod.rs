//! Repository adapters — implement `domain::ports::*` against SQLite.
//!
//! Each submodule exposes one adapter struct per port trait. The
//! `SqliteRepos` bundle wires them together into a single handle.

pub mod audit_repo_sqlite;
pub mod contract_repo_sqlite;
pub mod embedding_repo_sqlite_vec;
pub mod group_repo_sqlite;
pub mod oauth_repo_sqlite;
pub mod project_repo_sqlite;
pub mod session_repo_sqlite;
pub mod user_repo_sqlite;

// Re-export concrete adapter structs so the [`SqliteRepos`] bundle
// below can refer to them without per-field imports.
pub use audit_repo_sqlite::SqliteAuditRepo;
pub use contract_repo_sqlite::SqliteContractRepo;
pub use embedding_repo_sqlite_vec::SqliteEmbeddingRepo;
pub use group_repo_sqlite::SqliteGroupRepo;
pub use oauth_repo_sqlite::SqliteOAuthRepo;
pub use project_repo_sqlite::SqliteProjectRepo;
pub use session_repo_sqlite::SqliteSessionRepo;
pub use user_repo_sqlite::SqliteUserRepo;

use crate::core::id::Id;
use crate::domain::ports::{
    AuditRepo, ContractRepo, EmbeddingRepo, GroupRepo, ListContractsFilter, OAuthRepo, ProjectRepo,
    Repos, SearchResult, SessionRepo, UserRepo,
};
use std::sync::Arc;

/// Bundle of all repository handles. Held in `AppState`.
#[derive(Clone)]
pub struct SqliteRepos {
    pub db: crate::infra::db::Db,
    pub projects: Arc<SqliteProjectRepo>,
    pub groups: Arc<SqliteGroupRepo>,
    pub contracts: Arc<SqliteContractRepo>,
    pub embeddings: Arc<SqliteEmbeddingRepo>,
    pub users: Arc<SqliteUserRepo>,
    pub sessions: Arc<SqliteSessionRepo>,
    pub oauth: Arc<SqliteOAuthRepo>,
    pub audit: Arc<SqliteAuditRepo>,
}

impl SqliteRepos {
    /// Construct from an opened database. Runs once at bootstrap.
    #[must_use]
    pub fn new(db: crate::infra::db::Db) -> Self {
        Self {
            projects: Arc::new(SqliteProjectRepo { db: db.clone() }),
            groups: Arc::new(SqliteGroupRepo { db: db.clone() }),
            contracts: Arc::new(SqliteContractRepo { db: db.clone() }),
            embeddings: Arc::new(SqliteEmbeddingRepo { db: db.clone() }),
            users: Arc::new(SqliteUserRepo { db: db.clone() }),
            sessions: Arc::new(SqliteSessionRepo { db: db.clone() }),
            oauth: Arc::new(SqliteOAuthRepo { db: db.clone() }),
            audit: Arc::new(SqliteAuditRepo { db: db.clone() }),
            db,
        }
    }
}

impl Repos for SqliteRepos {
    fn projects(&self) -> &dyn ProjectRepo {
        self.projects.as_ref()
    }
    fn groups(&self) -> &dyn GroupRepo {
        self.groups.as_ref()
    }
    fn contracts(&self) -> &dyn ContractRepo {
        self.contracts.as_ref()
    }
    fn embeddings(&self) -> &dyn EmbeddingRepo {
        self.embeddings.as_ref()
    }
    fn users(&self) -> &dyn UserRepo {
        self.users.as_ref()
    }
    fn sessions(&self) -> &dyn SessionRepo {
        self.sessions.as_ref()
    }
    fn oauth(&self) -> &dyn OAuthRepo {
        self.oauth.as_ref()
    }
    fn audit(&self) -> &dyn AuditRepo {
        self.audit.as_ref()
    }
}

// Convenience: silence "unused" for `SearchResult` / `ListContractsFilter` /
// `Id` until use cases land and consume them.
#[allow(dead_code)]
fn _search_result_marker(_: SearchResult) {}
#[allow(dead_code)]
fn _list_filter_marker(_: ListContractsFilter) {}
#[allow(dead_code)]
fn _id_marker(_: Id) {}
