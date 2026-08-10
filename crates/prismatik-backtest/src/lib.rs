//! # prismatik-prismatik-backtest
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — backtest contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use assumptions::{CommissionModel, ExecutionAssumptions, FillModel, SlippageModel};
pub use engine::{
    Backtest, BacktestConfig, BacktestEngine, BacktestError, BacktestResult, BacktestWindow,
};
pub use fill::{Fill, FillKind};
pub use metrics::{BacktestMetrics, DeflatedSharpe, ProfitFactor, WalkForwardResult};

/// Bar-driven simulation engine with real market-data path.
pub mod simulation;
/// Performance statistics (Sharpe, Sortino, Calmar, drawdown, profit factor).
pub mod stats;
/// Seed strategy library (ORB, momentum, mean reversion, Donchian).
pub mod strategies;

pub use simulation::{
    simulate, simulate_weighted, EquityPoint, OhlcBar, Signal, SignalStrategy, SimFill, SimTrade,
    SimulationResult,
};
pub use stats::{calmar_ratio, sharpe_ratio, sortino_ratio, CalendarKind};
pub use strategies::{Donchian, MeanReversion, Momentum, OpeningRangeBreakout};

pub mod assumptions;
pub mod engine;
pub mod fill;
pub mod manifest;
pub mod metrics;

pub use manifest::{BacktestExtension, BacktestManifestBuilder, BacktestStubRun};
