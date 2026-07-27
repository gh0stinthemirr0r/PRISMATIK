//! `Strategy` trait floor and null implementation (`P4-QM-06` prep).
//!
//! **Gate status:** Wave 3 is BLOCKED on incomplete `P0-REMAINDER`.
//! Sync callbacks, stub events/intents, and `NullStrategy` are scaffolding
//! only. No DSL parser, async runtime, or order path is claimed here.

use thiserror::Error;

use crate::context::StrategyContext;
use crate::strategy_ir::{StrategyCapabilities, StrategyIr};
use prismatik_indicator_core::Bar;

/// Strategy runtime / callback errors (floor variants only).
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum StrategyError {
    /// Strategy rejected the invocation (generic floor).
    #[error("strategy error: {0}")]
    Message(String),
    /// Capability or context misuse (enforcement deferred to Wave 3 runtime).
    #[error("capability violation: {0}")]
    CapabilityViolation(String),
}

/// Order intent placeholder — strategies emit intents, never live orders.
///
/// Full OMS fields (side, qty, instrument, TIF, …) arrive when Wave 3 opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderIntent {
    /// Opaque instrument id (empty for null strategies).
    pub instrument_id: String,
    /// Signed quantity in integer lots (0 = no-op placeholder).
    pub quantity: i64,
}

impl OrderIntent {
    /// Construct a no-op intent (useful in tests).
    pub fn empty() -> Self {
        Self {
            instrument_id: String::new(),
            quantity: 0,
        }
    }
}

/// Market-event placeholder until typed events land in Wave 3.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarketEvent {
    /// Opaque event kind label (e.g. `"bar"`, `"print"`).
    pub kind: String,
}

/// Normative strategy surface (sync floor; async deferred).
///
/// Spec target: `DOCS/spec/STRATEGY_IR.md` §10 / `CRATE_ARCHITECTURE.md` §3.7.
/// This prep omits `async_trait` and full data/OMS wiring.
pub trait Strategy: Send + Sync {
    /// Compiled IR this instance was created from.
    fn ir(&self) -> &StrategyIr;

    /// Capability declaration (mirrors IR; runtime enforces later).
    fn capabilities(&self) -> &StrategyCapabilities {
        &self.ir().capabilities
    }

    /// Stable strategy id.
    fn id(&self) -> &str {
        &self.ir().strategy_id
    }

    /// Per-bar callback. Floor returns intents only; no fills.
    fn on_bar(
        &mut self,
        ctx: &StrategyContext,
        bar: &Bar,
    ) -> Result<Vec<OrderIntent>, StrategyError>;

    /// Per-event callback for non-bar market events.
    fn on_event(
        &mut self,
        ctx: &StrategyContext,
        event: &MarketEvent,
    ) -> Result<Vec<OrderIntent>, StrategyError>;
}

/// Trivial strategy that never emits intents — unit-test / wiring fixture.
#[derive(Clone, Debug)]
pub struct NullStrategy {
    ir: StrategyIr,
}

impl NullStrategy {
    /// Wrap a StrategyIR document.
    pub fn new(ir: StrategyIr) -> Self {
        Self { ir }
    }

    /// Minimal static-universe null strategy.
    pub fn minimal(strategy_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::new(StrategyIr::minimal(strategy_id, name))
    }
}

impl Strategy for NullStrategy {
    fn ir(&self) -> &StrategyIr {
        &self.ir
    }

    fn on_bar(
        &mut self,
        _ctx: &StrategyContext,
        _bar: &Bar,
    ) -> Result<Vec<OrderIntent>, StrategyError> {
        Ok(Vec::new())
    }

    fn on_event(
        &mut self,
        _ctx: &StrategyContext,
        _event: &MarketEvent,
    ) -> Result<Vec<OrderIntent>, StrategyError> {
        Ok(Vec::new())
    }
}

/// Alias emphasizing pass-through / no-op behavior.
pub type PassthroughStrategy = NullStrategy;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::StrategyContext;
    use crate::STRATEGY_IR_SCHEMA_VERSION;

    fn sample_bar() -> Bar {
        Bar {
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: 0.0,
        }
    }

    #[test]
    fn null_strategy_emits_no_orders() {
        let mut strat = NullStrategy::minimal("06d2e7e0-0000-4000-8000-000000000002", "null");
        assert_eq!(strat.id(), "06d2e7e0-0000-4000-8000-000000000002");
        assert_eq!(strat.ir().schema_version, STRATEGY_IR_SCHEMA_VERSION);
        assert!(!strat.capabilities().can_access_network);

        let ctx = StrategyContext::new(strat.id(), 0, 0, 7);
        let intents = strat.on_bar(&ctx, &sample_bar()).unwrap();
        assert!(intents.is_empty());

        let event = MarketEvent {
            kind: "print".into(),
        };
        let intents = strat.on_event(&ctx, &event).unwrap();
        assert!(intents.is_empty());
    }

    #[test]
    fn passthrough_alias_is_null_strategy() {
        let mut strat: PassthroughStrategy = NullStrategy::minimal("id", "pass");
        let ctx = StrategyContext::new(strat.id(), 1, 100, 1);
        assert!(strat.on_bar(&ctx, &sample_bar()).unwrap().is_empty());
    }
}
