//! Reconciliation delta and quarantine reasons (`P7-QM-03` floor).

use crate::broker_state::{BrokerOrderState, LocalOrderState};
use crate::idempotency::IdempotencyKey;
use serde::{Deserialize, Serialize};

/// Why an instrument was quarantined.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuarantineReason {
    /// Submission returned [`crate::SubmissionResult::Unknown`].
    UnknownSubmission,
    /// Transport dropped mid-flight.
    DisconnectMidSubmit,
    /// Periodic safety-net found divergence.
    PeriodicDivergence,
}

/// Delta between local belief and broker truth.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationDelta {
    /// Key under reconciliation.
    pub idempotency_key: IdempotencyKey,
    /// Local snapshot compared.
    pub local: LocalOrderState,
    /// Broker snapshot (if found).
    pub broker: Option<BrokerOrderState>,
    /// Signed quantity delta (`broker_filled - local_filled`).
    pub filled_quantity_delta: i64,
    /// True when states agree within floor tolerances.
    pub resolved: bool,
    /// Human-readable resolution note for ledger append.
    pub resolution_note: String,
}

/// Floor reconciler: compares local vs broker and produces a delta.
#[derive(Clone, Debug, Default)]
pub struct Reconciler;

impl Reconciler {
    /// Construct.
    pub fn new() -> Self {
        Self
    }

    /// Compare local belief to an optional broker snapshot.
    pub fn compute_delta(
        &self,
        local: &LocalOrderState,
        broker: Option<&BrokerOrderState>,
        reason: QuarantineReason,
    ) -> ReconciliationDelta {
        let (filled_quantity_delta, resolved, resolution_note) = match broker {
            None => (
                0,
                false,
                format!("no broker order found ({reason:?}); remain quarantined"),
            ),
            Some(b) => {
                let delta = b.filled_quantity - local.filled_quantity;
                let qty_match = b.quantity == local.quantity;
                let terminal_match =
                    b.status == "filled" && b.filled_quantity == local.quantity && qty_match;
                let in_sync = qty_match && b.filled_quantity == local.filled_quantity;
                let resolved = in_sync || terminal_match;
                let note = if resolved {
                    format!("reconciled ok ({reason:?}); filled_delta={delta}")
                } else {
                    format!(
                        "divergence ({reason:?}): local_filled={} broker_filled={} status={}",
                        local.filled_quantity, b.filled_quantity, b.status
                    )
                };
                (delta, resolved, note)
            },
        };

        ReconciliationDelta {
            idempotency_key: local.idempotency_key,
            local: local.clone(),
            broker: broker.cloned(),
            filled_quantity_delta,
            resolved,
            resolution_note,
        }
    }

    /// Apply a successful delta: update filled qty and release quarantine.
    pub fn apply_resolution(local: &mut LocalOrderState, delta: &ReconciliationDelta) -> bool {
        if !delta.resolved {
            return false;
        }
        if let Some(b) = &delta.broker {
            local.filled_quantity = b.filled_quantity;
        }
        local.release_quarantine();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idempotency::IdempotencyKey;

    #[test]
    fn unknown_path_stays_quarantined_until_resolved() {
        let key = IdempotencyKey::generate(b"recon");
        let mut local = LocalOrderState::pending(key, "MSFT", 5);
        local.quarantine();
        let broker = BrokerOrderState {
            broker_order_id: Some("b-1".into()),
            instrument_id: "MSFT".into(),
            quantity: 5,
            filled_quantity: 5,
            status: "filled".into(),
        };
        let recon = Reconciler::new();
        let delta = recon.compute_delta(&local, Some(&broker), QuarantineReason::UnknownSubmission);
        assert!(delta.resolved);
        assert!(Reconciler::apply_resolution(&mut local, &delta));
        assert!(!local.quarantined);
        assert_eq!(local.filled_quantity, 5);
    }

    #[test]
    fn missing_broker_order_does_not_release() {
        let key = IdempotencyKey::generate(b"missing");
        let mut local = LocalOrderState::pending(key, "MSFT", 5);
        local.quarantine();
        let delta =
            Reconciler::new().compute_delta(&local, None, QuarantineReason::DisconnectMidSubmit);
        assert!(!delta.resolved);
        assert!(!Reconciler::apply_resolution(&mut local, &delta));
        assert!(local.quarantined);
    }
}
