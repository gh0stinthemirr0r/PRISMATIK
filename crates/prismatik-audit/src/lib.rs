//! # prismatik-audit
//!
//! Layer 1 — Kernel extension.
//!
//! Append-only, hash-chained, Merkle-verifiable audit ledger (invariant I7).
//! Every mutation of durable state crosses this ledger; tampering is detected
//! by recomputing the chain and root, and any leaf can be proven included
//! against a published tree head with no PRISMATIK installation required.
//!
//! Spec: `DOCS/spec/CRATE_ARCHITECTURE.md`; wave plan `P0-DK-08`.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use error::AuditError;
pub use proof::{
    hash_children, hash_leaf, verify_inclusion, ConsistencyProof, InclusionProof, SignedTreeHead,
    TreeHead, VerificationReport,
};

pub mod backends;
/// Audit entry types (the canonical redaction-by-type definitions).
pub mod entry;
/// Audit errors.
pub mod error;
/// Append-only in-memory Merkle ledger and the [`AuditLedger`] trait.
pub mod ledger;
/// Merkle proof and tree-head types plus the domain-separated hash helpers.
pub mod proof;
/// Re-export of [`RedactedJson`] under its historical import path.
pub mod redacted;
/// Historical flat re-export surface for entry + ledger types.
pub mod trait_def;
/// Audit-as-write-path floor (`P6-DK-01`..`P6-DK-03`).
pub mod write_path;

// Flat crate-root re-exports. `trait_def` mirrors these for callers that
// imported from there historically; both paths resolve to the same canonical
// types in `entry` / `ledger`.
pub use entry::{Actor, AuditAction, AuditEntry, AuditReceipt, Outcome, RedactedJson, SubjectRef};
pub use ledger::{startup_verify, AuditLedger, InMemoryAuditLedger};
pub use write_path::{
    append_portfolio_write, tip_after, DivergenceAlert, PortfolioProjection, PortfolioWriteEvent,
    PortfolioWriteKind, ProjectionRebuild,
};
