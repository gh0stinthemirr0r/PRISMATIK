//! # prismatik-prismatik-quant-kernel
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — quant-kernel contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use greeks::{Greek, Greeks};
pub use iv_solve::{ImpliedVolSolver, IvMethod};
pub use pricing::{Price, PricingError, PricingModel};

pub mod greeks;
pub mod iv_solve;
pub mod pricing;
