//! Contract → validate → unsigned-draft lifecycle for agent and human authoring.

use crate::{StrategyIr, STRATEGY_IR_SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Capability and schema contract presented before authoring begins.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoringContract {
    /// Supported StrategyIR schema.
    pub schema_version: String,
    /// Native indicator kinds currently available.
    pub indicator_kinds: Vec<String>,
    /// Maximum declared compute budget per bar.
    pub max_compute_per_bar_ms: u32,
    /// Network capability is prohibited for authored strategies.
    pub network_allowed: bool,
    /// Filesystem capability is prohibited for authored strategies.
    pub filesystem_allowed: bool,
    /// AI invocation is prohibited inside deterministic StrategyIR.
    pub ai_allowed: bool,
}

impl AuthoringContract {
    /// Return the current fail-closed native contract.
    pub fn native() -> Self {
        Self {
            schema_version: STRATEGY_IR_SCHEMA_VERSION.into(),
            indicator_kinds: prismatik_indicator_core::NativeIndicatorKind::ALL
                .iter()
                .map(|kind| kind.as_str().to_owned())
                .collect(),
            max_compute_per_bar_ms: 1_000,
            network_allowed: false,
            filesystem_allowed: false,
            ai_allowed: false,
        }
    }
}

/// Explainable validation issue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Machine-readable code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Complete deterministic validation result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyValidation {
    /// True only when no issue remains.
    pub valid: bool,
    /// Deterministically ordered issues.
    pub issues: Vec<ValidationIssue>,
}

impl AuthoringContract {
    /// Validate schema, capability envelope, budgets, aliases, and indicators.
    pub fn validate(&self, strategy: &StrategyIr) -> StrategyValidation {
        let mut issues = Vec::new();
        let mut issue = |code: &str, message: &str| {
            issues.push(ValidationIssue {
                code: code.into(),
                message: message.into(),
            })
        };
        if strategy.schema_version != self.schema_version {
            issue("schema", "unsupported StrategyIR schema");
        }
        if strategy.capabilities.can_access_network && !self.network_allowed {
            issue("network", "network capability is denied");
        }
        if strategy.capabilities.can_access_filesystem && !self.filesystem_allowed {
            issue("filesystem", "filesystem capability is denied");
        }
        if strategy.capabilities.can_invoke_ai && !self.ai_allowed {
            issue(
                "ai",
                "AI invocation is denied inside deterministic StrategyIR",
            );
        }
        if strategy.capabilities.max_compute_per_bar_ms > self.max_compute_per_bar_ms {
            issue("compute_budget", "compute budget exceeds contract");
        }
        let allowed = self
            .indicator_kinds
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut aliases = BTreeSet::new();
        for indicator in &strategy.indicators {
            if !allowed.contains(&indicator.kind.to_ascii_lowercase()) {
                issue("indicator", "unknown native indicator");
            }
            if !aliases.insert(indicator.alias.clone()) {
                issue("alias", "indicator aliases must be unique");
            }
        }
        StrategyValidation {
            valid: issues.is_empty(),
            issues,
        }
    }
}

/// Local unsigned-draft store. Promotion/signing belongs to a separate gate.
#[derive(Clone, Debug, Default)]
pub struct DraftStore {
    drafts: BTreeMap<String, StrategyIr>,
}

impl DraftStore {
    /// Validate and save or replace an unsigned draft.
    pub fn save(
        &mut self,
        contract: &AuthoringContract,
        strategy: StrategyIr,
    ) -> Result<(), StrategyValidation> {
        let validation = contract.validate(&strategy);
        if !validation.valid {
            return Err(validation);
        }
        self.drafts.insert(strategy.strategy_id.clone(), strategy);
        Ok(())
    }
    /// Retrieve an unsigned draft.
    pub fn get(&self, strategy_id: &str) -> Option<&StrategyIr> {
        self.drafts.get(strategy_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_before_saving_unsigned_draft() {
        let contract = AuthoringContract::native();
        let mut store = DraftStore::default();
        let strategy = StrategyIr::minimal("s", "safe");
        store.save(&contract, strategy).unwrap();
        assert!(store.get("s").is_some());
    }
    #[test]
    fn rejects_network_capability() {
        let contract = AuthoringContract::native();
        let mut strategy = StrategyIr::minimal("s", "unsafe");
        strategy.capabilities.can_access_network = true;
        assert!(!contract.validate(&strategy).valid);
    }
    #[test]
    fn rejects_ai_capability() {
        let contract = AuthoringContract::native();
        let mut strategy = StrategyIr::minimal("s", "unsafe");
        strategy.capabilities.can_invoke_ai = true;
        assert!(!contract.validate(&strategy).valid);
    }
}
