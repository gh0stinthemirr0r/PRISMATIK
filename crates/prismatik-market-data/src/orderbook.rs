//! Deterministic fixed-point L2 order-book reconstruction and depth execution.

use prismatik_domain::ProviderId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use time::OffsetDateTime;

/// Order-book side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BookSide {
    /// Bid side.
    Bid,
    /// Ask side.
    Ask,
}

/// L2 update kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BookEventKind {
    /// Replace aggregate size.
    Upsert,
    /// Remove a level.
    Delete,
}

/// One normalized, sequenced L2 update.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookEvent {
    /// Provider provenance.
    pub provider: ProviderId,
    /// Provider-native instrument identifier.
    pub instrument_id: String,
    /// Monotonic venue sequence.
    pub sequence: u64,
    /// Side updated.
    pub side: BookSide,
    /// Fixed-point price micros.
    pub price_micros: u64,
    /// Aggregate size micros.
    pub size_micros: u64,
    /// Update operation.
    pub kind: BookEventKind,
    /// Venue event time.
    pub event_time: OffsetDateTime,
    /// Provider-boundary retrieval time.
    pub retrieved_at: OffsetDateTime,
}

/// One materialized L2 level.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookLevel {
    /// Price micros.
    pub price_micros: u64,
    /// Aggregate size micros.
    pub size_micros: u64,
}

/// Reconstruction anomaly that makes completeness explicit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BookAnomaly {
    /// Sequence number skipped updates.
    SequenceGap {
        /// Expected sequence.
        expected: u64,
        /// Observed sequence.
        observed: u64,
    },
    /// Update was not monotonic.
    NonMonotonicSequence {
        /// Prior sequence.
        prior: u64,
        /// Observed sequence.
        observed: u64,
    },
    /// Provider or instrument changed.
    MixedStream,
    /// Retrieval preceded event time.
    InvalidTimestamp,
    /// Materialized top of book crossed.
    CrossedBook,
}

/// Deterministically reconstructed L2 snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookSnapshot {
    /// Provider provenance.
    pub provider: ProviderId,
    /// Instrument identifier.
    pub instrument_id: String,
    /// Last applied sequence.
    pub sequence: u64,
    /// Bids descending by price.
    pub bids: Vec<BookLevel>,
    /// Asks ascending by price.
    pub asks: Vec<BookLevel>,
    /// Integrity anomalies.
    pub anomalies: Vec<BookAnomaly>,
}

/// Executable depth result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepthExecution {
    /// Requested size.
    pub requested_size_micros: u64,
    /// Size available in supplied depth.
    pub filled_size_micros: u64,
    /// Volume-weighted average price.
    pub average_price_micros: Option<u64>,
    /// Worst consumed price.
    pub worst_price_micros: Option<u64>,
    /// Impact from first level in parts per million.
    pub impact_ppm: u32,
}

impl BookSnapshot {
    /// Simulate a buy from asks or sell into bids without invented liquidity.
    pub fn execute_depth(&self, side: BookSide, requested_size_micros: u64) -> DepthExecution {
        let levels = match side {
            BookSide::Bid => &self.bids,
            BookSide::Ask => &self.asks,
        };
        let mut remaining = requested_size_micros;
        let mut filled = 0_u64;
        let mut notional = 0_u128;
        let mut worst = None;
        for level in levels {
            if remaining == 0 {
                break;
            }
            let take = remaining.min(level.size_micros);
            remaining -= take;
            filled += take;
            notional += u128::from(take) * u128::from(level.price_micros);
            worst = Some(level.price_micros);
        }
        let average =
            (filled > 0).then(|| u64::try_from(notional / u128::from(filled)).unwrap_or(u64::MAX));
        let impact_ppm = match (levels.first(), average) {
            (Some(first), Some(avg)) if first.price_micros > 0 => u32::try_from(
                u128::from(avg.abs_diff(first.price_micros)) * 1_000_000
                    / u128::from(first.price_micros),
            )
            .unwrap_or(u32::MAX),
            _ => 0,
        };
        DepthExecution {
            requested_size_micros,
            filled_size_micros: filled,
            average_price_micros: average,
            worst_price_micros: worst,
            impact_ppm,
        }
    }
}

/// Reconstruct one stream from deterministic event order.
pub fn reconstruct_book(events: &[BookEvent]) -> Option<BookSnapshot> {
    let first = events.first()?;
    let mut bids = BTreeMap::<u64, u64>::new();
    let mut asks = BTreeMap::<u64, u64>::new();
    let mut anomalies = Vec::new();
    let mut prior = None;
    for event in events {
        if event.provider != first.provider || event.instrument_id != first.instrument_id {
            anomalies.push(BookAnomaly::MixedStream);
            continue;
        }
        if event.retrieved_at < event.event_time {
            anomalies.push(BookAnomaly::InvalidTimestamp);
        }
        if let Some(previous) = prior {
            if event.sequence <= previous {
                anomalies.push(BookAnomaly::NonMonotonicSequence {
                    prior: previous,
                    observed: event.sequence,
                });
                continue;
            }
            if event.sequence != previous + 1 {
                anomalies.push(BookAnomaly::SequenceGap {
                    expected: previous + 1,
                    observed: event.sequence,
                });
            }
        }
        prior = Some(event.sequence);
        let levels = match event.side {
            BookSide::Bid => &mut bids,
            BookSide::Ask => &mut asks,
        };
        match event.kind {
            BookEventKind::Upsert if event.size_micros > 0 => {
                levels.insert(event.price_micros, event.size_micros);
            },
            BookEventKind::Upsert | BookEventKind::Delete => {
                levels.remove(&event.price_micros);
            },
        }
    }
    let bids = bids
        .into_iter()
        .rev()
        .map(|(price_micros, size_micros)| BookLevel {
            price_micros,
            size_micros,
        })
        .collect::<Vec<_>>();
    let asks = asks
        .into_iter()
        .map(|(price_micros, size_micros)| BookLevel {
            price_micros,
            size_micros,
        })
        .collect::<Vec<_>>();
    if matches!((bids.first(), asks.first()), (Some(bid), Some(ask)) if bid.price_micros > ask.price_micros)
    {
        anomalies.push(BookAnomaly::CrossedBook);
    }
    Some(BookSnapshot {
        provider: first.provider,
        instrument_id: first.instrument_id.clone(),
        sequence: prior.unwrap_or(first.sequence),
        bids,
        asks,
        anomalies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(sequence: u64, side: BookSide, price: u64, size: u64) -> BookEvent {
        BookEvent {
            provider: ProviderId::COINGECKO,
            instrument_id: "btc-usd".into(),
            sequence,
            side,
            price_micros: price,
            size_micros: size,
            kind: BookEventKind::Upsert,
            event_time: OffsetDateTime::UNIX_EPOCH,
            retrieved_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
    #[test]
    fn reconstructs_and_executes_real_depth() {
        let book = reconstruct_book(&[
            event(1, BookSide::Ask, 100, 5),
            event(2, BookSide::Ask, 110, 10),
            event(3, BookSide::Bid, 90, 7),
        ])
        .unwrap();
        let execution = book.execute_depth(BookSide::Ask, 12);
        assert_eq!(execution.filled_size_micros, 12);
        assert_eq!(execution.average_price_micros, Some(105));
    }
    #[test]
    fn sequence_gaps_are_never_hidden() {
        let book = reconstruct_book(&[
            event(1, BookSide::Bid, 90, 5),
            event(3, BookSide::Ask, 100, 5),
        ])
        .unwrap();
        assert!(matches!(book.anomalies[0], BookAnomaly::SequenceGap { .. }));
    }
}
