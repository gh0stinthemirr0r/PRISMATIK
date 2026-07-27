//! Repository abstraction and transaction contract.

use async_trait::async_trait;
use thiserror::Error;

/// Repository operation errors.
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// Backend failure.
    #[error("storage backend failure: {0}")]
    Backend(String),
    /// Transaction conflict.
    #[error("storage conflict: {0}")]
    Conflict(String),
    /// Input validation failure.
    #[error("invalid storage input: {0}")]
    InvalidInput(String),
}

/// Repository transaction contract.
#[async_trait]
pub trait RepositoryTx: Send + Sync {
    /// Commit transaction.
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
    /// Rollback transaction.
    async fn rollback(self: Box<Self>) -> Result<(), RepositoryError>;
}

/// Root repository contract.
#[async_trait]
pub trait Repository: Send + Sync {
    /// Begin a transaction.
    async fn begin_tx(&self) -> Result<Box<dyn RepositoryTx>, RepositoryError>;
    /// Health-check backend availability.
    async fn health_check(&self) -> Result<(), RepositoryError>;
}
