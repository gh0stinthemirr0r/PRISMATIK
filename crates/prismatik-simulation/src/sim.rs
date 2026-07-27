//! Monte Carlo simulation contracts.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Simulation source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationSource {
    /// Run from backtest result distribution.
    BacktestReplay,
    /// Run from synthetic process.
    SyntheticProcess,
}

/// Monte Carlo configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonteCarloConfig {
    /// Source of simulated returns.
    pub source: SimulationSource,
    /// Number of paths.
    pub paths: u64,
    /// Number of steps per path.
    pub steps: u32,
    /// Deterministic seed.
    pub seed: u64,
}

/// Monte Carlo result summary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MonteCarloResult {
    /// Mean terminal value.
    pub mean_terminal_value: f64,
    /// P05 terminal value.
    pub p05_terminal_value: f64,
    /// P95 terminal value.
    pub p95_terminal_value: f64,
}

/// Monte Carlo failures.
#[derive(Debug, Error)]
pub enum MonteCarloError {
    /// Invalid config.
    #[error("invalid monte carlo config: {0}")]
    InvalidConfig(String),
}

/// Monte Carlo engine interface.
pub trait MonteCarloEngine: Send + Sync {
    /// Execute simulation.
    fn run(&self, config: &MonteCarloConfig) -> Result<MonteCarloResult, MonteCarloError>;
}
