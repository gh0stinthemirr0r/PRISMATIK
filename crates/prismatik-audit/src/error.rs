//! Audit errors.

use thiserror::Error;

/// Audit subsystem errors.
#[derive(Debug, Error)]
pub enum AuditError {
    /// Ledger storage failed.
    #[error("audit storage error: {0}")]
    Storage(String),
    /// Tree/proof validation failed.
    #[error("audit proof validation failed: {0}")]
    Proof(String),
    /// Invalid caller input.
    #[error("audit invalid input: {0}")]
    InvalidInput(String),
}
