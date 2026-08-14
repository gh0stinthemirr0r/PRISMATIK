//! Public crypto venue candle adapters: Kraken, Coinbase Exchange, Binance.
//!
//! Both answer the same question — daily OHLCV for one pair — over an
//! unauthenticated endpoint, so they share a file and an error type. Keeping
//! them side by side also keeps the one place they genuinely disagree
//! visible: Coinbase returns its candle columns in a non-obvious order, and
//! reading them positionally as OHLC silently swaps two of the four prices.
//!
//! Neither needs a credential. "Connected" for these providers records that
//! the operator turned the poll on, not that a secret was stored.

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use prismatik_domain::ProviderId;
use thiserror::Error;
use time::OffsetDateTime;

use crate::{
    http::{HttpMethod, HttpRequest, HttpTransport, TransportError},
    provider::{
        Capability, Entitlement, EntitlementSet, Provider, ProviderCapabilities, ProviderHealth,
    },
    request::{CostUnits, ProviderRequest},
    types::VenueCandle,
};

/// Public venue adapter errors.
#[derive(Debug, Error)]
pub enum VenueError {
    /// HTTP transport failed.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The venue returned an unsuccessful response.
    #[error("{venue} returned status {status}")]
    Status {
        /// Venue name.
        venue: &'static str,
        /// HTTP status.
        status: u16,
    },
    /// Response decoding failed.
    #[error("{venue} decode: {detail}")]
    Decode {
        /// Venue name.
        venue: &'static str,
        /// What could not be read.
        detail: String,
    },
    /// The venue answered with an application-level error.
    #[error("{venue}: {detail}")]
    Venue {
        /// Venue name.
        venue: &'static str,
        /// Message as reported.
        detail: String,
    },
}

fn decode(venue: &'static str, detail: impl Into<String>) -> VenueError {
    VenueError::Decode {
        venue,
        detail: detail.into(),
    }
}

/// Seconds since the epoch to a UTC timestamp.
fn from_unix(venue: &'static str, seconds: i64) -> Result<OffsetDateTime, VenueError> {
    OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|error| decode(venue, format!("unusable timestamp {seconds}: {error}")))
}

/// Render a JSON number or string as a decimal string without going through
/// binary floating point where the source already gave us text.
fn decimal(venue: &'static str, value: &serde_json::Value) -> Result<String, VenueError> {
    match value {
        serde_json::Value::String(text) => Ok(text.clone()),
        serde_json::Value::Number(number) => Ok(number.to_string()),
        other => Err(decode(venue, format!("expected a price, got {other}"))),
    }
}

// ---------------------------------------------------------------------------
// Kraken
// ---------------------------------------------------------------------------

/// Public Kraken OHLC adapter.
pub struct KrakenAdapter {
    transport: Arc<dyn HttpTransport>,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for KrakenAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KrakenAdapter").finish_non_exhaustive()
    }
}

