//! # prismatik-prismatik-domain
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — shared domain primitives.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use currency::CurrencyCode;
pub use dataset_version::{DatasetVersionId, DatasetVersionRef};
pub use layer::DataLayer;
pub use provider_id::ProviderId;
pub use quality::DataQualityScore;

pub mod currency;
pub mod dataset_version;
pub mod layer;
pub mod provider_id;
pub mod quality;
