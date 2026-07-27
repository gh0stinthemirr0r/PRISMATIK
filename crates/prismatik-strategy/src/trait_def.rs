//! Strategy runtime trait contracts.

use crate::ir::{StrategyCapabilities, StrategyIR};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Runtime market event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketEvent {
    /// Event kind.
    pub kind: String,
    /// Event payload in canonical json text.
    pub payload_json: String,
}

/// Strategy order intent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderIntent {
    /// Asset identifier.
    pub asset_id: String,
    /// Side (`buy` / `sell`).
    pub side: String,
    /// Quantity as decimal string.
    pub quantity: String,
}

/// Execution context marker.
#[derive(Clone, Debug, Default)]
pub struct ExecutionContext;

/// Strategy context marker.
#[derive(Clone, Debug, Default)]
pub struct StrategyContext;

/// Strategy behavior contract.
#[async_trait]
pub trait Strategy: Send + Sync {
    /// Return immutable IR.
    fn ir(&self) -> &StrategyIR;
    /// Return inferred capabilities.
    fn capabilities(&self) -> &StrategyCapabilities;
    /// Handle bar update.
    async fn on_bar(
        &mut self,
        ctx: &mut StrategyContext,
        bar: &prismatik_indicator_core::Bar,
    ) -> Result<Vec<OrderIntent>, crate::RuntimeError>;
    /// Handle general event update.
    async fn on_event(
        &mut self,
        ctx: &mut StrategyContext,
        event: &MarketEvent,
    ) -> Result<Vec<OrderIntent>, crate::RuntimeError>;
}
