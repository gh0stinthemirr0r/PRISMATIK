//! # prismatik-prismatik-execution
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — broker gateway contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use broker_state::{BrokerOrderState, LocalOrderState, OrderStateError};
pub use gateway::{BrokerError, BrokerErrorCode, BrokerGateway, BrokerRejection, SubmissionResult};
pub use idempotency::{IdempotencyError, IdempotencyKey};
pub use reconcile::{QuarantineReason, Reconciler, ReconciliationDelta};

/// Broker gateway contracts.
pub mod gateway {
    use crate::broker_state::LocalOrderState;
    use crate::idempotency::IdempotencyKey;
    use prismatik_risk::RiskApprovedOrderIntent;

    /// Broker error category.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum BrokerErrorCode {
        /// Permanent configuration error.
        Config,
        /// Permanent authentication error.
        Auth,
        /// Transient network error.
        Network,
        /// Transient exchange-side rejection.
        Exchange,
        /// Market closed status.
        MarketClosed,
        /// Temporary connecting state.
        Connecting,
        /// Unknown error category.
        Unknown,
    }

    impl BrokerErrorCode {
        /// Returns true if this class should disable account activity.
        pub fn permanent(self) -> bool {
            matches!(self, Self::Config | Self::Auth)
        }
    }

    /// Broker rejection details.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct BrokerRejection {
        /// Rejection code text.
        pub code: String,
        /// Rejection message.
        pub message: String,
    }

    /// Broker error details.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct BrokerError {
        /// Error code class.
        pub code: BrokerErrorCode,
        /// Human-readable details.
        pub message: String,
    }

    /// Submission result.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum SubmissionResult {
        /// Accepted by broker.
        Accepted {
            /// Broker order id.
            broker_order_id: String,
            /// Accepted-at timestamp text.
            accepted_at: String,
        },
        /// Rejected by broker.
        Rejected {
            /// Rejection details.
            reason: BrokerRejection,
        },
        /// Unknown final state requiring reconciliation.
        Unknown {
            /// Idempotency key.
            idempotency_key: IdempotencyKey,
            /// Last known local state.
            last_known_state: LocalOrderState,
        },
    }

    impl SubmissionResult {
        /// Return true when reconciliation is required before retry.
        pub fn requires_reconciliation(&self) -> bool {
            matches!(self, Self::Unknown { .. })
        }
    }

    /// Broker gateway abstraction.
    pub trait BrokerGateway: Send + Sync {
        /// Submit risk-approved order intent.
        fn submit(&self, intent: RiskApprovedOrderIntent) -> Result<SubmissionResult, BrokerError>;
    }
}

/// Idempotency contracts.
pub mod idempotency {
    /// Idempotency key wrapper.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct IdempotencyKey(pub String);

    impl IdempotencyKey {
        /// Parse a key from text with validation.
        pub fn parse(raw: impl Into<String>) -> Result<Self, IdempotencyError> {
            let value = raw.into();
            if value.trim().is_empty() {
                return Err(IdempotencyError {
                    message: "idempotency key cannot be empty".to_owned(),
                });
            }
            Ok(Self(value))
        }
    }

    /// Idempotency failures.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct IdempotencyError {
        /// Error message.
        pub message: String,
    }
}

/// Reconciliation contracts.
pub mod reconcile {
    use std::collections::BTreeMap;