impl KrakenAdapter {
    /// Construct using an injected HTTP transport.
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            transport,
            entitlements: [Entitlement::PublicVenue].into_iter().collect(),
        }
    }

    /// Daily candles for a pair, e.g. `XBTUSD`.
    ///
    /// Kraken keys its result object by its *own* normalised pair name, which
    /// is often not what was asked for — `XBTUSD` comes back as `XXBTZUSD`.
    /// The single non-`last` key is taken rather than the requested one,
    /// because looking up the request would find nothing and read as an empty
    /// market.
    pub async fn candles(
        &self,
        pair: &str,
        interval_minutes: u32,
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<VenueCandle>, VenueError> {
        const VENUE: &str = "Kraken";
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/0/public/OHLC".into(),
                query: BTreeMap::from([
                    ("pair".to_owned(), pair.to_owned()),
                    ("interval".to_owned(), interval_minutes.to_string()),
                ]),
                headers: BTreeMap::new(),
                body: None,
            })
            .await?;
        if response.status != 200 {
            return Err(VenueError::Status {
                venue: VENUE,
                status: response.status,
            });
        }
        let value: serde_json::Value =
            serde_json::from_str(&response.body).map_err(|e| decode(VENUE, e.to_string()))?;

        // Kraken answers 200 with a populated `error` array on a bad pair, so
        // the status code alone does not mean success.
        if let Some(errors) = value.get("error").and_then(|e| e.as_array()) {
            if !errors.is_empty() {
                return Err(VenueError::Venue {
                    venue: VENUE,
                    detail: errors
                        .iter()
                        .filter_map(|e| e.as_str())
                        .collect::<Vec<_>>()
                        .join("; "),
                });
            }
        }
        let result = value
            .get("result")
            .and_then(|r| r.as_object())
            .ok_or_else(|| decode(VENUE, "response carried no result object"))?;
        let rows = result
            .iter()
            .find(|(key, _)| key.as_str() != "last")
            .map(|(_, rows)| rows)
            .and_then(|rows| rows.as_array())
            .ok_or_else(|| decode(VENUE, format!("no candle series for pair {pair}")))?;

        rows.iter()
            .map(|row| {
                let cells = row
                    .as_array()
                    .ok_or_else(|| decode(VENUE, "candle row is not an array"))?;
                // [time, open, high, low, close, vwap, volume, count]
                if cells.len() < 7 {
                    return Err(decode(
                        VENUE,
                        format!("short candle row: {} cells", cells.len()),
                    ));
                }
                Ok(VenueCandle {
                    pair: pair.to_uppercase(),
                    bar_start: from_unix(
                        VENUE,
                        cells[0]
                            .as_i64()
                            .ok_or_else(|| decode(VENUE, "candle time is not an integer"))?,
                    )?,
                    open: decimal(VENUE, &cells[1])?,
                    high: decimal(VENUE, &cells[2])?,
                    low: decimal(VENUE, &cells[3])?,
                    close: decimal(VENUE, &cells[4])?,
                    volume: decimal(VENUE, &cells[6])?,
                    provider: ProviderId::KRAKEN,
                    retrieved_at,
                })
            })
            .collect()
    }
}

#[async_trait]
impl Provider for KrakenAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::KRAKEN
    }

    fn capabilities(&self) -> ProviderCapabilities {
        [Capability::Ohlcv, Capability::Ohlc].into_iter().collect()
    }

    fn entitlements(&self) -> &EntitlementSet {
        &self.entitlements
    }

    fn cost_of(&self, _request: &ProviderRequest) -> CostUnits {
        CostUnits::new(1)
    }

    async fn health(&self) -> ProviderHealth {
        ProviderHealth::healthy()
    }
}

// ---------------------------------------------------------------------------
// Coinbase Exchange
// ---------------------------------------------------------------------------

/// Public Coinbase Exchange candle adapter.
pub struct CoinbaseAdapter {
    transport: Arc<dyn HttpTransport>,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for CoinbaseAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoinbaseAdapter").finish_non_exhaustive()
    }
}

impl CoinbaseAdapter {
    /// Construct using an injected HTTP transport.
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            transport,
            entitlements: [Entitlement::PublicVenue].into_iter().collect(),
        }
    }

    /// Candles for a product, e.g. `BTC-USD`, at a granularity in seconds.
    ///
    /// **Coinbase orders its columns `[time, low, high, open, close, volume]`**
    /// — low and high come *before* open and close, which is not the order
    /// any other venue here uses. Reading them positionally as OHLC swaps the
    /// open with the low and the high with the close, producing candles that
    /// look plausible and are wrong. That is the entire reason this adapter
    /// does not share a row parser with Kraken.
    pub async fn candles(
        &self,
        product: &str,
        granularity_seconds: u32,
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<VenueCandle>, VenueError> {
        const VENUE: &str = "Coinbase";
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: format!("/products/{product}/candles"),
                query: BTreeMap::from([(
                    "granularity".to_owned(),
                    granularity_seconds.to_string(),
                )]),
                // Coinbase rejects requests without a User-Agent.
                headers: BTreeMap::from([(
                    "user-agent".to_owned(),
                    "Mythos-PRISMATIK/0.1".to_owned(),
                )]),
                body: None,
            })
            .await?;
        if response.status != 200 {
            return Err(VenueError::Status {
                venue: VENUE,
                status: response.status,
            });
        }
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&response.body).map_err(|e| decode(VENUE, e.to_string()))?;

        rows.iter()
            .map(|row| {
                let cells = row
                    .as_array()
                    .ok_or_else(|| decode(VENUE, "candle row is not an array"))?;
                if cells.len() < 6 {
                    return Err(decode(
                        VENUE,
                        format!("short candle row: {} cells", cells.len()),
                    ));
                }
                Ok(VenueCandle {
                    pair: product.to_uppercase(),
                    bar_start: from_unix(
                        VENUE,
                        cells[0]
                            .as_i64()
                            .ok_or_else(|| decode(VENUE, "candle time is not an integer"))?,
                    )?,
                    // Indices are deliberate and non-obvious. See the doc above.
                    low: decimal(VENUE, &cells[1])?,
                    high: decimal(VENUE, &cells[2])?,
                    open: decimal(VENUE, &cells[3])?,
                    close: decimal(VENUE, &cells[4])?,
                    volume: decimal(VENUE, &cells[5])?,
                    provider: ProviderId::COINBASE,
                    retrieved_at,
                })
            })
            .collect()
    }
}

