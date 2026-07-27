//! Indicator trait and common types.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Canonical OHLCV bar input.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bar {
    /// Open price.
    pub open: f64,
    /// High price.
    pub high: f64,
    /// Low price.
    pub low: f64,
    /// Close price.
    pub close: f64,
    /// Volume.
    pub volume: f64,
}

/// Indicator descriptor metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndicatorDescriptor {
    /// Indicator id.
    pub id: String,
    /// Display name.
    pub name: String,
}

/// Warmup behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarmupBehavior {
    /// Return NaN before warmup.
    ReturnNaN,
    /// Return explicit error before warmup.
    ReturnError,
}

/// Indicator failures.
#[derive(Debug, Error)]
pub enum IndicatorError {
    /// Not enough bars.
    #[error("insufficient warmup bars")]
    Warmup,
    /// Invalid input values.
    #[error("invalid bar input: {0}")]
    InvalidInput(String),
}

/// Canonical indicator trait.
pub trait Indicator: Send + Sync {
    /// Descriptor metadata.
    fn descriptor(&self) -> &IndicatorDescriptor;
    /// Required warmup bars.
    fn warmup_len(&self) -> usize;
    /// Batch evaluation over a window.
    fn evaluate(&self, window: &[Bar]) -> Result<f64, IndicatorError>;
    /// Streaming update with next bar.
    fn next(&mut self, bar: &Bar) -> Result<f64, IndicatorError>;
}
