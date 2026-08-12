//! # prismatik-prismatik-strategy
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — strategy contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use ir::{
    SchemaVersion, StrategyCapabilities as LegacyStrategyCapabilities, StrategyIR, StrategyIRError,
};
pub use runtime::{RuntimeError, StrategyRuntime};
pub use trait_def::{ExecutionContext, MarketEvent, OrderIntent, Strategy, StrategyContext};

pub mod codegen;
pub mod dsl;
pub mod ir;
pub mod runtime;
pub mod trait_def;

pub mod context;
pub mod data_access;
pub mod strategy;
pub mod strategy_ir;

pub use authoring::{AuthoringContract, DraftStore, StrategyValidation, ValidationIssue};
pub use strategy_ir::{
    IndicatorRef, StrategyCapabilities, StrategyIr, UniverseSpec, STRATEGY_IR_SCHEMA_VERSION,
};

/// Contract-first strategy authoring and unsigned draft persistence.
pub mod authoring;

/// Parse the intentionally small, deterministic authoring DSL floor into the
/// canonical strategy document used by every authoring surface.
///
/// The current floor accepts `strategy Name { sma(close, period) > 0 }`. It is
/// deliberately fail-closed until the full grammar and type checker land.
pub fn parse_typecheck_to_ir_stub(source: &str) -> Result<StrategyIr, StrategyIRError> {
    let source = source.trim();
    let remainder = source.strip_prefix("strategy ").ok_or_else(|| {
        StrategyIRError::InvalidPayload("expected `strategy <name> { ... }`".into())
    })?;
    let (name, body) = remainder
        .split_once('{')
        .ok_or_else(|| StrategyIRError::InvalidPayload("strategy body is missing".into()))?;
    let body = body
        .strip_suffix('}')
        .ok_or_else(|| StrategyIRError::InvalidPayload("strategy body is not closed".into()))?
        .trim();
    let sma = body
        .strip_prefix("sma(close,")
        .and_then(|value| value.split_once(')'))
        .ok_or_else(|| {
            StrategyIRError::InvalidPayload("only `sma(close, period)` is supported".into())
        })?;
    if sma.1.trim() != "> 0" {
        return Err(StrategyIRError::InvalidPayload(
            "only the `> 0` rule floor is supported".into(),
        ));
    }
    let period: u32 = sma
        .0
        .trim()
        .parse()
        .map_err(|_| StrategyIRError::InvalidPayload("SMA period must be an integer".into()))?;
    if period == 0 {
        return Err(StrategyIRError::InvalidPayload(
            "SMA period must be positive".into(),
        ));
    }

    Ok(StrategyIr {
        schema_version: STRATEGY_IR_SCHEMA_VERSION.into(),
        strategy_id: format!("dsl:{}", name.trim().to_ascii_lowercase()),
        name: name.trim().into(),
        description: None,
        capabilities: StrategyCapabilities::default(),
        universe: UniverseSpec::Static {
            static_members: Vec::new(),
        },
        indicators: vec![IndicatorRef {
            kind: "sma".into(),
            alias: format!("sma_{period}"),
            warmup: period - 1,
        }],
        rules: serde_json::json!({ "entries": [], "exits": [] }),
    })
}
