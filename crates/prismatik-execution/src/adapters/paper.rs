//! Deterministic paper broker with fill / latency / reject hooks (`P6-QM-04` floor).

use crate::broker_state::{BrokerOrderState, LocalOrderState};
use crate::error::BrokerError;
use crate::gateway::{
    BrokerGateway, BrokerOrderId, BrokerRejection, CancelResult, OrderModification, ReplaceResult,
    SubmissionResult,
};
use crate::idempotency::{IdempotencyError, IdempotencyStore};
use crate::order::ApprovedOrder;
use crate::reconcile::ReconciliationDelta;
use prismatik_determinism::DetMap;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

/// Deterministic hooks controlling paper-broker behavior.
///
/// All decisions are pure functions of the hook counters / flags — no wall-clock
/// randomness — so CI replays are bit-stable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperHooks {
    /// Simulated one-way latency in milliseconds (recorded; not slept).
    pub latency_ms: u64,
    /// When non-empty, the next N submits are rejected with this code.
    pub reject_next: u32,
    /// Rejection code used when `reject_next > 0`.
    pub reject_code: String,
    /// When non-empty, the next N submits return [`SubmissionResult::Unknown`].
    pub unknown_next: u32,
    /// Partial-fill fraction in basis points (0..=10_000). `10_000` = full fill.
    pub fill_bps: u32,
}

impl Default for PaperHooks {
    fn default() -> Self {
        Self {
            latency_ms: 0,
            reject_next: 0,
            reject_code: "paper_reject".into(),
            unknown_next: 0,
            fill_bps: 10_000,
        }
    }
}

impl PaperHooks {
    /// Full immediate fill, no rejects.
    pub fn full_fill() -> Self {
        Self::default()
    }

    /// Always reject the next submit.
    pub fn reject_once(code: impl Into<String>) -> Self {
        Self {
            reject_next: 1,
            reject_code: code.into(),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug)]
struct WorkingOrder {
    broker_order_id: BrokerOrderId,
    instrument_id: String,
    quantity: i64,
    filled_quantity: i64,
    status: String,
    accepted_at: OffsetDateTime,
}

/// Deterministic in-process paper broker.
#[derive(Clone, Debug)]
pub struct PaperBroker {
    hooks: PaperHooks,
    idem: IdempotencyStore,
    orders: DetMap<String, WorkingOrder>,
    /// Map idempotency hex → broker order id.
    by_key: DetMap<String, String>,
    next_id: u64,
    /// Deterministic clock base (UNIX epoch + offsets from latency).
    clock_offset_ms: i64,
}

impl Default for PaperBroker {
    fn default() -> Self {
        Self::new(PaperHooks::default())
    }
}

impl PaperBroker {
    /// Construct with hooks.
    pub fn new(hooks: PaperHooks) -> Self {
        Self {
            hooks,
            idem: IdempotencyStore::new(),
            orders: DetMap::default(),
            by_key: DetMap::default(),
            next_id: 1,
            clock_offset_ms: 0,
        }
    }

    /// Mutable access to hooks (tests / simulators).
    pub fn hooks_mut(&mut self) -> &mut PaperHooks {
        &mut self.hooks
    }

    /// Last recorded latency hook value.
    pub fn latency_ms(&self) -> u64 {
        self.hooks.latency_ms
    }

    fn now(&mut self) -> OffsetDateTime {
        self.clock_offset_ms += self.hooks.latency_ms as i64;
        OffsetDateTime::UNIX_EPOCH + Duration::milliseconds(self.clock_offset_ms)
    }

    fn payload_bytes(order: &ApprovedOrder) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(order.instrument_id.as_bytes());
        v.push(b'|');
        v.extend_from_slice(&order.quantity.to_le_bytes());
        v.push(b'|');
        v.extend_from_slice(order.idempotency_key.as_bytes());
        v
    }
}

impl BrokerGateway for PaperBroker {
    fn broker_id(&self) -> &str {
        "paper"
    }

    fn submit(&mut self, order: &ApprovedOrder) -> Result<SubmissionResult, BrokerError> {
        let payload = Self::payload_bytes(order);
        let settled = match self.idem.reserve(order.idempotency_key, &payload) {
            Err(IdempotencyError::Collision) => {
                return Err(BrokerError::Invariant("idempotency key collision".into()));
            },
            Err(IdempotencyError::AlreadySettled) => {
                return Err(BrokerError::Invariant("idempotency already settled".into()));
            },
            Ok(rec) => rec.settled_token.clone(),
        };

        if let Some(token) = settled {
            // Replay: return prior acceptance without creating a new order.
            if let Some(w) = self.orders.get(&token) {
                return Ok(SubmissionResult::Accepted {
                    broker_order_id: w.broker_order_id.clone(),
                    accepted_at: w.accepted_at,
                });
            }
        }

        // Replay of unsettled reservation with same key already mapped.
        if let Some(existing_id) = self.by_key.get(&order.idempotency_key.to_hex()) {
            if let Some(w) = self.orders.get(existing_id) {
                return Ok(SubmissionResult::Accepted {
                    broker_order_id: w.broker_order_id.clone(),
                    accepted_at: w.accepted_at,
                });
            }
        }

        if self.hooks.reject_next > 0 {
            self.hooks.reject_next -= 1;
            return Ok(SubmissionResult::Rejected {
                reason: BrokerRejection::new(
                    self.hooks.reject_code.clone(),
                    "paper broker reject hook",
                ),
            });
        }

        if self.hooks.unknown_next > 0 {
            self.hooks.unknown_next -= 1;
            let mut state = LocalOrderState::pending(
                order.idempotency_key,
                &order.instrument_id,
                order.quantity,
            );
            state.quarantine();
            return Ok(SubmissionResult::Unknown {
                idempotency_key: order.idempotency_key,
                last_known_state: state,
            });
        }

        let accepted_at = self.now();
        let id = format!("paper-{}", self.next_id);
        self.next_id += 1;

        let fill_bps = self.hooks.fill_bps.min(10_000);
        let filled = order.quantity.abs() * i64::from(fill_bps) / 10_000;
        let filled = if order.quantity < 0 { -filled } else { filled };
        let status = if filled.abs() >= order.quantity.abs() {
            "filled"
        } else if filled == 0 {
            "new"
        } else {
            "partially_filled"
        };

        let working = WorkingOrder {
            broker_order_id: BrokerOrderId::new(id.clone()),
            instrument_id: order.instrument_id.clone(),
            quantity: order.quantity,
            filled_quantity: filled,
            status: status.into(),
            accepted_at,
        };
        self.orders.insert(id.clone(), working);
        self.by_key
            .insert(order.idempotency_key.to_hex(), id.clone());
        let _ = self.idem.settle(order.idempotency_key, id.clone());

        Ok(SubmissionResult::Accepted {
            broker_order_id: BrokerOrderId::new(id),
            accepted_at,
        })
    }

