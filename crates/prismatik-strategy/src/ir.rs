//! Strategy IR contracts.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// IR schema version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Major version.
    pub major: u16,
    /// Minor version.
    pub minor: u16,
    /// Patch version.
    pub patch: u16,
}

/// Strategy capability flags.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyCapabilities {
    /// Reads market data.
    pub reads_market_data: bool,
    /// Emits order intents.
    pub emits_orders: bool,
}

/// Canonical strategy IR.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyIR {
    /// Stable strategy id.
    pub id: String,
    /// Human name.
    pub name: String,
    /// Schema version.
    pub schema_version: SchemaVersion,
    /// Canonicalized JSON payload.
    pub json: String,
    /// Capabilities.
    pub capabilities: StrategyCapabilities,
}

/// Strategy IR errors.
#[derive(Debug, Error)]
pub enum StrategyIRError {
    /// Invalid schema version.
    #[error("unsupported schema version")]
    UnsupportedSchema,
    /// Payload was not valid.
    #[error("invalid strategy IR payload: {0}")]
    InvalidPayload(String),
}
