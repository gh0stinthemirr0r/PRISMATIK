//! # prismatik-prismatik-manifest
//!
//! Layer 1 — Kernel extension
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — foundational manifest contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use bundle::{BundleError, ResearchBundle};
pub use manifest::{Manifest, ManifestBuilder, ManifestV1};
pub use verify::{StandaloneVerifier, VerificationReport};

pub mod bundle;
pub mod manifest;
pub mod verify;
