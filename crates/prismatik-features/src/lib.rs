//! # prismatik-prismatik-features
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — feature store contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use observation::ObservationDelay;
pub use store::{EntityTimeFrame, FeatureError, FeatureStore, MaterializationReport};
pub use view::{EntityKind, FeatureSpec, FeatureView, FeatureViewId, OfflineStore, OnlineStore};

pub mod observation;
pub mod store;
pub mod view;
