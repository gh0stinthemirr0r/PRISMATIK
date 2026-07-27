//! # prismatik-prismatik-storage
//!
//! Layer 1 — Kernel extension
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — foundational storage contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use backends::{DuckdbBackend, LancedbBackend, SqliteBackend};
pub use profile::{CloudProfile, DesktopProfile, EnterpriseProfile, StorageProfile};
pub use repository::{Repository, RepositoryError, RepositoryTx};

pub mod backends;
pub mod migrations;
pub mod profile;
pub mod repository;
