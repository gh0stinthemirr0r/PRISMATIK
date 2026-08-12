//! # prismatik-prismatik-indicator-core
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — indicator contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use conformance::{
    run_case, CandidateImpl, ConformanceCase, ReferenceImpl, NUMERICAL_TOLERANCE,
};
pub use golden_vectors::GoldenVector;
pub use indicator::{Bar, Indicator, IndicatorDescriptor, IndicatorError, WarmupBehavior};
pub use native_indicators::*;
pub use registry::{IndicatorRegistry, NativeIndicatorKind};
pub use sampler::{BarInterval, BarSampler};

/// Batch/stream conformance harness.
pub mod conformance;
pub mod golden_vectors;
pub mod indicator;
/// Built-in native technical indicators.
pub mod native_indicators;
/// Native indicator catalog.
pub mod registry;
pub mod sampler;
pub mod yata_adapter;