    /// Quarantine reason for unresolved states.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum QuarantineReason {
        /// Broker state is unknown.
        UnknownState,
        /// Reconciliation detected position delta.
        DeltaMismatch,
    }

    /// Reconciliation delta summary.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ReconciliationDelta {
        /// Symbol or asset id.
        pub asset_id: String,
        /// Delta amount as decimal string.
        pub quantity_delta: String,
    }

    /// Reconciler contract.
    pub trait Reconciler: Send + Sync {
        /// Compare local and broker snapshots.
        fn reconcile(&self) -> Result<Vec<ReconciliationDelta>, QuarantineReason>;
    }

    /// Position snapshot entry.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct PositionSnapshot {
        /// Canonical asset id.
        pub asset_id: String,
        /// Quantity as decimal string.
        pub quantity: String,
    }

    /// Reconciliation plan summary.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ReconciliationPlan {
        /// Deltas requiring resolution.
        pub deltas: Vec<ReconciliationDelta>,
        /// Whether this plan requires quarantine.
        pub quarantine_required: bool,
    }

    /// Build deterministic reconciliation plan from local and broker snapshots.
    pub fn reconcile_positions(
        local: &[PositionSnapshot],
        broker: &[PositionSnapshot],
    ) -> Result<ReconciliationPlan, QuarantineReason> {
        let local_map = to_quantity_map(local)?;
        let broker_map = to_quantity_map(broker)?;

        let mut all_assets = local_map
            .keys()
            .chain(broker_map.keys())
            .cloned()
            .collect::<Vec<_>>();
        all_assets.sort();
        all_assets.dedup();

        let mut deltas = Vec::new();
        for asset_id in all_assets {
            let local_qty = local_map.get(&asset_id).copied().unwrap_or(0.0);
            let broker_qty = broker_map.get(&asset_id).copied().unwrap_or(0.0);
            let diff = local_qty - broker_qty;
            if diff.abs() > f64::EPSILON {
                deltas.push(ReconciliationDelta {
                    asset_id,
                    quantity_delta: format!("{diff:.10}"),
                });
            }
        }

        Ok(ReconciliationPlan {
            quarantine_required: !deltas.is_empty(),
            deltas,
        })
    }

    fn to_quantity_map(
        snapshot: &[PositionSnapshot],
    ) -> Result<BTreeMap<String, f64>, QuarantineReason> {
        let mut map = BTreeMap::new();
        for row in snapshot {
            let quantity = row
                .quantity
                .parse::<f64>()
                .map_err(|_| QuarantineReason::UnknownState)?;
            map.insert(row.asset_id.clone(), quantity);
        }
        Ok(map)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn reconciliation_is_deterministic_and_flags_deltas() {
            let local = vec![
                PositionSnapshot {
                    asset_id: "B".to_owned(),
                    quantity: "10".to_owned(),
                },
                PositionSnapshot {
                    asset_id: "A".to_owned(),
                    quantity: "5".to_owned(),
                },
            ];
            let broker = vec![
                PositionSnapshot {
                    asset_id: "A".to_owned(),
                    quantity: "5".to_owned(),
                },
                PositionSnapshot {
                    asset_id: "B".to_owned(),
                    quantity: "8".to_owned(),
                },
            ];
            let plan = reconcile_positions(&local, &broker).expect("reconcile");
            assert!(plan.quarantine_required);
            assert_eq!(plan.deltas.len(), 1);
            assert_eq!(plan.deltas[0].asset_id, "B");
        }

        #[test]
        fn invalid_quantity_is_unknown_state() {
            let local = vec![PositionSnapshot {
                asset_id: "A".to_owned(),
                quantity: "x".to_owned(),
            }];
            let broker = Vec::new();
            let error = reconcile_positions(&local, &broker).expect_err("invalid quantity");
            assert_eq!(error, QuarantineReason::UnknownState);
        }
    }
}

/// Broker/local state contracts.
pub mod broker_state {
    /// Local order state machine.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum LocalOrderState {
        /// Created locally.
        Created,
        /// Submitted to broker.
        Submitted,
        /// Filled.
        Filled,
        /// Rejected.
        Rejected,
        /// Unknown.
        Unknown,
    }

    /// Broker-reported state.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum BrokerOrderState {
        /// Pending.
        Pending,
        /// Filled.
        Filled,
        /// Cancelled.
        Cancelled,
        /// Rejected.
        Rejected,
        /// Unknown.
        Unknown,
    }

    /// Order state conversion error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct OrderStateError {
        /// Error message.
        pub message: String,
    }
}

/// Adapter namespace.
pub mod adapters {
    use crate::gateway::{BrokerError, SubmissionResult};
    use prismatik_risk::RiskApprovedOrderIntent;

    /// Broker adapter kind.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum AdapterKind {
        /// Alpaca adapter.
        Alpaca,
        /// CCXT adapter.
        Ccxt,
        /// Interactive Brokers adapter.
        Ibkr,
        /// Coinbase adapter.
        Coinbase,
    }

    /// Broker adapter abstraction.
    pub trait BrokerAdapter: Send + Sync {
        /// Adapter kind identifier.
        fn kind(&self) -> AdapterKind;
        /// Submit order intent through adapter.
        fn submit(&self, intent: RiskApprovedOrderIntent) -> Result<SubmissionResult, BrokerError>;
    }
}