#[async_trait]
impl Provider for CoinbaseAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::COINBASE
    }

    fn capabilities(&self) -> ProviderCapabilities {
        [Capability::Ohlcv, Capability::Ohlc].into_iter().collect()
    }

    fn entitlements(&self) -> &EntitlementSet {
        &self.entitlements
    }

    fn cost_of(&self, _request: &ProviderRequest) -> CostUnits {
        CostUnits::new(1)
    }

    async fn health(&self) -> ProviderHealth {
        ProviderHealth::healthy()
    }
}

// ---------------------------------------------------------------------------
// Binance
// ---------------------------------------------------------------------------

/// Public Binance kline adapter.
///
/// Defaults to `api.binance.us`. The global `api.binance.com` answers HTTP
/// 451 to US addresses — a jurisdictional block, not a rate limit or a bad
/// request — so pointing at it by default would make the integration look
/// broken for most of this desk's users. The host is the transport's base
/// URL, so anyone outside that block can supply the global endpoint instead.
pub struct BinanceAdapter {
    transport: Arc<dyn HttpTransport>,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for BinanceAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceAdapter").finish_non_exhaustive()
    }
}

impl BinanceAdapter {
    /// Construct using an injected HTTP transport.
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            transport,
            entitlements: [Entitlement::PublicVenue].into_iter().collect(),
        }
    }

    /// Klines for a symbol, e.g. `BTCUSDT`, at an interval such as `1d`.
    pub async fn candles(
        &self,
        symbol: &str,
        interval: &str,
        limit: u32,
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<VenueCandle>, VenueError> {
        const VENUE: &str = "Binance";
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/api/v3/klines".into(),
                query: BTreeMap::from([
                    ("symbol".to_owned(), symbol.to_uppercase()),
                    ("interval".to_owned(), interval.to_owned()),
                    ("limit".to_owned(), limit.to_string()),
                ]),
                headers: BTreeMap::new(),
                body: None,
            })
            .await?;
        if response.status == 451 {
            return Err(VenueError::Venue {
                venue: VENUE,
                detail: "this endpoint is not available from your jurisdiction (HTTP 451). \
                         api.binance.us serves US addresses; api.binance.com serves most others."
                    .to_owned(),
            });
        }
        if response.status != 200 {
            return Err(VenueError::Status {
                venue: VENUE,
                status: response.status,
            });
        }
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&response.body).map_err(|e| decode(VENUE, e.to_string()))?;

        rows.iter()
            .map(|row| {
                let cells = row
                    .as_array()
                    .ok_or_else(|| decode(VENUE, "kline row is not an array"))?;
                // [open_time_ms, open, high, low, close, volume, close_time, ...]
                if cells.len() < 6 {
                    return Err(decode(
                        VENUE,
                        format!("short kline row: {} cells", cells.len()),
                    ));
                }
                // Binance stamps in milliseconds where Kraken and Coinbase use
                // seconds. Reading it as seconds would place every candle
                // roughly fifty thousand years in the future.
                let millis = cells[0]
                    .as_i64()
                    .ok_or_else(|| decode(VENUE, "kline time is not an integer"))?;
                Ok(VenueCandle {
                    pair: symbol.to_uppercase(),
                    bar_start: from_unix(VENUE, millis / 1_000)?,
                    open: decimal(VENUE, &cells[1])?,
                    high: decimal(VENUE, &cells[2])?,
                    low: decimal(VENUE, &cells[3])?,
                    close: decimal(VENUE, &cells[4])?,
                    volume: decimal(VENUE, &cells[5])?,
                    provider: ProviderId::BINANCE,
                    retrieved_at,
                })
            })
            .collect()
    }
}

