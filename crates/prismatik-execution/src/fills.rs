//! Fill-stream buffer: partial fills and out-of-order events (`P7-QM-06`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// A single fill or partial-fill event from the broker stream.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FillEvent {
    /// Monotonic broker sequence (may arrive out of order).
    pub sequence: u64,
    /// Broker order id.
    pub broker_order_id: String,
    /// Incremental filled quantity for this event (not cumulative).
    pub fill_quantity: i64,
    /// Fill price in micros.
    pub price_micros: i64,
    /// True when the order is fully filled after this event.
    pub is_terminal: bool,
}

/// Fill-stream processing errors.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum FillStreamError {
    /// Duplicate sequence for the same stream.
    #[error("duplicate fill sequence {0}")]
    DuplicateSequence(u64),
    /// Fill would exceed the order quantity.
    #[error("fill exceeds remaining quantity")]
    Overfill,
}

/// Buffers out-of-order fill events and applies them in sequence order.
#[derive(Clone, Debug)]
pub struct FillStreamBuffer {
    /// Expected next sequence (global stream watermark).
    next_sequence: u64,
    /// Held events keyed by sequence until the gap closes.
    pending: BTreeMap<u64, FillEvent>,
    /// Cumulative filled quantity per broker order id.
    filled_by_order: BTreeMap<String, i64>,
    /// Declared order quantities (for overfill checks).
    order_qty: BTreeMap<String, i64>,
}

impl Default for FillStreamBuffer {
    fn default() -> Self {
        Self::new(0)
    }
}

impl FillStreamBuffer {
    /// Start with an expected next sequence (usually 0).
    pub fn new(next_sequence: u64) -> Self {
        Self {
            next_sequence,
            pending: BTreeMap::new(),
            filled_by_order: BTreeMap::new(),
            order_qty: BTreeMap::new(),
        }
    }

    /// Register an order's total quantity before fills arrive.
    pub fn register_order(&mut self, broker_order_id: impl Into<String>, quantity: i64) {
        let id = broker_order_id.into();
        self.order_qty.insert(id.clone(), quantity.abs());
        self.filled_by_order.entry(id).or_insert(0);
    }

    /// Ingest one event; returns the contiguous applied prefix (may be empty
    /// if this event opens a gap).
    pub fn ingest(&mut self, event: FillEvent) -> Result<Vec<FillEvent>, FillStreamError> {
        if event.sequence < self.next_sequence {
            return Err(FillStreamError::DuplicateSequence(event.sequence));
        }
        if self.pending.contains_key(&event.sequence) {
            return Err(FillStreamError::DuplicateSequence(event.sequence));
        }
        self.pending.insert(event.sequence, event);
        self.drain_ready()
    }

    fn drain_ready(&mut self) -> Result<Vec<FillEvent>, FillStreamError> {
        let mut applied = Vec::new();
        while let Some(ev) = self.pending.remove(&self.next_sequence) {
            let filled = self
                .filled_by_order
                .entry(ev.broker_order_id.clone())
                .or_insert(0);
            let order_qty = self
                .order_qty
                .get(&ev.broker_order_id)
                .copied()
                .unwrap_or(i64::MAX);
            if *filled + ev.fill_quantity.abs() > order_qty {
                self.pending.insert(self.next_sequence, ev);
                return Err(FillStreamError::Overfill);
            }
            *filled += ev.fill_quantity.abs();
            applied.push(ev);
            self.next_sequence += 1;
        }
        Ok(applied)
    }

    /// Cumulative filled quantity for an order.
    pub fn filled_quantity(&self, broker_order_id: &str) -> i64 {
        self.filled_by_order
            .get(broker_order_id)
            .copied()
            .unwrap_or(0)
    }

    /// Next expected sequence.
    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// Number of events held waiting for a gap to close.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill(seq: u64, qty: i64, terminal: bool) -> FillEvent {
        FillEvent {
            sequence: seq,
            broker_order_id: "o1".into(),
            fill_quantity: qty,
            price_micros: 1_000_000,
            is_terminal: terminal,
        }
    }

    #[test]
    fn out_of_order_partial_fills_apply_in_sequence() {
        let mut buf = FillStreamBuffer::new(0);
        buf.register_order("o1", 10);

        assert!(buf.ingest(fill(1, 3, false)).unwrap().is_empty());
        assert_eq!(buf.pending_count(), 1);

        let applied = buf.ingest(fill(0, 4, false)).unwrap();
        assert_eq!(applied.len(), 2);
        assert_eq!(applied[0].sequence, 0);
        assert_eq!(applied[1].sequence, 1);
        assert_eq!(buf.filled_quantity("o1"), 7);

        let applied = buf.ingest(fill(2, 3, true)).unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(buf.filled_quantity("o1"), 10);
        assert_eq!(buf.next_sequence(), 3);
    }

    #[test]
    fn duplicate_sequence_errors() {
        let mut buf = FillStreamBuffer::new(0);
        buf.register_order("o1", 5);
        buf.ingest(fill(0, 1, false)).unwrap();
        assert_eq!(
            buf.ingest(fill(0, 1, false)).unwrap_err(),
            FillStreamError::DuplicateSequence(0)
        );
    }
}
