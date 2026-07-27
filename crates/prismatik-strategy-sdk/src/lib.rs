//! # prismatik-strategy-sdk
//!
//! Rust builder API for hand-written strategies (`P4-EX-03`).
//!
//! Hand-written strategies construct [`StrategyIr`] via [`StrategyIrBuilder`].
//! Visual builder codegen, the Strategy DSL, and the Python research emitter
//! produce the **same** IR document (`schema_version` [`STRATEGY_IR_SCHEMA_VERSION`]);
//! this crate is the Rust authoring path into that shared IR.
//!
//! Spec: `DOCS/spec/STRATEGY_IR.md` §9.3

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use prismatik_strategy::{
    IndicatorRef, StrategyCapabilities, StrategyIr, UniverseSpec, STRATEGY_IR_SCHEMA_VERSION,
};

/// Fluent builder that produces a deny-by-default [`StrategyIr`].
///
/// Capabilities start from [`StrategyCapabilities::default`] (network, filesystem,
/// and AI all denied). Callers must opt in explicitly if a later wave permits it.
#[derive(Clone, Debug)]
pub struct StrategyIrBuilder {
    strategy_id: String,
    name: String,
    description: Option<String>,
    capabilities: StrategyCapabilities,
    universe: UniverseSpec,
    indicators: Vec<IndicatorRef>,
    rules: serde_json::Value,
}

impl StrategyIrBuilder {
    /// Start a builder with required identity fields.
    ///
    /// Capabilities default to deny-by-default; universe defaults to an empty
    /// static membership list.
    pub fn new(strategy_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            strategy_id: strategy_id.into(),
            name: name.into(),
            description: None,
            capabilities: StrategyCapabilities::default(),
            universe: UniverseSpec::Static {
                static_members: Vec::new(),
            },
            indicators: Vec::new(),
            rules: serde_json::json!({ "entries": [], "exits": [] }),
        }
    }

    /// Optional human description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set a static universe (fixed asset id list).
    pub fn universe_static<I, S>(mut self, members: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.universe = UniverseSpec::Static {
            static_members: members.into_iter().map(Into::into).collect(),
        };
        self
    }

    /// Append an indicator reference (`kind`, `alias`, `warmup`).
    pub fn indicator(
        mut self,
        kind: impl Into<String>,
        alias: impl Into<String>,
        warmup: u32,
    ) -> Self {
        self.indicators.push(IndicatorRef {
            kind: kind.into(),
            alias: alias.into(),
            warmup,
        });
        self
    }

    /// Soft compute budget per bar (milliseconds). Other capabilities remain deny-by-default.
    pub fn max_compute_per_bar_ms(mut self, ms: u32) -> Self {
        self.capabilities.max_compute_per_bar_ms = ms;
        self
    }

    /// Replace the opaque rules tree (defaults to empty entries/exits).
    pub fn rules(mut self, rules: serde_json::Value) -> Self {
        self.rules = rules;
        self
    }

    /// Build a [`StrategyIr`] with `schema_version` 1.0.0.
    pub fn build(self) -> StrategyIr {
        StrategyIr {
            schema_version: STRATEGY_IR_SCHEMA_VERSION.into(),
            strategy_id: self.strategy_id,
            name: self.name,
            description: self.description,
            capabilities: self.capabilities,
            universe: self.universe,
            indicators: self.indicators,
            rules: self.rules,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_matches_schema_version_and_round_trips() {
        let ir =
            StrategyIrBuilder::new("06d2e7e0-0000-4000-8000-000000000099", "SDK momentum floor")
                .description("Hand-written via strategy-sdk")
                .universe_static(["BTC", "ETH"])
                .indicator("sma", "sma_50", 50)
                .indicator("sma", "sma_200", 200)
                .max_compute_per_bar_ms(50)
                .build();

        assert_eq!(ir.schema_version, "1.0.0");
        assert_eq!(ir.schema_version, STRATEGY_IR_SCHEMA_VERSION);
        assert!(ir.schema_supported());
        assert!(!ir.capabilities.can_access_network);
        assert!(!ir.capabilities.can_access_filesystem);
        assert!(!ir.capabilities.can_invoke_ai);
        assert_eq!(
            ir.universe,
            UniverseSpec::Static {
                static_members: vec!["BTC".into(), "ETH".into()],
            }
        );
        assert_eq!(ir.indicators.len(), 2);

        let json = serde_json::to_string_pretty(&ir).expect("serialize");
        let back: StrategyIr = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ir, back);
        assert_eq!(back.schema_version, "1.0.0");
    }

    #[test]
    fn capabilities_are_deny_by_default() {
        let ir = StrategyIrBuilder::new("id", "name").build();
        assert_eq!(ir.capabilities, StrategyCapabilities::default());
        assert!(!ir.capabilities.can_access_network);
        assert!(!ir.capabilities.can_access_filesystem);
        assert!(!ir.capabilities.can_invoke_ai);
    }
}
