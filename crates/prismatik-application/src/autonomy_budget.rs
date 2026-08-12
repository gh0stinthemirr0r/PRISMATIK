//! Monetary autonomy budgets that fail closed at the trading boundary.

use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use thiserror::Error;

/// Independent spend envelopes in integer micro-units of the configured currency.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutonomyBudgetPolicy {
    /// ISO-4217 currency code.
    pub currency: String,
    /// Maximum model, data, and research spend in one UTC operating period.
    pub operations_limit_micros: u64,
    /// Maximum gross capital reservation for autonomous orders in one period.
    pub trading_limit_micros: u64,
    /// Maximum one-operation charge.
    pub per_operation_limit_micros: u64,
    /// Maximum one-order capital reservation.
    pub per_trade_limit_micros: u64,
}

/// Budget lane being charged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetLane {
    /// Research, model inference, ingestion, and analysis.
    Operations,
    /// Capital exposed by an autonomous order.
    Trading,
}

/// Current autonomous operating posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyPosture {
    /// Both analysis and explicitly enabled trading may continue.
    Active,
    /// Analysis may continue, but no new autonomous trade can be promoted.
    ResearchOnly,
    /// Operating spend is exhausted; local no-cost safety work may continue.
    LocalOnly,
}

/// Immutable budget snapshot for audit and UI display.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutonomyBudgetSnapshot {
    /// Active policy.
    pub policy: AutonomyBudgetPolicy,
    /// Charged or reserved operating spend.
    pub operations_used_micros: u64,
    /// Charged or reserved trading capital.
    pub trading_used_micros: u64,
    /// Derived fail-closed posture.
    pub posture: AutonomyPosture,
}

/// Budget rejection.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum BudgetError {
    /// Currency or a required limit is invalid.
    #[error("budget policy requires a currency and non-zero per-action limits")]
    InvalidPolicy,
    /// One request exceeded its lane's per-action cap.
    #[error("{lane:?} request {requested_micros} exceeds per-action cap {limit_micros}")]
    PerActionLimit {
        /// Rejected lane.
        lane: BudgetLane,
        /// Requested reservation.
        requested_micros: u64,
        /// Configured per-action ceiling.
        limit_micros: u64,
    },
    /// The lane has insufficient remaining allowance.
    #[error(
        "{lane:?} budget exhausted: requested {requested_micros}, remaining {remaining_micros}"
    )]
    Exhausted {
        /// Exhausted lane.
        lane: BudgetLane,
        /// Requested reservation.
        requested_micros: u64,
        /// Remaining allowance.
        remaining_micros: u64,
    },
    /// Shared state could not be read or updated.
    #[error("autonomy budget state is unavailable")]
    StateUnavailable,
}

/// Thread-safe budget ledger. Values are integer micro-units to avoid floating-point drift.
#[derive(Debug)]
pub struct AutonomyBudget {
    policy: AutonomyBudgetPolicy,
    usage: RwLock<(u64, u64)>,
}

impl AutonomyBudget {
    /// Create a budget ledger after validating its policy.
    pub fn new(policy: AutonomyBudgetPolicy) -> Result<Self, BudgetError> {
        if policy.currency.trim().is_empty()
            || policy.per_operation_limit_micros == 0
            || policy.per_trade_limit_micros == 0
        {
            return Err(BudgetError::InvalidPolicy);
        }
        Ok(Self {
            policy,
            usage: RwLock::new((0, 0)),
        })
    }

    /// Restore an exact previously persisted snapshot after validating its policy.
    ///
    /// Usage may exceed a newly lowered lane limit; that is a valid exhausted
    /// posture and must remain visible rather than being silently clamped.
    pub fn from_snapshot(snapshot: AutonomyBudgetSnapshot) -> Result<Self, BudgetError> {
        let budget = Self::new(snapshot.policy)?;
        *budget
            .usage
            .write()
            .map_err(|_| BudgetError::StateUnavailable)? = (
            snapshot.operations_used_micros,
            snapshot.trading_used_micros,
        );
        Ok(budget)
    }

