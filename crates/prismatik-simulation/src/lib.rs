//! # prismatik-prismatik-simulation
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — simulation contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use sim::{
    MonteCarloConfig, MonteCarloEngine, MonteCarloError, MonteCarloResult, SimulationSource,
};

pub mod sim;
