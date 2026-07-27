//! Implied-volatility solver contracts.

use serde::{Deserialize, Serialize};

/// Implied-vol solver algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IvMethod {
    /// Newton-Raphson.
    NewtonRaphson,
    /// Bisection.
    Bisection,
}

/// Implied-vol solver interface.
pub trait ImpliedVolSolver: Send + Sync {
    /// Solve implied volatility from target price.
    fn solve_iv(
        &self,
        target_price: f64,
        initial_guess: f64,
        method: IvMethod,
    ) -> Result<f64, String>;
}
