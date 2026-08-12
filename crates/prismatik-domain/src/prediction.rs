//! Canonical prediction-event, contract, and cross-venue quote primitives.
//!
//! Values use fixed-point integers so comparison and replay do not depend on
//! floating-point rounding. Venue settlement language remains attached to the
//! contract; superficially similar questions are never merged by title alone.

use crate::ProviderId;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

/// One-millionth probability unit. `1_000_000` is certainty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProbabilityPpm(u32);

impl ProbabilityPpm {
    /// Construct a validated probability.
    pub fn new(value: u32) -> Result<Self, PredictionError> {
        (value <= 1_000_000)
            .then_some(Self(value))
            .ok_or(PredictionError::ProbabilityOutOfRange(value))
    }

    /// Return the fixed-point value.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Stable canonical event identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EventId(String);

impl EventId {
    /// Construct a non-empty event id.
    pub fn new(value: impl Into<String>) -> Result<Self, PredictionError> {
        nonempty(value.into()).map(Self)
    }

    /// Borrow the identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable canonical contract identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContractId(String);

impl ContractId {
    /// Construct a non-empty contract id.
    pub fn new(value: impl Into<String>) -> Result<Self, PredictionError> {
        nonempty(value.into()).map(Self)
    }

    /// Borrow the identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn nonempty(value: String) -> Result<String, PredictionError> {
    if value.trim().is_empty() {
        Err(PredictionError::EmptyIdentifier)
    } else {
        Ok(value)
    }
}

/// Event lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    /// The event is accepting observations or venue activity.
    Open,
    /// The event is temporarily not accepting venue activity.
    Suspended,
    /// The event has a final outcome.
    Resolved,
    /// The event was cancelled without a normal outcome.
    Voided,
}

/// Contract outcome.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Affirmative binary outcome.
    Yes,
    /// Negative binary outcome.
    No,
    /// Named outcome for non-binary markets.
    Named(String),
}

/// Canonical event shared by one or more venue contracts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMarket {
    /// Stable canonical event identifier.
    pub id: EventId,
    /// Human-readable event title.
    pub title: String,
    /// Normalized event category.
    pub category: String,
    /// Current lifecycle state.
    pub status: EventStatus,
    /// Latest instant at which resolution may occur, when known.
    pub resolution_deadline: Option<OffsetDateTime>,
}

/// Venue-independent contract identity plus exact resolution language.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredictionContract {
    /// Stable canonical contract identifier.
    pub id: ContractId,
    /// Canonical event containing this contract.
    pub event_id: EventId,
    /// Outcome purchased by this contract.
    pub outcome: Outcome,
    /// Authoritative source used to resolve the contract.
    pub resolution_source: String,
    /// Exact normalized resolution rules supplied by the venue.
    pub resolution_rules: String,
}

/// Normalized top-of-book quote from one venue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VenueMarketQuote {
    /// Canonical contract represented by the venue quote.
    pub contract_id: ContractId,
    /// Provider that produced the observation.
    pub provider: ProviderId,
    /// Provider-native market identifier retained for provenance.
    pub venue_market_id: String,
    /// Highest executable buy probability price.
    pub bid: ProbabilityPpm,
    /// Lowest executable sell probability price.
    pub ask: ProbabilityPpm,
    /// Bid-side quantity in millionths of the settlement unit.
    pub bid_size_micros: u64,
    /// Ask-side quantity in millionths of the settlement unit.
    pub ask_size_micros: u64,
    /// Venue taker fee in basis points.
    pub taker_fee_bps: u32,
    /// Provider-reported market event time.
    pub event_time: OffsetDateTime,
    /// Time the observation crossed the PRISMATIK provider boundary.
    pub retrieved_at: OffsetDateTime,
}

impl VenueMarketQuote {
    /// Validate spread, timestamps, and venue identity.
    pub fn validate(&self) -> Result<(), PredictionError> {
        if self.venue_market_id.trim().is_empty() {
            return Err(PredictionError::EmptyIdentifier);
        }
        if self.bid > self.ask {
            return Err(PredictionError::CrossedMarket);
        }
        if self.retrieved_at < self.event_time {
            return Err(PredictionError::RetrievalBeforeEvent);
        }
        Ok(())
    }
}

