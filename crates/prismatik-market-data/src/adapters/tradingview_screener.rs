//! TradingView screener adapter — cross-market instrument discovery.
//!
//! # Status: unofficial
//!
//! This talks to `scanner.tradingview.com`, which TradingView does **not**
//! document, support, or offer under any published API terms. It is the same
//! endpoint the community `tradingview-screener` library uses. That has three
//! consequences the rest of this crate does not have to live with:
//!
//! - The response shape can change without notice, so every field is decoded
//!   defensively and a row that does not parse is dropped rather than faked.
//! - It carries no licensing guarantee, so results are treated as *discovery*
//!   input — candidates to go and analyse — never as citable observations. The
//!   quotes PRISMATIK reasons over still come from providers it has terms with.
//! - It must be opt-in. Nothing here is reachable unless an operator has
//!   explicitly enabled the unofficial source.
//!
//! The value it buys is real: without a screener you can only measure the
//! handful of instruments you already thought to track. With one you can sweep
//! thousands and go looking for the few that are worth tracking.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::http::{HttpMethod, HttpRequest, HttpTransport, TransportError};
use std::sync::Arc;
use thiserror::Error;

/// Screener adapter errors.
#[derive(Debug, Error)]
pub enum ScreenerError {
    /// HTTP transport failed.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// Endpoint returned an unsuccessful status.
    #[error("screener returned status {0}")]
    Status(u16),
    /// Response could not be decoded.
    #[error("screener decode: {0}")]
    Decode(String),
    /// The caller asked for a market this adapter does not serve.
    #[error("unsupported screener market: {0}")]
    UnsupportedMarket(String),
}

/// Markets the screener can sweep.
///
/// Deliberately a closed set rather than a free string: the market name is
/// interpolated into the request path, and an open string would let a caller
/// steer that path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenerMarket {
    /// US equities.
    America,
    /// Crypto pairs across centralised and decentralised venues.
    Crypto,
    /// FX pairs.
    Forex,
}

impl ScreenerMarket {
    /// Field carrying *turnover* — price x volume — for this market, if any.
    ///
    /// Ranking by raw unit volume is actively misleading across price scales:
    /// 448M shares of a $0.0002 OTC shell is about $90k of real turnover and
    /// outranks SPY, and the crypto market is topped by DEX pairs priced at
    /// 1e-17 with astronomical unit counts. Turnover is the only ranking that
    /// means the same thing for every instrument.
    ///
    /// The field differs per market and forex exposes none, so this returns
    /// `None` there and the caller falls back to a price floor alone.
    pub fn turnover_field(self) -> Option<&'static str> {
        match self {
            Self::America => Some("Value.Traded"),
            Self::Crypto => Some("24h_vol|5"),
            Self::Forex => None,
        }
    }

    /// Path segment for this market.
    pub fn slug(self) -> &'static str {
        match self {
            Self::America => "america",
            Self::Crypto => "crypto",
            Self::Forex => "forex",
        }
    }

    /// Parse a caller-supplied market name.
    pub fn parse(value: &str) -> Result<Self, ScreenerError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "america" | "us" | "equity" | "stocks" => Ok(Self::America),
            "crypto" => Ok(Self::Crypto),
            "forex" | "fx" => Ok(Self::Forex),
            other => Err(ScreenerError::UnsupportedMarket(other.to_owned())),
        }
    }
}

/// How to rank the sweep.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenerSort {
    /// Highest turnover, falling back to unit volume where unavailable.
    Turnover,
    /// Highest traded volume, in units.
    Volume,
    /// Largest positive change.
    ChangeDesc,
    /// Largest negative change.
    ChangeAsc,
    /// Largest market capitalisation.
    MarketCap,
}

impl ScreenerSort {
    fn field(self, market: ScreenerMarket) -> &'static str {
        match self {
            Self::Turnover => market.turnover_field().unwrap_or("volume"),
            Self::Volume => "volume",
            Self::ChangeDesc | Self::ChangeAsc => "change",
            Self::MarketCap => "market_cap_basic",
        }
    }

    fn order(self) -> &'static str {
        match self {
            Self::ChangeAsc => "asc",
            _ => "desc",
        }
    }
}

/// One instrument returned by a sweep.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScreenerHit {
    /// Bare ticker, e.g. `AAPL`.
    pub symbol: String,
    /// Venue-qualified identifier as returned, e.g. `NASDAQ:AAPL`.
    pub qualified: String,
    /// Listing venue.
    pub exchange: String,
    /// Last price.
    pub price: f64,
    /// Session change, percent.
    pub change_pct: f64,
    /// Traded volume.
    pub volume: f64,
    /// Market capitalisation, when the market reports one.
    pub market_cap: Option<f64>,
    /// Market this hit came from.
    pub market: ScreenerMarket,
    /// When the sweep ran.
    pub retrieved_at: OffsetDateTime,
}

