//! Redacted JSON wrapper for audit details.
//!
//! The canonical type lives in [`crate::entry::RedactedJson`]: a typed,
//! order-stable map of non-secret string fields. This module re-exports it so
//! the historical import path `prismatik_audit::RedactedJson` keeps working.

pub use crate::entry::RedactedJson;