/// Best executable cross-venue market for one canonical contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossVenueMarket {
    /// Canonical contract shared by all compared quotes.
    pub contract_id: ContractId,
    /// Venue quote containing the highest bid.
    pub best_bid: VenueMarketQuote,
    /// Venue quote containing the lowest ask.
    pub best_ask: VenueMarketQuote,
    /// Positive only when buying at best ask and selling at best bid remains
    /// profitable after both venues' taker fees.
    pub net_arbitrage_ppm: i64,
    /// Maximum immediately executable quantity across both selected sides.
    pub executable_size_micros: u64,
}

/// Select best executable prices without merging different contracts.
pub fn best_cross_venue_market(
    quotes: &[VenueMarketQuote],
) -> Result<CrossVenueMarket, PredictionError> {
    let first = quotes.first().ok_or(PredictionError::NoQuotes)?;
    for quote in quotes {
        quote.validate()?;
        if quote.contract_id != first.contract_id {
            return Err(PredictionError::MixedContracts);
        }
    }
    let best_bid = quotes
        .iter()
        .max_by_key(|quote| quote.bid)
        .expect("non-empty")
        .clone();
    let best_ask = quotes
        .iter()
        .min_by_key(|quote| quote.ask)
        .expect("non-empty")
        .clone();
    let gross = i64::from(best_bid.bid.get()) - i64::from(best_ask.ask.get());
    let fee = |quote: &VenueMarketQuote, price: ProbabilityPpm| {
        i64::from(price.get()) * i64::from(quote.taker_fee_bps) / 10_000
    };
    let net_arbitrage_ppm = gross - fee(&best_bid, best_bid.bid) - fee(&best_ask, best_ask.ask);
    let executable_size_micros = best_bid.bid_size_micros.min(best_ask.ask_size_micros);
    Ok(CrossVenueMarket {
        contract_id: first.contract_id.clone(),
        best_bid,
        best_ask,
        net_arbitrage_ppm,
        executable_size_micros,
    })
}

/// Prediction-domain validation errors.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PredictionError {
    /// A required identifier contained no non-whitespace characters.
    #[error("identifier must not be empty")]
    EmptyIdentifier,
    /// A fixed-point probability exceeded certainty.
    #[error("probability {0} exceeds 1,000,000 ppm")]
    ProbabilityOutOfRange(u32),
    /// A quote's bid exceeded its ask.
    #[error("bid exceeds ask")]
    CrossedMarket,
    /// Retrieval occurred before the provider-reported event time.
    #[error("retrieval time precedes event time")]
    RetrievalBeforeEvent,
    /// Cross-venue comparison received no quotes.
    #[error("no quotes supplied")]
    NoQuotes,
    /// Cross-venue comparison received different canonical contracts.
    #[error("quotes belong to different canonical contracts")]
    MixedContracts,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quote(provider: ProviderId, bid: u32, ask: u32, fee: u32, size: u64) -> VenueMarketQuote {
        VenueMarketQuote {
            contract_id: ContractId::new("event-1:yes").unwrap(),
            provider,
            venue_market_id: format!("venue-{}", provider.0),
            bid: ProbabilityPpm::new(bid).unwrap(),
            ask: ProbabilityPpm::new(ask).unwrap(),
            bid_size_micros: size,
            ask_size_micros: size,
            taker_fee_bps: fee,
            event_time: OffsetDateTime::UNIX_EPOCH,
            retrieved_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn finds_fee_adjusted_cross_venue_market_and_size() {
        let result = best_cross_venue_market(&[
            quote(ProviderId::POLYMARKET, 620_000, 625_000, 10, 2_000_000),
            quote(ProviderId::KALSHI, 640_000, 645_000, 20, 900_000),
        ])
        .unwrap();
        assert_eq!(result.best_bid.provider, ProviderId::KALSHI);
        assert_eq!(result.best_ask.provider, ProviderId::POLYMARKET);
        assert_eq!(result.executable_size_micros, 900_000);
        assert!(result.net_arbitrage_ppm > 0);
    }

    #[test]
    fn rejects_crossed_and_mixed_contract_quotes() {
        let crossed = quote(ProviderId::POLYMARKET, 700_000, 600_000, 0, 1);
        assert_eq!(crossed.validate(), Err(PredictionError::CrossedMarket));
        let mut other = quote(ProviderId::KALSHI, 500_000, 510_000, 0, 1);
        other.contract_id = ContractId::new("event-2:yes").unwrap();
        assert_eq!(
            best_cross_venue_market(&[
                quote(ProviderId::POLYMARKET, 500_000, 510_000, 0, 1),
                other
            ]),
            Err(PredictionError::MixedContracts)
        );
    }
}
