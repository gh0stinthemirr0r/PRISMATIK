//! Backtest engine contracts.

use crate::assumptions::ExecutionAssumptions;
use crate::metrics::BacktestMetrics;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Backtest configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BacktestConfig {
    /// Universe asset ids.
    pub universe: Vec<String>,
    /// Starting capital (decimal string).
    pub starting_capital: String,
    /// Execution assumptions.
    pub execution_assumptions: ExecutionAssumptions,
    /// Deterministic seed.
    pub seed: u64,
}

/// Backtest result bundle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BacktestResult {
    /// Config hash.
    pub config_hash: String,
    /// Metrics.
    pub metrics: BacktestMetrics,
}

/// Backtest failures.
#[derive(Debug, Error)]
pub enum BacktestError {
    /// Invalid config.
    #[error("invalid backtest config: {0}")]
    InvalidConfig(String),
    /// Runtime failure.
    #[error("backtest runtime failure: {0}")]
    Runtime(String),
}

/// Backtest engine interface.
pub trait Backtest: Send + Sync {
    /// Run a deterministic backtest.
    fn run(&self, config: &BacktestConfig) -> Result<BacktestResult, BacktestError>;
}
