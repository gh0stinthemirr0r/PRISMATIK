//! # prismatik-prismatik-indicator-core
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — indicator contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use golden_vectors::GoldenVector;
pub use indicator::{Bar, Indicator, IndicatorDescriptor, IndicatorError, WarmupBehavior};
pub use sampler::{BarInterval, BarSampler};

pub mod golden_vectors;
pub mod indicator;
pub mod sampler;
pub mod yata_adapter;
