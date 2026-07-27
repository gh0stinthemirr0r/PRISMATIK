//! `BrokerGateway` trait and three-outcome `SubmissionResult` (`P7-QM-01`).

use crate::broker_state::{LocalOrderState, OrderStateError};
use crate::error::BrokerError;
use crate::idempotency::IdempotencyKey;
use crate::order::ApprovedOrder;
use crate::reconcile::ReconciliationDelta;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Broker-assigned order identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BrokerOrderId(pub String);

impl BrokerOrderId {
    /// Construct from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the inner id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BrokerOrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Venue rejection detail for [`SubmissionResult::Rejected`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokerRejection {
    /// Machine-readable reason code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

impl BrokerRejection {
    /// Construct a rejection.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Three outcomes of submission — the `Unknown` path MUST NOT blind-retry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubmissionResult {
    /// Broker acknowledged the order.
    Accepted {
        /// Venue order id.
        broker_order_id: BrokerOrderId,
        /// Acceptance timestamp (UTC).
        accepted_at: OffsetDateTime,
    },
    /// Broker explicitly rejected the order.
    Rejected {
        /// Rejection detail.
        reason: BrokerRejection,
    },
    /// Request may or may not have reached the broker.
    ///
    /// Quarantine the instrument, call [`BrokerGateway::reconcile`], compare
    /// delta, append resolution to the audit ledger, then release quarantine
    /// only on success. Never retry blindly — that creates duplicates.
    Unknown {
        /// Key used for the ambiguous submission.
        idempotency_key: IdempotencyKey,
        /// Local belief at the time of ambiguity.
        last_known_state: LocalOrderState,
    },
}

/// Result of a cancel request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CancelResult {
    /// Cancel accepted / order no longer working.
    Cancelled {
        /// Broker order id.
        broker_order_id: BrokerOrderId,
    },
    /// Order was already terminal.
    AlreadyTerminal {
        /// Broker order id.
        broker_order_id: BrokerOrderId,
    },
}

/// Result of a replace request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplaceResult {
    /// Replace accepted; may carry a new broker id.
    Replaced {
        /// Possibly new broker order id.
        broker_order_id: BrokerOrderId,
    },
    /// Broker does not support replace.
    Unsupported,
}

/// Quantity / price modification for replace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderModification {
    /// New quantity (if changing).
    pub quantity: Option<i64>,
    /// New limit price in micros (if changing).
    pub limit_price_micros: Option<i64>,
}

/// Execution gateway abstraction. Broker adapters implement this.
///
/// Sync floor — async transport wrappers land with live adapters.
pub trait BrokerGateway: Send + Sync {
    /// Stable broker adapter id (e.g. `"paper"`, `"alpaca"`).
    fn broker_id(&self) -> &str;

    /// Submit a risk-approved order. Retries must reuse the same idempotency key.
    fn submit(&mut self, order: &ApprovedOrder) -> Result<SubmissionResult, BrokerError>;

    /// Cancel a working order (idempotent).
    fn cancel(&mut self, order_id: &BrokerOrderId) -> Result<CancelResult, BrokerError>;

    /// Replace a working order when supported.
    fn replace(
        &mut self,
        order_id: &BrokerOrderId,
        modification: &OrderModification,
    ) -> Result<ReplaceResult, BrokerError>;

    /// Reconcile local belief with broker truth after `Unknown` / disconnects.
    fn reconcile(&self, local_state: &LocalOrderState) -> Result<ReconciliationDelta, BrokerError>;

    /// Guard: refuse new intents for quarantined instruments.
    fn assert_not_quarantined(local: &LocalOrderState) -> Result<(), OrderStateError> {
        if local.quarantined {
            Err(OrderStateError::Quarantined)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idempotency::IdempotencyKey;
    use time::OffsetDateTime;

    #[test]
    fn submission_result_unknown_carries_key() {
        let key = IdempotencyKey::generate(b"u");
        let state = LocalOrderState::pending(key, "AAPL", 10);
        let r = SubmissionResult::Unknown {
            idempotency_key: key,
            last_known_state: state.clone(),
        };
        match r {
            SubmissionResult::Unknown {
                idempotency_key,
                last_known_state,
            } => {
                assert_eq!(idempotency_key, key);
                assert_eq!(last_known_state.instrument_id, "AAPL");
            },
            _ => panic!("expected Unknown"),
        }
        let _ = SubmissionResult::Accepted {
            broker_order_id: BrokerOrderId::new("1"),
            accepted_at: OffsetDateTime::UNIX_EPOCH,
        };
    }
}
