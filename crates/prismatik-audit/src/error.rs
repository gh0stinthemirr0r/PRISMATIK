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
    /// `prev_hash` on a tendered entry did not match the current ledger tip.
    ///
    /// Append is the only mutation path; chaining is enforced structurally.
    #[error("prev_hash mismatch: expected {expected}, got {got}")]
    PrevHashMismatch {
        /// Hash the ledger expected as the next `prev_hash`.
        expected: String,
        /// Hash the caller supplied.
        got: String,
    },
    /// Requested leaf position is outside the current tree.
    #[error("position {position} out of range for tree size {tree_size}")]
    PositionOutOfRange {
        /// Requested zero-based leaf position.
        position: u64,
        /// Current tree leaf count.
        tree_size: u64,
    },
    /// Consistency proof requested with `from` larger than `to`.
    #[error("inconsistent consistency sizes: from={from}, to={to}")]
    InconsistentSizes {
        /// `from` tree size.
        from: u64,
        /// `to` tree size.
        to: u64,
    },
    /// Ledger integrity check failed (chain, Merkle root, or proof mismatch).
    #[error("audit integrity failure: {0}")]
    Integrity(String),
}