/// Result of one sweep.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScreenerPage {
    /// Instruments that decoded cleanly.
    pub hits: Vec<ScreenerHit>,
    /// Total matching the filter upstream, before paging.
    pub total_count: usize,
    /// Rows the endpoint returned that could not be decoded.
    ///
    /// Surfaced rather than swallowed: a sudden rise here is how an
    /// undocumented response-shape change becomes visible instead of silently
    /// shrinking every sweep.
    pub undecodable_rows: usize,
}

/// Read-only screener over TradingView's public scanner endpoint.
pub struct TradingViewScreener {
    transport: Arc<dyn HttpTransport>,
}

impl std::fmt::Debug for TradingViewScreener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TradingViewScreener")
            .finish_non_exhaustive()
    }
}

/// Columns requested, in the order their values arrive in `d`.
const COLUMNS: [&str; 6] = [
    "name",
    "close",
    "change",
    "volume",
    "market_cap_basic",
    "exchange",
];

/// Upper bound on rows per sweep, to keep one call bounded.
const MAX_LIMIT: usize = 200;

impl TradingViewScreener {
    /// Construct with an injected transport.
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self { transport }
    }

    /// Sweep one market.
    ///
    /// `min_turnover` removes the illiquid tail in currency terms, and
    /// `min_price` removes the sub-penny tail that no desk can act on. Both
    /// matter: without them a volume-ranked sweep returns OTC shells and DEX
    /// dust rather than anything tradeable.
    pub async fn scan(
        &self,
        market: ScreenerMarket,
        sort: ScreenerSort,
        limit: usize,
        min_turnover: f64,
        min_price: f64,
        retrieved_at: OffsetDateTime,
    ) -> Result<ScreenerPage, ScreenerError> {
        let limit = limit.clamp(1, MAX_LIMIT);
        let mut filter = Vec::new();
        if min_turnover > 0.0 {
            // Where the market exposes no turnover field, fall back to unit
            // volume rather than dropping the liquidity floor entirely.
            match market.turnover_field() {
                Some(field) => filter.push(serde_json::json!({
                    "left": field,
                    "operation": "egreater",
                    "right": min_turnover,
                })),
                None => filter.push(serde_json::json!({
                    "left": "volume",
                    "operation": "egreater",
                    "right": min_turnover,
                })),
            }
        }
        if min_price > 0.0 {
            filter.push(serde_json::json!({
                "left": "close",
                "operation": "egreater",
                "right": min_price,
            }));
        }

        let body = serde_json::json!({
            "filter": filter,
            "options": { "lang": "en" },
            "markets": [market.slug()],
            "symbols": { "query": { "types": [] }, "tickers": [] },
            "columns": COLUMNS,
            "sort": { "sortBy": sort.field(market), "sortOrder": sort.order() },
            "range": [0, limit],
        })
        .to_string();

        let mut headers = BTreeMap::new();
        headers.insert("Content-Type".to_owned(), "application/json".to_owned());

        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Post,
                path: format!("/{}/scan", market.slug()),
                query: BTreeMap::new(),
                headers,
                body: Some(body),
            })
            .await?;
        if response.status != 200 {
            return Err(ScreenerError::Status(response.status));
        }

        let envelope: ScanEnvelope = serde_json::from_str(&response.body)
            .map_err(|error| ScreenerError::Decode(error.to_string()))?;

        let mut hits = Vec::with_capacity(envelope.data.len());
        let mut undecodable = 0_usize;
        for row in envelope.data {
            match decode_row(&row, market, retrieved_at) {
                Some(hit) => hits.push(hit),
                None => undecodable += 1,
            }
        }

        Ok(ScreenerPage {
            hits,
            total_count: envelope.total_count,
            undecodable_rows: undecodable,
        })
    }
}

#[derive(Deserialize)]
struct ScanEnvelope {
    #[serde(rename = "totalCount", default)]
    total_count: usize,
    #[serde(default)]
    data: Vec<ScanRow>,
}

#[derive(Deserialize)]
struct ScanRow {
    #[serde(default)]
    s: String,
    #[serde(default)]
    d: Vec<serde_json::Value>,
}

