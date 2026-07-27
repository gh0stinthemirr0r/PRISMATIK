//! Pricing contracts.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Price value wrapper.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Price(pub f64);

/// Supported pricing models.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PricingModel {
    /// Black-Scholes model.
    BlackScholes,
    /// Binomial tree model.
    Binomial,
}

/// Pricing failures.
#[derive(Debug, Error)]
pub enum PricingError {
    /// Invalid model/input combination.
    #[error("invalid pricing input: {0}")]
    InvalidInput(String),
    /// Numerical method did not converge.
    #[error("pricing convergence failed")]
    Convergence,
}
