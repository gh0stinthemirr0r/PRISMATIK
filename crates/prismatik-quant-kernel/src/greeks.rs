//! Greeks contracts.

use serde::{Deserialize, Serialize};

/// Greek axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Greek {
    /// dPrice / dSpot.
    Delta,
    /// dDelta / dSpot.
    Gamma,
    /// dPrice / dVol.
    Vega,
    /// dPrice / dTime.
    Theta,
    /// dPrice / dRate.
    Rho,
}

/// Collection of option Greeks.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Greeks {
    /// Delta.
    pub delta: f64,
    /// Gamma.
    pub gamma: f64,
    /// Vega.
    pub vega: f64,
    /// Theta.
    pub theta: f64,
    /// Rho.
    pub rho: f64,
}