/// Decode one row, or `None` when it does not match the requested columns.
///
/// Values arrive as a positional array matching [`COLUMNS`], with nulls for
/// fields a market does not report. A row missing a price is dropped: an
/// instrument with no price is not a screener hit, and defaulting it to zero
/// would put it at the top of an ascending sort.
fn decode_row(
    row: &ScanRow,
    market: ScreenerMarket,
    retrieved_at: OffsetDateTime,
) -> Option<ScreenerHit> {
    if row.d.len() < COLUMNS.len() || row.s.trim().is_empty() {
        return None;
    }
    let price = row.d[1].as_f64()?;
    if !price.is_finite() || price <= 0.0 {
        return None;
    }
    let symbol = row.d[0].as_str().unwrap_or_default().trim().to_owned();
    if symbol.is_empty() {
        return None;
    }
    let exchange = row.d[5]
        .as_str()
        .map(str::to_owned)
        .or_else(|| row.s.split(':').next().map(str::to_owned))
        .unwrap_or_default();

    Some(ScreenerHit {
        symbol,
        qualified: row.s.clone(),
        exchange,
        price,
        change_pct: row.d[2].as_f64().filter(|v| v.is_finite()).unwrap_or(0.0),
        volume: row.d[3].as_f64().filter(|v| v.is_finite()).unwrap_or(0.0),
        market_cap: row.d[4].as_f64().filter(|v| v.is_finite()),
        market,
        retrieved_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap()
    }

    fn row(values: serde_json::Value, s: &str) -> ScanRow {
        ScanRow {
            s: s.to_owned(),
            d: values.as_array().cloned().unwrap_or_default(),
        }
    }

    #[test]
    fn market_slugs_are_closed_and_path_safe() {
        for market in [
            ScreenerMarket::America,
            ScreenerMarket::Crypto,
            ScreenerMarket::Forex,
        ] {
            let slug = market.slug();
            assert!(slug.chars().all(|c| c.is_ascii_lowercase()));
            assert!(!slug.contains('/') && !slug.contains('.'));
        }
    }

    #[test]
    fn unknown_markets_are_rejected_rather_than_interpolated() {
        // The market name lands in the request path, so a traversal attempt
        // must fail rather than reach the transport.
        assert!(ScreenerMarket::parse("../admin").is_err());
        assert!(ScreenerMarket::parse("").is_err());
        assert_eq!(ScreenerMarket::parse("FX").unwrap(), ScreenerMarket::Forex);
        assert_eq!(
            ScreenerMarket::parse(" stocks ").unwrap(),
            ScreenerMarket::America
        );
    }

    #[test]
    fn a_well_formed_row_decodes() {
        let hit = decode_row(
            &row(
                serde_json::json!(["AAPL", 232.5, 1.25, 41_000_000.0, 3.4e12, "NASDAQ"]),
                "NASDAQ:AAPL",
            ),
            ScreenerMarket::America,
            now(),
        )
        .expect("decodes");
        assert_eq!(hit.symbol, "AAPL");
        assert_eq!(hit.qualified, "NASDAQ:AAPL");
        assert_eq!(hit.exchange, "NASDAQ");
        assert_eq!(hit.price, 232.5);
        assert_eq!(hit.market_cap, Some(3.4e12));
    }

    #[test]
    fn a_row_without_a_usable_price_is_dropped() {
        // Zero or null prices must not become hits: an ascending sort would
        // rank them first, and a regime built on them would be meaningless.
        for price in [
            serde_json::json!(null),
            serde_json::json!(0.0),
            serde_json::json!(-1.0),
        ] {
            let value = serde_json::json!(["X", price, 0.0, 1.0, null, "EX"]);
            assert!(decode_row(&row(value, "EX:X"), ScreenerMarket::Crypto, now()).is_none());
        }
    }

    #[test]
    fn a_short_row_is_dropped_rather_than_padded() {
        // The endpoint is undocumented; a changed column set must shrink the
        // result set visibly, not produce hits with invented fields.
        let value = serde_json::json!(["AAPL", 232.5]);
        assert!(decode_row(&row(value, "NASDAQ:AAPL"), ScreenerMarket::America, now()).is_none());
    }

    #[test]
    fn a_missing_market_cap_is_none_not_zero() {
        let hit = decode_row(
            &row(
                serde_json::json!(["BTCUSD", 61000.0, -0.5, 900.0, null, "BINANCE"]),
                "BINANCE:BTCUSD",
            ),
            ScreenerMarket::Crypto,
            now(),
        )
        .expect("decodes");
        assert_eq!(hit.market_cap, None);
        assert_eq!(hit.change_pct, -0.5);
    }

    #[test]
    fn exchange_falls_back_to_the_qualified_prefix() {
        let hit = decode_row(
            &row(
                serde_json::json!(["EURUSD", 1.09, 0.1, 5.0, null, null]),
                "OANDA:EURUSD",
            ),
            ScreenerMarket::Forex,
            now(),
        )
        .expect("decodes");
        assert_eq!(hit.exchange, "OANDA");
    }

    #[test]
    fn sort_directions_match_their_intent() {
        assert_eq!(ScreenerSort::ChangeAsc.order(), "asc");
        assert_eq!(ScreenerSort::ChangeDesc.order(), "desc");
        assert_eq!(
            ScreenerSort::ChangeAsc.field(ScreenerMarket::America),
            "change"
        );
        assert_eq!(
            ScreenerSort::MarketCap.field(ScreenerMarket::America),
            "market_cap_basic"
        );
        // Turnover resolves per market, and falls back where none exists.
        assert_eq!(
            ScreenerSort::Turnover.field(ScreenerMarket::America),
            "Value.Traded"
        );
        assert_eq!(
            ScreenerSort::Turnover.field(ScreenerMarket::Crypto),
            "24h_vol|5"
        );
        assert_eq!(
            ScreenerSort::Turnover.field(ScreenerMarket::Forex),
            "volume"
        );
    }
}
