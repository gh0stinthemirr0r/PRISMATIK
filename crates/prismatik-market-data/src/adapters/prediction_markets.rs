//! Public prediction-venue market listings: Polymarket and Kalshi.
//!
//! Companion to `prediction_venues.rs`, which parses order books but has no
//! HTTP layer — these adapters fetch. Both endpoints are unauthenticated, so
//! "connected" records that the operator turned the poll on rather than that
//! a secret was stored.
//!
//! The two venues quote the same quantity in different shapes, and both are
//! easy to get subtly wrong:
//!
//! - Polymarket returns `outcomePrices` as a **JSON array encoded inside a
//!   string** — `"[\"0.0445\", \"0.9555\"]"` — so it needs parsing twice.
//! - Kalshi quotes in dollars per contract, which is already probability, but
//!   reports `0.0000` for unquoted markets. That is absence, not a
//!   zero-percent forecast, and it is recorded as `None`. Its untargeted
//!   listing is dominated by auto-generated sports parlays nothing is
//!   pricing, so reaching real markets means keeping what carries a quote —
//!   or asking for a series by name.

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
    types::PredictionMarket,
};

/// Prediction venue adapter errors.
#[derive(Debug, Error)]
pub enum PredictionVenueError {
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
}

fn decode(venue: &'static str, detail: impl Into<String>) -> PredictionVenueError {
    PredictionVenueError::Decode {
        venue,
        detail: detail.into(),
    }
}

/// Parse an RFC3339 timestamp, treating an unparsable one as absent.
///
/// A close time is context, not the payload. Losing the whole market because
/// one venue rendered a date unusually would be the wrong trade.
fn timestamp(value: Option<&str>) -> Option<OffsetDateTime> {
    let raw = value?;
    OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339).ok()
}

/// A quoted price is only a probability if it is inside `[0, 1]` and non-zero.
///
/// Zero means "nobody has quoted this", which every venue here renders the
/// same way as a genuine zero. Treating it as a forecast would fill the book
/// with confident impossibilities.
fn probability(raw: &str) -> Option<String> {
    let parsed: f64 = raw.trim().parse().ok()?;
    (parsed.is_finite() && parsed > 0.0 && parsed <= 1.0).then(|| raw.trim().to_owned())
}

// ---------------------------------------------------------------------------
// Polymarket
// ---------------------------------------------------------------------------

/// Public Polymarket gamma-API adapter.
pub struct PolymarketAdapter {
    transport: Arc<dyn HttpTransport>,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for PolymarketAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolymarketAdapter").finish_non_exhaustive()
    }
}

impl PolymarketAdapter {
    /// Construct using an injected HTTP transport.
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            transport,
            entitlements: [Entitlement::PublicVenue].into_iter().collect(),
        }
    }

    /// List open markets, newest first, up to `limit`.
    pub async fn open_markets(
        &self,
        limit: u32,
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<PredictionMarket>, PredictionVenueError> {
        const VENUE: &str = "Polymarket";
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/markets".into(),
                query: BTreeMap::from([
                    ("limit".to_owned(), limit.to_string()),
                    ("closed".to_owned(), "false".to_owned()),
                ]),
                headers: BTreeMap::new(),
                body: None,
            })
            .await?;
        if response.status != 200 {
            return Err(PredictionVenueError::Status {
                venue: VENUE,
                status: response.status,
            });
        }
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&response.body).map_err(|e| decode(VENUE, e.to_string()))?;

        Ok(rows
            .iter()
            .filter_map(|row| {
                Some(PredictionMarket {
                    id: row.get("id")?.as_str()?.to_owned(),
                    question: row.get("question")?.as_str()?.to_owned(),
                    // `outcomePrices` is a JSON array serialised *into a
                    // string*, so it parses twice. The first element is YES.
                    yes_price: row
                        .get("outcomePrices")
                        .and_then(|v| v.as_str())
                        .and_then(|text| serde_json::from_str::<Vec<String>>(text).ok())
                        .and_then(|prices| prices.first().cloned())
                        .and_then(|price| probability(&price)),
                    closes_at: timestamp(row.get("endDate").and_then(|v| v.as_str())),
                    provider: ProviderId::POLYMARKET,
                    retrieved_at,
                })
            })
            .collect())
    }
}

