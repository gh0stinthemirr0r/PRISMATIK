//! Audit ledger trait and entry types — historical re-export surface.
//!
//! The canonical definitions live in [`crate::entry`] (entry types) and
//! [`crate::ledger`] (the [`AuditLedger`] trait and [`InMemoryAuditLedger`]).
//! This module re-exports them under the original flat paths so existing call
//! sites (`prismatik_audit::AuditLedger`, `prismatik_audit::Actor`, …) keep
//! compiling against the richer, current contract.

pub use crate::entry::{Actor, AuditAction, AuditEntry, AuditReceipt, Outcome, SubjectRef};
pub use crate::ledger::AuditLedger;
