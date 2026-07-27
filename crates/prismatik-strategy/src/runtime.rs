//! Strategy runtime contracts.

use crate::trait_def::ExecutionContext;
use thiserror::Error;

/// Strategy runtime error.
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Runtime precondition failed.
    #[error("strategy runtime precondition failed: {0}")]
    Preconditions(String),
    /// Strategy callback failed.
    #[error("strategy callback failed: {0}")]
    Callback(String),
}

/// Runtime marker.
#[derive(Debug, Default, Clone)]
pub struct StrategyRuntime {
    /// Runtime execution context.
    pub context: ExecutionContext,
}
