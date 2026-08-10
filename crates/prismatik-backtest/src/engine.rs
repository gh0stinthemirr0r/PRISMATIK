//! Backtest engine contracts.

use crate::assumptions::ExecutionAssumptions;
use crate::metrics::BacktestMetrics;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

/// Inclusive-exclusive time window for a point-in-time-safe backtest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BacktestWindow {
    /// First observable instant.
    pub start: OffsetDateTime,
    /// Exclusive ending instant.
    pub end: OffsetDateTime,
}

/// Backtest configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BacktestConfig {
    /// Canonical strategy IR identifier.
    pub strategy_id: String,
    /// Evaluation window.
    pub window: BacktestWindow,
    /// Starting capital in currency micros.
    pub starting_capital_micros: i64,
    /// Execution assumptions.
    pub execution: ExecutionAssumptions,
    /// Bar interval in seconds.
    pub bar_interval_seconds: u64,
}

/// Backtest result bundle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BacktestResult {
    /// Config hash.
    pub config_hash: String,
    /// Metrics.
    pub metrics: BacktestMetrics,
    /// Number of deterministic bar steps evaluated.
    pub bars_processed: u64,
    /// Terminal run status.
    pub status: String,
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

/// Deterministic local backtest floor used to produce reproducible manifests.
#[derive(Clone, Debug)]
pub struct BacktestEngine {
    config: BacktestConfig,
}

impl BacktestEngine {
    /// Validate and construct an engine.
    pub fn new(config: BacktestConfig) -> Result<Self, BacktestError> {
        if config.strategy_id.trim().is_empty() {
            return Err(BacktestError::InvalidConfig("strategy id is empty".into()));
        }
        if config.window.end <= config.window.start {
            return Err(BacktestError::InvalidConfig("window is empty".into()));
        }
        if config.starting_capital_micros <= 0 || config.bar_interval_seconds == 0 {
            return Err(BacktestError::InvalidConfig(
                "capital and bar interval must be positive".into(),
            ));
        }
        Ok(Self { config })
    }

    /// Run the deterministic no-market-data floor. It never fabricates fills.
    pub fn run_stub(&self) -> BacktestResult {
        let seconds = (self.config.window.end - self.config.window.start).whole_seconds();
        let bars_processed = u64::try_from(seconds)
            .unwrap_or_default()
            .div_ceil(self.config.bar_interval_seconds);
        BacktestResult {
            config_hash: blake3::hash(
                serde_json::to_string(&self.config)
                    .expect("backtest config serializes")
                    .as_bytes(),
            )
            .to_hex()
            .to_string(),
            metrics: BacktestMetrics {
                total_return: 0.0,
                max_drawdown: 0.0,
                deflated_sharpe: crate::metrics::DeflatedSharpe(0.0),
                profit_factor: crate::metrics::ProfitFactor(0.0),
            },
            bars_processed,
            status: "no_market_data".into(),
        }
    }

    /// Produce the deterministic floor result and its signed manifest.
    pub fn run_stub_signed(
        &self,
        produced_at: OffsetDateTime,
    ) -> Result<crate::manifest::BacktestStubRun, String> {
        let result = self.run_stub();
        let manifest = crate::manifest::BacktestManifestBuilder::new(
            self.config.clone(),
            result.clone(),
            produced_at,
        )
        .build_signed()?;
        Ok(crate::manifest::BacktestStubRun { result, manifest })
    }
}
