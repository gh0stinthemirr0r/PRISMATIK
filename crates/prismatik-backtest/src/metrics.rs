//! Backtest metrics contracts.

use serde::{Deserialize, Serialize};

/// Deflated Sharpe ratio wrapper.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DeflatedSharpe(pub f64);

/// Profit factor wrapper.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ProfitFactor(pub f64);

/// Backtest metrics set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BacktestMetrics {
    /// Cumulative return.
    pub total_return: f64,
    /// Maximum drawdown.
    pub max_drawdown: f64,
    /// Deflated sharpe.
    pub deflated_sharpe: DeflatedSharpe,
    /// Profit factor.
    pub profit_factor: ProfitFactor,
}

/// Walk-forward result summary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WalkForwardResult {
    /// Number of windows evaluated.
    pub windows: u32,
    /// Aggregate metrics.
    pub metrics: BacktestMetrics,
}