#[async_trait]
impl Provider for PolymarketAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::POLYMARKET
    }

    fn capabilities(&self) -> ProviderCapabilities {
        [Capability::Trending].into_iter().collect()
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
// Kalshi
// ---------------------------------------------------------------------------

/// Public Kalshi trade-API adapter.
pub struct KalshiAdapter {
    transport: Arc<dyn HttpTransport>,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for KalshiAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KalshiAdapter").finish_non_exhaustive()
    }
}

impl KalshiAdapter {
    /// Construct using an injected HTTP transport.
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            transport,
            entitlements: [Entitlement::PublicVenue].into_iter().collect(),
        }
    }

    /// List open markets that carry a live quote, up to `limit`.
    ///
    /// The unfiltered `status=open` listing is not usable as-is. Its default
    /// ordering is dominated by auto-generated multi-leg sports parlays with
    /// no bid on either side, and a thousand-row page can contain not one
    /// quoted market. The endpoint offers no sort, so the only way through it
    /// is to over-fetch and keep what the venue is actually pricing.
    ///
    /// A market with no bid has no implied probability, which is the entire
    /// reason to read a prediction venue, so those are dropped.
    ///
    /// When the caller knows what it wants, [`Self::markets_in_series`] is
    /// one exact request instead of a thousand rows.
    pub async fn open_markets(
        &self,
        limit: u32,
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<PredictionMarket>, PredictionVenueError> {
        self.fetch(None, limit, retrieved_at, true).await
    }

    /// Every open market in one series, e.g. `KXFEDDECISION`.
    ///
    /// Quotes are reported as the venue gives them and nothing is filtered:
    /// an unquoted market inside an explicitly requested series is a fact
    /// about that series, not noise to hide.
    pub async fn markets_in_series(
        &self,
        series_ticker: &str,
        limit: u32,
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<PredictionMarket>, PredictionVenueError> {
        self.fetch(Some(series_ticker), limit, retrieved_at, false)
            .await
    }

    async fn fetch(
        &self,
        series_ticker: Option<&str>,
        limit: u32,
        retrieved_at: OffsetDateTime,
        require_quote: bool,
    ) -> Result<Vec<PredictionMarket>, PredictionVenueError> {
        const VENUE: &str = "Kalshi";
        /// Kalshi's maximum page size.
        const MAX_PAGE: u32 = 1_000;
        // Only the untargeted listing needs the wide sweep.
        let page = if require_quote {
            limit.saturating_mul(50).clamp(limit, MAX_PAGE)
        } else {
            limit.min(MAX_PAGE)
        };
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/trade-api/v2/markets".into(),
                query: {
                    let mut query = BTreeMap::from([
                        ("limit".to_owned(), page.to_string()),
                        ("status".to_owned(), "open".to_owned()),
                    ]);
                    if let Some(series) = series_ticker {
                        query.insert("series_ticker".to_owned(), series.to_owned());
                    }
                    query
                },
                headers: BTreeMap::new(),
                body: None,
            })
            .await?;
        if response.status != 200 {
            return Err(PredictionVenueError::Status {
                venue: VENUE,
                status: response.status,
            });
        }
        let envelope: serde_json::Value =
            serde_json::from_str(&response.body).map_err(|e| decode(VENUE, e.to_string()))?;
        let rows = envelope
            .get("markets")
            .and_then(|m| m.as_array())
            .ok_or_else(|| decode(VENUE, "response carried no markets array"))?;

        Ok(rows
            .iter()
            .filter_map(|row| {
                Some(PredictionMarket {
                    id: row.get("ticker")?.as_str()?.to_owned(),
                    question: row.get("title")?.as_str()?.to_owned(),
                    // Kalshi quotes dollars per contract, which is already a
                    // probability. An unquoted market reports "0.0000", which
                    // `probability` rejects as absence rather than recording a
                    // zero-percent forecast.
                    yes_price: row
                        .get("yes_bid_dollars")
                        .and_then(|v| v.as_str())
                        .and_then(probability),
                    closes_at: timestamp(row.get("close_time").and_then(|v| v.as_str())),
                    provider: ProviderId::KALSHI,
                    retrieved_at,
                })
            })
            .filter(|market| !require_quote || market.yes_price.is_some())
            .take(limit as usize)
            .collect())
    }
}

