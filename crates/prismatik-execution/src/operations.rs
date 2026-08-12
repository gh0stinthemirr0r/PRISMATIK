//! Deterministic account health and continuous position-reconciliation decisions.

use crate::gateway::BrokerErrorCode;
use crate::reconcile::{
    reconcile_positions, PositionSnapshot, QuarantineReason, ReconciliationDelta,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Account connectivity and authorization state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountHealth {
    /// No connection has been requested.
    Disconnected,
    /// Normal connection or recovery is in progress and is not a failure.
    Connecting,
    /// Reads are healthy but risk-increasing operations are disabled.
    ReadOnly,
    /// Paper-only operations are healthy.
    PaperReady,
    /// Live account is healthy but still subject to separate session gates.
    LiveReady,
    /// Transient degradation.
    Degraded {
        /// Explainable reason.
        reason: String,
    },
    /// Permanent account disablement.
    Disabled {
        /// Explainable reason.
        reason: String,
    },
    /// State is ambiguous and must reconcile before further submissions.
    Quarantined {
        /// Explainable reason.
        reason: String,
    },
}

/// Supplied facts used to derive account health.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountHealthInput {
    /// Whether a connection attempt is in progress.
    pub connecting: bool,
    /// Last classified broker error.
    pub last_error: Option<BrokerErrorCode>,
    /// Whether credentials/entitlements passed a real validation.
    pub credentials_validated: bool,
    /// Deployment is environment-locked read-only.
    pub environment_read_only: bool,
    /// Paper mode selected.
    pub paper_mode: bool,
    /// Unknown submission or unresolved reconciliation exists.
    pub unresolved_ambiguity: bool,
}

/// Derive health fail-closed, classifying connecting before errors.
pub fn evaluate_account_health(input: &AccountHealthInput) -> AccountHealth {
    if input.connecting {
        return AccountHealth::Connecting;
    }
    if input.unresolved_ambiguity {
        return AccountHealth::Quarantined {
            reason: "unresolved broker ambiguity".into(),
        };
    }
    if let Some(error) = input.last_error {
        if error.permanent() {
            return AccountHealth::Disabled {
                reason: format!("permanent broker error: {error:?}"),
            };
        }
        if error != BrokerErrorCode::MarketClosed {
            return AccountHealth::Degraded {
                reason: format!("transient broker state: {error:?}"),
            };
        }
    }
    if !input.credentials_validated {
        return AccountHealth::Disconnected;
    }
    if input.environment_read_only {
        AccountHealth::ReadOnly
    } else if input.paper_mode {
        AccountHealth::PaperReady
    } else {
        AccountHealth::LiveReady
    }
}

/// Action selected after one reconciliation cycle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationAction {
    /// Snapshots agree.
    Continue,
    /// Benign caller-approved deltas may be journaled and applied.
    ApplyBenign,
    /// Ambiguous divergence must halt submissions.
    Quarantine,
}

/// Immutable result of one periodic reconciliation cycle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationCycle {
    /// Cycle observation time supplied by caller clock.
    pub observed_at: OffsetDateTime,
    /// Exact position deltas.
    pub deltas: Vec<ReconciliationDelta>,
    /// Deterministic action.
    pub action: ReconciliationAction,
}

/// Compare venue truth with local state for a continuous reconciliation loop.
pub fn reconcile_cycle(
    local: &[PositionSnapshot],
    broker: &[PositionSnapshot],
    benign_assets: &[String],
    observed_at: OffsetDateTime,
) -> Result<ReconciliationCycle, QuarantineReason> {
    let plan = reconcile_positions(local, broker)?;
    let action = if plan.deltas.is_empty() {
        ReconciliationAction::Continue
    } else if plan
        .deltas
        .iter()
        .all(|delta| benign_assets.contains(&delta.asset_id))
    {
        ReconciliationAction::ApplyBenign
    } else {
        ReconciliationAction::Quarantine
    };
    Ok(ReconciliationCycle {
        observed_at,
        deltas: plan.deltas,
        action,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connecting_is_not_a_failure() {
        assert_eq!(
            evaluate_account_health(&AccountHealthInput {
                connecting: true,
                last_error: Some(BrokerErrorCode::Auth),
                credentials_validated: false,
                environment_read_only: false,
                paper_mode: false,
                unresolved_ambiguity: false
            }),
            AccountHealth::Connecting
        );
    }
    #[test]
    fn unknown_delta_quarantines() {
        let local = [PositionSnapshot {
            asset_id: "BTC".into(),
            quantity: "1".into(),
        }];
        let broker = [PositionSnapshot {
            asset_id: "BTC".into(),
            quantity: "2".into(),
        }];
        let cycle = reconcile_cycle(&local, &broker, &[], OffsetDateTime::UNIX_EPOCH).unwrap();
        assert_eq!(cycle.action, ReconciliationAction::Quarantine);
    }
}
