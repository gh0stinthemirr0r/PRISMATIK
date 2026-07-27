//! # prismatik-prismatik-audit
//!
//! Layer 1 — Kernel extension
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — foundational audit contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use error::AuditError;
pub use proof::{ConsistencyProof, InclusionProof, SignedTreeHead, TreeHead, VerificationReport};
pub use redacted::RedactedJson;
pub use trait_def::{
    Actor, AuditAction, AuditEntry, AuditLedger, AuditReceipt, Outcome, SubjectRef,
};

pub mod backends;
pub mod error;
pub mod proof;
pub mod redacted;
pub mod trait_def;