#[async_trait]
impl Provider for KalshiAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::KALSHI
    }

    fn capabilities(&self) -> ProviderCapabilities {
        [Capability::Trending].into_iter().collect()
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

    struct Canned(&'static str);

    #[async_trait]
    impl HttpTransport for Canned {
        async fn execute(
            &self,
            _request: &HttpRequest,
        ) -> Result<crate::http::HttpResponse, TransportError> {
            Ok(crate::http::HttpResponse {
                status: 200,
                headers: BTreeMap::new(),
                body: self.0.to_owned(),
            })
        }
    }

    #[test]
    fn an_unquoted_market_has_no_probability() {
        // Every venue here renders "nobody has quoted this" as zero. Reading
        // that as a zero-percent forecast would fill the book with confident
        // impossibilities.
        assert_eq!(probability("0.0000"), None);
        assert_eq!(probability("0"), None);
        assert_eq!(probability("0.0445"), Some("0.0445".to_owned()));
        assert_eq!(probability("1.0000"), Some("1.0000".to_owned()));
        // Outside the unit interval it is not a probability at all.
        assert_eq!(probability("1.5"), None);
        assert_eq!(probability("-0.2"), None);
        assert_eq!(probability("not a number"), None);
    }

    #[tokio::test]
    async fn polymarket_unwraps_its_double_encoded_prices() {
        // outcomePrices is a JSON array serialised into a string.
        let adapter = PolymarketAdapter::new(Arc::new(Canned(
            r#"[{"id":"559651","question":"Xi out before 2027?",
                 "outcomePrices":"[\"0.0445\", \"0.9555\"]",
                 "endDate":"2026-12-31T00:00:00Z"}]"#,
        )));
        let markets = adapter.open_markets(1, now()).await.expect("markets");
        assert_eq!(markets.len(), 1);
        assert_eq!(markets[0].yes_price.as_deref(), Some("0.0445"));
        assert!(markets[0].closes_at.is_some());
    }

    #[tokio::test]
    async fn kalshi_reads_its_markets_envelope() {
        // The rows sit under a `markets` key rather than at the top level.
        let adapter = KalshiAdapter::new(Arc::new(Canned(
            r#"{"markets":[{"ticker":"ABC","title":"Something",
                 "yes_bid_dollars":"0.6100","close_time":"2026-08-17T20:00:00Z"}]}"#,
        )));
        let markets = adapter.open_markets(5, now()).await.expect("markets");
        assert_eq!(markets.len(), 1);
        assert_eq!(markets[0].yes_price.as_deref(), Some("0.6100"));
        assert!(markets[0].closes_at.is_some());
    }

    #[tokio::test]
    async fn the_untargeted_listing_keeps_only_quoted_markets() {
        let adapter = KalshiAdapter::new(Arc::new(Canned(
            r#"{"markets":[
                 {"ticker":"QUOTED","title":"Real","yes_bid_dollars":"0.6400"},
                 {"ticker":"UNQUOTED","title":"Parlay","yes_bid_dollars":"0.0000"}]}"#,
        )));
        let markets = adapter.open_markets(10, now()).await.expect("markets");
        assert_eq!(markets.len(), 1);
        assert_eq!(markets[0].id, "QUOTED");
    }

    #[tokio::test]
    async fn a_series_query_reports_unquoted_markets() {
        // Inside a named series an unquoted market is information, not noise.
        let adapter = KalshiAdapter::new(Arc::new(Canned(
            r#"{"markets":[
                 {"ticker":"KXFEDDECISION-A","title":"Cut","yes_bid_dollars":"0.6400"},
                 {"ticker":"KXFEDDECISION-B","title":"Hold","yes_bid_dollars":"0.0000"}]}"#,
        )));
        let markets = adapter
            .markets_in_series("KXFEDDECISION", 10, now())
            .await
            .expect("markets");
        assert_eq!(markets.len(), 2);
        assert_eq!(markets[1].yes_price, None);
    }

    #[tokio::test]
    async fn a_malformed_close_time_does_not_lose_the_market() {
        let adapter = KalshiAdapter::new(Arc::new(Canned(
            r#"{"markets":[{"ticker":"ABC","title":"Something",
                 "yes_bid_dollars":"0.5000","close_time":"next tuesday"}]}"#,
        )));
        let markets = adapter.open_markets(1, now()).await.expect("markets");
        assert_eq!(markets.len(), 1);
        assert_eq!(markets[0].closes_at, None);
        assert_eq!(markets[0].yes_price.as_deref(), Some("0.5000"));
    }
}