    fn cancel(&mut self, order_id: &BrokerOrderId) -> Result<CancelResult, BrokerError> {
        match self.orders.get_mut(order_id.as_str()) {
            Some(w) if w.status == "filled" || w.status == "cancelled" => {
                Ok(CancelResult::AlreadyTerminal {
                    broker_order_id: order_id.clone(),
                })
            },
            Some(w) => {
                w.status = "cancelled".into();
                Ok(CancelResult::Cancelled {
                    broker_order_id: order_id.clone(),
                })
            },
            None => Err(BrokerError::Invariant(format!("unknown order {order_id}"))),
        }
    }

    fn replace(
        &mut self,
        order_id: &BrokerOrderId,
        modification: &OrderModification,
    ) -> Result<ReplaceResult, BrokerError> {
        let w = self
            .orders
            .get_mut(order_id.as_str())
            .ok_or_else(|| BrokerError::Invariant(format!("unknown order {order_id}")))?;
        if let Some(q) = modification.quantity {
            w.quantity = q;
        }
        Ok(ReplaceResult::Replaced {
            broker_order_id: order_id.clone(),
        })
    }

    fn reconcile(&self, local_state: &LocalOrderState) -> Result<ReconciliationDelta, BrokerError> {
        let broker = self
            .by_key
            .get(&local_state.idempotency_key.to_hex())
            .and_then(|id| self.orders.get(id))
            .map(|w| BrokerOrderState {
                broker_order_id: Some(w.broker_order_id.0.clone()),
                instrument_id: w.instrument_id.clone(),
                quantity: w.quantity,
                filled_quantity: w.filled_quantity,
                status: w.status.clone(),
            });

        Ok(crate::reconcile::Reconciler::new().compute_delta(
            local_state,
            broker.as_ref(),
            crate::reconcile::QuarantineReason::UnknownSubmission,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idempotency::IdempotencyKey;

    #[test]
    fn deterministic_fill_and_reject_hooks() {
        let mut hooks = PaperHooks::full_fill();
        hooks.latency_ms = 5;
        hooks.fill_bps = 5_000; // half fill
        let mut paper = PaperBroker::new(hooks);

        let key = IdempotencyKey::generate(b"paper-1");
        let order = ApprovedOrder::new("AAPL", 10, key);
        let res = paper.submit(&order).unwrap();
        match res {
            SubmissionResult::Accepted {
                broker_order_id, ..
            } => {
                let w = paper.orders.get(broker_order_id.as_str()).unwrap();
                assert_eq!(w.filled_quantity, 5);
                assert_eq!(w.status, "partially_filled");
            },
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(paper.latency_ms(), 5);

        paper.hooks_mut().reject_next = 1;
        let key2 = IdempotencyKey::generate(b"paper-2");
        let order2 = ApprovedOrder::new("AAPL", 1, key2);
        match paper.submit(&order2).unwrap() {
            SubmissionResult::Rejected { reason } => {
                assert_eq!(reason.code, "paper_reject");
            },
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn same_key_does_not_duplicate_order() {
        let mut paper = PaperBroker::default();
        let key = IdempotencyKey::generate(b"idem");
        let order = ApprovedOrder::new("QQQ", 2, key);
        let a = paper.submit(&order).unwrap();
        let b = paper.submit(&order).unwrap();
        match (a, b) {
            (
                SubmissionResult::Accepted {
                    broker_order_id: id_a,
                    ..
                },
                SubmissionResult::Accepted {
                    broker_order_id: id_b,
                    ..
                },
            ) => {
                assert_eq!(id_a, id_b);
                assert_eq!(paper.orders.len(), 1);
            },
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn unknown_hook_quarantines() {
        let hooks = PaperHooks {
            unknown_next: 1,
            ..Default::default()
        };
        let mut paper = PaperBroker::new(hooks);
        let key = IdempotencyKey::generate(b"unk");
        let order = ApprovedOrder::new("SPY", 3, key);
        match paper.submit(&order).unwrap() {
            SubmissionResult::Unknown {
                last_known_state, ..
            } => assert!(last_known_state.quarantined),
            other => panic!("unexpected {other:?}"),
        }
    }
}