#[async_trait]
impl Provider for BinanceAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::BINANCE
    }

    fn capabilities(&self) -> ProviderCapabilities {
        [Capability::Ohlcv, Capability::Ohlc].into_iter().collect()
    }

    fn entitlements(&self) -> &EntitlementSet {
        &self.entitlements
    }

    fn cost_of(&self, _request: &ProviderRequest) -> CostUnits {
        CostUnits::new(1)
    }

    async fn health(&self) -> ProviderHealth {
        ProviderHealth::healthy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("timestamp")
    }

    struct Canned(&'static str, u16);

    #[async_trait]
    impl HttpTransport for Canned {
        async fn execute(
            &self,
            _request: &HttpRequest,
        ) -> Result<crate::http::HttpResponse, TransportError> {
            Ok(crate::http::HttpResponse {
                status: self.1,
                headers: BTreeMap::new(),
                body: self.0.to_owned(),
            })
        }
    }

    #[tokio::test]
    async fn coinbase_columns_are_read_in_its_own_order() {
        // [time, low, high, open, close, volume]. If this is ever "simplified"
        // to positional OHLC, open becomes the low and the candle is wrong in
        // a way no type checks.
        let adapter = CoinbaseAdapter::new(Arc::new(Canned(
            "[[1700000000, 10.0, 40.0, 20.0, 30.0, 5.5]]",
            200,
        )));
        let candles = adapter
            .candles("BTC-USD", 86_400, now())
            .await
            .expect("candles");
        let candle = &candles[0];
        assert_eq!(candle.low, "10.0");
        assert_eq!(candle.high, "40.0");
        assert_eq!(candle.open, "20.0");
        assert_eq!(candle.close, "30.0");
        assert_eq!(candle.volume, "5.5");
    }

    #[tokio::test]
    async fn kraken_reports_its_application_level_errors() {
        // Kraken answers 200 with a populated `error` array on a bad pair, so
        // trusting the status code would surface "no candles" instead of the
        // reason.
        let adapter = KrakenAdapter::new(Arc::new(Canned(
            r#"{"error":["EQuery:Unknown asset pair"],"result":{}}"#,
            200,
        )));
        let error = adapter
            .candles("NOTAPAIR", 1440, now())
            .await
            .expect_err("should surface the venue error");
        assert!(format!("{error}").contains("Unknown asset pair"), "{error}");
    }

    #[tokio::test]
    async fn binance_reads_millisecond_timestamps() {
        // Binance stamps in milliseconds where the other two use seconds.
        // Reading it as seconds puts every candle ~50,000 years out.
        let adapter = BinanceAdapter::new(Arc::new(Canned(
            r#"[[1700000000000,"1.0","2.0","0.5","1.5","3.3",1700086399999,"0",1,"0","0","0"]]"#,
            200,
        )));
        let candles = adapter
            .candles("BTCUSDT", "1d", 1, now())
            .await
            .expect("candles");
        assert_eq!(candles[0].bar_start.unix_timestamp(), 1_700_000_000);
        assert_eq!(candles[0].open, "1.0");
        assert_eq!(candles[0].volume, "3.3");
    }

    #[tokio::test]
    async fn binance_names_a_jurisdictional_block() {
        // 451 is not a rate limit or a bad request, and reporting it as a
        // bare status would send someone hunting a bug that is not there.
        let adapter = BinanceAdapter::new(Arc::new(Canned("{}", 451)));
        let error = adapter
            .candles("BTCUSDT", "1d", 1, now())
            .await
            .expect_err("should refuse");
        let text = format!("{error}");
        assert!(text.contains("jurisdiction"), "{text}");
        assert!(text.contains("binance.us"), "{text}");
    }

    #[tokio::test]
    async fn kraken_finds_its_series_under_a_renamed_key() {
        // XBTUSD is returned under XXBTZUSD; looking up the requested name
        // would find nothing and read as an empty market.
        let adapter = KrakenAdapter::new(Arc::new(Canned(
            r#"{"error":[],"result":{"XXBTZUSD":[[1700000000,"1","2","0.5","1.5","1.2","3.3",7]],"last":1700000000}}"#,
            200,
        )));
        let candles = adapter
            .candles("XBTUSD", 1440, now())
            .await
            .expect("candles");
        assert_eq!(candles.len(), 1);
        assert_eq!(candles[0].open, "1");
        assert_eq!(candles[0].high, "2");
        assert_eq!(candles[0].low, "0.5");
        assert_eq!(candles[0].close, "1.5");
        // Volume is column 6, not 5 — column 5 is the vwap.
        assert_eq!(candles[0].volume, "3.3");
    }
}
