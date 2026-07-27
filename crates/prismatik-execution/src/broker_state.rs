//! Local and broker-view order state (`P7-QM-01` / `P7-QM-03` support).

use crate::idempotency::IdempotencyKey;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Local belief about an in-flight or working order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalOrderState {
    /// Idempotency key that uniquely identifies the intent.
    pub idempotency_key: IdempotencyKey,
    /// Opaque instrument id.
    pub instrument_id: String,
    /// Signed quantity believed submitted.
    pub quantity: i64,
    /// Quantity believed filled locally.
    pub filled_quantity: i64,
    /// Whether the instrument is currently quarantined pending reconcile.
    pub quarantined: bool,
}

impl LocalOrderState {
    /// New local state for a just-submitted (or about-to-submit) intent.
    pub fn pending(
        idempotency_key: IdempotencyKey,
        instrument_id: impl Into<String>,
        quantity: i64,
    ) -> Self {
        Self {
            idempotency_key,
            instrument_id: instrument_id.into(),
            quantity,
            filled_quantity: 0,
            quarantined: false,
        }
    }

    /// Mark quarantined after an `Unknown` submission result.
    pub fn quarantine(&mut self) {
        self.quarantined = true;
    }

    /// Release quarantine after successful reconciliation.
    pub fn release_quarantine(&mut self) {
        self.quarantined = false;
    }
}

/// Broker-reported order state snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokerOrderState {
    /// Broker-assigned order id when known.
    pub broker_order_id: Option<String>,
    /// Opaque instrument id.
    pub instrument_id: String,
    /// Signed quantity at the venue.
    pub quantity: i64,
    /// Filled quantity at the venue.
    pub filled_quantity: i64,
    /// Venue status label (e.g. `"new"`, `"partially_filled"`, `"filled"`).
    pub status: String,
}

/// Order-state transition errors.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum OrderStateError {
    /// Instrument remains quarantined; no further intents admitted.
    #[error("instrument quarantined pending reconciliation")]
    Quarantined,
    /// Local and broker views disagree irreconcilably.
    #[error("irreconcilable order state: {0}")]
    Irreconcilable(String),
}
