//! # prismatik-prismatik-backtest
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — backtest contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use assumptions::{CommissionModel, ExecutionAssumptions, FillModel, SlippageModel};
pub use engine::{Backtest, BacktestConfig, BacktestError, BacktestResult};
pub use fill::{Fill, FillKind};
pub use metrics::{BacktestMetrics, DeflatedSharpe, ProfitFactor, WalkForwardResult};

pub mod assumptions;
pub mod engine;
pub mod fill;
pub mod metrics;