    /// Atomically reserve an estimated charge before external work or order promotion.
    pub fn reserve(
        &self,
        lane: BudgetLane,
        amount_micros: u64,
    ) -> Result<AutonomyBudgetSnapshot, BudgetError> {
        let per_action = match lane {
            BudgetLane::Operations => self.policy.per_operation_limit_micros,
            BudgetLane::Trading => self.policy.per_trade_limit_micros,
        };
        if amount_micros > per_action {
            return Err(BudgetError::PerActionLimit {
                lane,
                requested_micros: amount_micros,
                limit_micros: per_action,
            });
        }
        let mut usage = self
            .usage
            .write()
            .map_err(|_| BudgetError::StateUnavailable)?;
        let (used, limit) = match lane {
            BudgetLane::Operations => (&mut usage.0, self.policy.operations_limit_micros),
            BudgetLane::Trading => (&mut usage.1, self.policy.trading_limit_micros),
        };
        let remaining = limit.saturating_sub(*used);
        if amount_micros > remaining {
            return Err(BudgetError::Exhausted {
                lane,
                requested_micros: amount_micros,
                remaining_micros: remaining,
            });
        }
        *used = used.saturating_add(amount_micros);
        Ok(Self::snapshot_from(&self.policy, *usage))
    }

    /// Read the current state. Policy changes and period resets remain explicit gated actions.
    pub fn snapshot(&self) -> Result<AutonomyBudgetSnapshot, BudgetError> {
        let usage = *self
            .usage
            .read()
            .map_err(|_| BudgetError::StateUnavailable)?;
        Ok(Self::snapshot_from(&self.policy, usage))
    }

    /// Release a prior reservation after a definitive pre-submission or broker rejection.
    /// Unknown broker outcomes must remain reserved until reconciliation resolves them.
    pub fn release(
        &self,
        lane: BudgetLane,
        amount_micros: u64,
    ) -> Result<AutonomyBudgetSnapshot, BudgetError> {
        let mut usage = self
            .usage
            .write()
            .map_err(|_| BudgetError::StateUnavailable)?;
        match lane {
            BudgetLane::Operations => usage.0 = usage.0.saturating_sub(amount_micros),
            BudgetLane::Trading => usage.1 = usage.1.saturating_sub(amount_micros),
        }
        Ok(Self::snapshot_from(&self.policy, *usage))
    }

    fn snapshot_from(policy: &AutonomyBudgetPolicy, usage: (u64, u64)) -> AutonomyBudgetSnapshot {
        let posture = if usage.0 >= policy.operations_limit_micros {
            AutonomyPosture::LocalOnly
        } else if usage.1 >= policy.trading_limit_micros {
            AutonomyPosture::ResearchOnly
        } else {
            AutonomyPosture::Active
        };
        AutonomyBudgetSnapshot {
            policy: policy.clone(),
            operations_used_micros: usage.0,
            trading_used_micros: usage.1,
            posture,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ledger() -> AutonomyBudget {
        AutonomyBudget::new(AutonomyBudgetPolicy {
            currency: "USD".into(),
            operations_limit_micros: 100,
            trading_limit_micros: 500,
            per_operation_limit_micros: 50,
            per_trade_limit_micros: 500,
        })
        .unwrap()
    }

    #[test]
    fn exhausted_trading_moves_to_research_only_without_stopping_operations() {
        let budget = ledger();
        let snapshot = budget.reserve(BudgetLane::Trading, 500).unwrap();
        assert_eq!(snapshot.posture, AutonomyPosture::ResearchOnly);
        assert!(budget.reserve(BudgetLane::Operations, 25).is_ok());
        assert!(matches!(
            budget.reserve(BudgetLane::Trading, 1),
            Err(BudgetError::Exhausted { .. })
        ));
    }

    #[test]
    fn per_action_caps_fail_before_mutating_usage() {
        let budget = ledger();
        assert!(matches!(
            budget.reserve(BudgetLane::Operations, 51),
            Err(BudgetError::PerActionLimit { .. })
        ));
        assert_eq!(budget.snapshot().unwrap().operations_used_micros, 0);
    }

    #[test]
    fn exact_snapshot_survives_restore_without_resetting_usage() {
        let budget = ledger();
        let expected = budget.reserve(BudgetLane::Operations, 25).unwrap();
        let restored = AutonomyBudget::from_snapshot(expected.clone()).unwrap();
        assert_eq!(restored.snapshot().unwrap(), expected);
    }
}
