//! StrategyIR floor types (`P4-QM-01`).
//!
//! Stable IR schema types so strategy authoring modes and the runtime converge
//! on one document. Capabilities default-deny; universe spec and indicator
//! refs are typed here; rule trees are opaque JSON pending full compilation.

use serde::{Deserialize, Serialize};

/// StrategyIR schema version (semver string).
pub const STRATEGY_IR_SCHEMA_VERSION: &str = "1.0.0";

/// Capability declaration enforced by the runtime.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyCapabilities {
    /// May open network sockets.
    pub can_access_network: bool,
    /// May touch the host filesystem.
    pub can_access_filesystem: bool,
    /// May invoke AI tools.
    pub can_invoke_ai: bool,
    /// Soft compute budget per bar (milliseconds).
    pub max_compute_per_bar_ms: u32,
}

impl Default for StrategyCapabilities {
    fn default() -> Self {
        Self {
            can_access_network: false,
            can_access_filesystem: false,
            can_invoke_ai: false,
            max_compute_per_bar_ms: 50,
        }
    }
}

/// How the strategy universe is defined.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UniverseSpec {
    /// Fixed asset id list.
    Static {
        /// Asset ids (opaque strings at this floor; typed later).
        static_members: Vec<String>,
    },
    /// Placeholder for DataFusion plan (Wave 3A full).
    DynamicQuery {
        /// Opaque plan bytes (base64 in JSON).
        datafusion_plan: String,
        /// Rebalance cadence label.
        rebalance_frequency: String,
    },
}

/// Indicator reference inside inputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndicatorRef {
    /// Closed-set kind (e.g. `"sma"`).
    pub kind: String,
    /// Alias used in rules.
    pub alias: String,
    /// Warmup length declared by the indicator.
    pub warmup: u32,
}

/// Top-level StrategyIR document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyIr {
    /// Schema version.
    pub schema_version: String,
    /// Stable strategy id (UUID string).
    pub strategy_id: String,
    /// Human name.
    pub name: String,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Capabilities.
    pub capabilities: StrategyCapabilities,
    /// Universe.
    pub universe: UniverseSpec,
    /// Indicator inputs (feature views deferred).
    #[serde(default)]
    pub indicators: Vec<IndicatorRef>,
    /// Opaque rule tree JSON for later compilation.
    #[serde(default)]
    pub rules: serde_json::Value,
}

impl StrategyIr {
    /// Construct a minimal static-universe strategy.
    pub fn minimal(strategy_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            schema_version: STRATEGY_IR_SCHEMA_VERSION.into(),
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

    /// Schema version check.
    pub fn schema_supported(&self) -> bool {
        self.schema_version == STRATEGY_IR_SCHEMA_VERSION || self.schema_version.starts_with("1.0.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_ir_round_trips_and_defaults_deny_capabilities() {
        let ir = StrategyIr::minimal("06d2e7e0-0000-4000-8000-000000000001", "floor");
        assert!(ir.schema_supported());
        assert!(!ir.capabilities.can_access_network);
        let json = serde_json::to_string(&ir).unwrap();
        let back: StrategyIr = serde_json::from_str(&json).unwrap();
        assert_eq!(ir, back);
        assert_eq!(back.schema_version, STRATEGY_IR_SCHEMA_VERSION);
    }

    #[test]
    fn schema_version_constant_is_semver_1_0_0() {
        assert_eq!(STRATEGY_IR_SCHEMA_VERSION, "1.0.0");
        let ir = StrategyIr::minimal("id", "name");
        assert_eq!(ir.schema_version, "1.0.0");
        assert!(ir.schema_supported());
        let mut unsupported = ir.clone();
        unsupported.schema_version = "2.0.0".into();
        assert!(!unsupported.schema_supported());
    }

    #[test]
    fn dynamic_universe_round_trips() {
        let ir = StrategyIr {
            schema_version: STRATEGY_IR_SCHEMA_VERSION.into(),
            strategy_id: "dyn".into(),
            name: "dynamic".into(),
            description: Some("prep".into()),
            capabilities: StrategyCapabilities::default(),
            universe: UniverseSpec::DynamicQuery {
                datafusion_plan: "cGxhbg==".into(),
                rebalance_frequency: "1d".into(),
            },
            indicators: vec![IndicatorRef {
                kind: "sma".into(),
                alias: "sma_20".into(),
                warmup: 20,
            }],
            rules: serde_json::json!({ "entries": [], "exits": [] }),
        };
        let json = serde_json::to_string_pretty(&ir).unwrap();
        let back: StrategyIr = serde_json::from_str(&json).unwrap();
        assert_eq!(ir, back);
        assert_eq!(back.schema_version, STRATEGY_IR_SCHEMA_VERSION);
    }
}
