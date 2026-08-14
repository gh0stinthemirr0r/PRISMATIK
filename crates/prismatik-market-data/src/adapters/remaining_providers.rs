//! The last four catalogued providers: GDELT, Alpha Vantage, Polygon and
//! Interactive Brokers.
//!
//! Grouped because each is a small surface and they share nothing but the
//! transport. What they do *not* share is worth stating, since each fails in
//! its own way and each failure is easy to misread:
//!
//! - **Alpha Vantage** answers HTTP 200 for a rejected key, a rate limit and
//!   an unknown symbol alike, distinguishing them only by which JSON key it
//!   returns. Checking the status code proves nothing.
//! - **Polygon** returns 401 without a key and 403 when the plan does not
//!   cover the endpoint. Those are different problems with different fixes.
//! - **GDELT** rate-limits aggressively and answers 429 without a body.
//! - **Interactive Brokers** has no cloud REST API at all. It talks to a
//!   Client Portal Gateway the operator runs locally, so "not connected"
//!   usually means "the gateway is not running", not "the credential is
//!   wrong".

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

/// Errors shared by the providers in this module.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// HTTP transport failed.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The provider returned an unsuccessful response.
    #[error("{provider} returned status {status}")]
    Status {
        /// Provider name.
        provider: &'static str,
        /// HTTP status.
        status: u16,
    },
    /// Response decoding failed.
    #[error("{provider} decode: {detail}")]
    Decode {
        /// Provider name.
        provider: &'static str,
        /// What could not be read.
        detail: String,
    },
    /// The provider reported a problem in its response body.
    #[error("{provider}: {detail}")]
    Reported {
        /// Provider name.
        provider: &'static str,
        /// Message as reported.
        detail: String,
    },
}

fn decode(provider: &'static str, detail: impl Into<String>) -> ProviderError {
    ProviderError::Decode {
        provider,
        detail: detail.into(),
    }
}

fn reported(provider: &'static str, detail: impl Into<String>) -> ProviderError {
    ProviderError::Reported {
        provider,
        detail: detail.into(),
    }
}

// ---------------------------------------------------------------------------
// Alpha Vantage
// ---------------------------------------------------------------------------

/// Alpha Vantage daily-series adapter.
pub struct AlphaVantageAdapter {
    transport: Arc<dyn HttpTransport>,
    api_key: String,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for AlphaVantageAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlphaVantageAdapter")
            .finish_non_exhaustive()
    }
}

impl AlphaVantageAdapter {
    /// Construct with an API key.
    pub fn new(transport: Arc<dyn HttpTransport>, api_key: impl Into<String>) -> Self {
        Self {
            transport,
            api_key: api_key.into(),
            entitlements: [Entitlement::AlphaVantage].into_iter().collect(),
        }
    }

    /// Daily bars for one symbol.
    ///
    /// Alpha Vantage answers 200 for everything — a rejected key, a rate
    /// limit, an unknown symbol — and signals the difference only through
    /// which top-level JSON key comes back. Its `Note` and `Information`
    /// keys carry throttling and entitlement messages respectively, and its
    /// `Error Message` key carries a bad request. Those are surfaced verbatim
    /// rather than collapsed into "no data", because "you are rate limited"
    /// and "that symbol does not exist" call for opposite responses.
    pub async fn daily_bars(
        &self,
        symbol: &str,
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<VenueCandle>, ProviderError> {
        const PROVIDER: &str = "Alpha Vantage";
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/query".into(),
                query: BTreeMap::from([
                    ("function".to_owned(), "TIME_SERIES_DAILY".to_owned()),
                    ("symbol".to_owned(), symbol.to_uppercase()),
                    ("apikey".to_owned(), self.api_key.clone()),
                ]),
                headers: BTreeMap::new(),
                body: None,
            })
            .await?;
        if response.status != 200 {
            return Err(ProviderError::Status {
                provider: PROVIDER,
                status: response.status,
            });
        }
        let value: serde_json::Value =
            serde_json::from_str(&response.body).map_err(|e| decode(PROVIDER, e.to_string()))?;

        for key in ["Error Message", "Note", "Information"] {
            if let Some(message) = value.get(key).and_then(|v| v.as_str()) {
                return Err(reported(PROVIDER, message.to_owned()));
            }
        }
        let series = value
            .get("Time Series (Daily)")
            .and_then(|v| v.as_object())
            .ok_or_else(|| decode(PROVIDER, "response carried no daily series"))?;

        let mut candles: Vec<VenueCandle> = series
            .iter()
            .filter_map(|(date, row)| {
                let field = |name: &str| row.get(name)?.as_str().map(str::to_owned);
                Some(VenueCandle {
                    pair: symbol.to_uppercase(),
                    bar_start: time::Date::parse(
                        date,
                        time::macros::format_description!("[year]-[month]-[day]"),
                    )
                    .ok()?
                    .midnight()
                    .assume_utc(),
                    // Keys are numbered by the provider: "1. open" and so on.
                    open: field("1. open")?,
                    high: field("2. high")?,
                    low: field("3. low")?,
                    close: field("4. close")?,
                    volume: field("5. volume")?,
                    provider: ProviderId::ALPHA_VANTAGE,
                    retrieved_at,
                })
            })
            .collect();
        // The series arrives keyed by date string, so iteration order is
        // lexicographic rather than chronological. It happens to coincide for
        // ISO dates, but sorting explicitly means a format change cannot
        // silently reorder a price series.
        candles.sort_by_key(|candle| candle.bar_start);
        Ok(candles)
    }
}

#[async_trait]
impl Provider for AlphaVantageAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::ALPHA_VANTAGE
    }

    fn capabilities(&self) -> ProviderCapabilities {
        [Capability::Bars, Capability::Ohlcv].into_iter().collect()
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
// Polygon
// ---------------------------------------------------------------------------

/// Polygon aggregates adapter.
pub struct PolygonAdapter {
    transport: Arc<dyn HttpTransport>,
    api_key: String,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for PolygonAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolygonAdapter").finish_non_exhaustive()
    }
}

impl PolygonAdapter {
    /// Construct with an API key.
    pub fn new(transport: Arc<dyn HttpTransport>, api_key: impl Into<String>) -> Self {
        Self {
            transport,
            api_key: api_key.into(),
            entitlements: [Entitlement::Polygon].into_iter().collect(),
        }
    }

    /// The previous session's aggregate bar for one ticker.
    ///
    /// 401 and 403 are separated deliberately: the first means the key was
    /// rejected, the second means the key is fine and the plan does not cover
    /// the endpoint. Reporting both as "unauthorized" sends someone to
    /// regenerate a key that was never the problem.
    pub async fn previous_close(
        &self,
        ticker: &str,
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<VenueCandle>, ProviderError> {
        const PROVIDER: &str = "Polygon";
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: format!("/v2/aggs/ticker/{}/prev", ticker.to_uppercase()),
                query: BTreeMap::from([("apiKey".to_owned(), self.api_key.clone())]),
                headers: BTreeMap::new(),
                body: None,
            })
            .await?;
        match response.status {
            200 => {},
            401 => {
                return Err(reported(
                    PROVIDER,
                    "the API key was rejected (HTTP 401). Check the key itself.",
                ))
            },
            403 => {
                return Err(reported(
                    PROVIDER,
                    "the key is valid but the plan does not include this endpoint (HTTP 403). \
                     This is an entitlement, not a bad credential.",
                ))
            },
            429 => {
                return Err(reported(
                    PROVIDER,
                    "rate limited (HTTP 429). The free plan allows five calls a minute.",
                ))
            },
            status => {
                return Err(ProviderError::Status {
                    provider: PROVIDER,
                    status,
                })
            },
        }
        let value: serde_json::Value =
            serde_json::from_str(&response.body).map_err(|e| decode(PROVIDER, e.to_string()))?;
        let rows = value
            .get("results")
            .and_then(|v| v.as_array())
            .ok_or_else(|| decode(PROVIDER, "response carried no results array"))?;

        rows.iter()
            .map(|row| {
                let number = |name: &str| -> Result<String, ProviderError> {
                    row.get(name)
                        .and_then(|v| v.as_f64())
                        .map(|v| v.to_string())
                        .ok_or_else(|| decode(PROVIDER, format!("missing field {name}")))
                };
                // Polygon stamps in milliseconds.
                let millis = row
                    .get("t")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| decode(PROVIDER, "aggregate carries no timestamp"))?;
                Ok(VenueCandle {
                    pair: ticker.to_uppercase(),
                    bar_start: OffsetDateTime::from_unix_timestamp(millis / 1_000)
                        .map_err(|e| decode(PROVIDER, e.to_string()))?,
                    open: number("o")?,
                    high: number("h")?,
                    low: number("l")?,
                    close: number("c")?,
                    volume: number("v")?,
                    provider: ProviderId::POLYGON,
                    retrieved_at,
                })
            })
            .collect()
    }
}

#[async_trait]
impl Provider for PolygonAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::POLYGON
    }

    fn capabilities(&self) -> ProviderCapabilities {
        [Capability::Bars, Capability::Ohlcv].into_iter().collect()
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
// GDELT
// ---------------------------------------------------------------------------

/// One article from the GDELT document API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdeltArticle {
    /// Article URL.
    pub url: String,
    /// Headline as GDELT recorded it.
    pub title: String,
    /// Publishing domain.
    pub domain: String,
    /// Source language, as reported.
    pub language: String,
}

/// GDELT document-API adapter. Public, no credential.
pub struct GdeltAdapter {
    transport: Arc<dyn HttpTransport>,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for GdeltAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GdeltAdapter").finish_non_exhaustive()
    }
}

impl GdeltAdapter {
    /// Construct using an injected HTTP transport.
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            transport,
            entitlements: [Entitlement::PublicVenue].into_iter().collect(),
        }
    }

    /// Search recent coverage.
    ///
    /// GDELT throttles hard and answers 429 with no body, which json parsing
    /// would report as a malformed response. It is named instead: the caller
    /// should back off, not investigate a decoder.
    pub async fn search(&self, query: &str, max: u32) -> Result<Vec<GdeltArticle>, ProviderError> {
        const PROVIDER: &str = "GDELT";
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/api/v2/doc/doc".into(),
                query: BTreeMap::from([
                    ("query".to_owned(), query.to_owned()),
                    ("mode".to_owned(), "artlist".to_owned()),
                    ("format".to_owned(), "json".to_owned()),
                    ("maxrecords".to_owned(), max.to_string()),
                ]),
                headers: BTreeMap::new(),
                body: None,
            })
            .await?;
        if response.status == 429 {
            return Err(reported(
                PROVIDER,
                "rate limited (HTTP 429). GDELT throttles aggressively; retry shortly.",
            ));
        }
        if response.status != 200 {
            return Err(ProviderError::Status {
                provider: PROVIDER,
                status: response.status,
            });
        }
        let value: serde_json::Value =
            serde_json::from_str(&response.body).map_err(|e| decode(PROVIDER, e.to_string()))?;
        let rows = value
            .get("articles")
            .and_then(|v| v.as_array())
            .ok_or_else(|| decode(PROVIDER, "response carried no articles array"))?;

        Ok(rows
            .iter()
            .filter_map(|row| {
                let text = |name: &str| row.get(name)?.as_str().map(str::to_owned);
                Some(GdeltArticle {
                    url: text("url")?,
                    title: text("title").unwrap_or_default(),
                    domain: text("domain").unwrap_or_default(),
                    language: text("language").unwrap_or_default(),
                })
            })
            .collect())
    }
}

#[async_trait]
impl Provider for GdeltAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::GDELT
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
// Interactive Brokers
// ---------------------------------------------------------------------------

/// What the local IBKR gateway reports about its session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IbkrAuthStatus {
    /// The gateway holds an authenticated session.
    pub authenticated: bool,
    /// The gateway is connected to IBKR's servers.
    pub connected: bool,
    /// The session is live rather than merely present.
    pub competing: bool,
}

/// Interactive Brokers Client Portal Gateway adapter.
///
/// IBKR publishes no cloud REST API for this. The operator runs a Client
/// Portal Gateway on their own machine and authenticates it in a browser;
/// PRISMATIK then talks to `https://localhost:5000/v1/api`. Two consequences
/// worth stating plainly, because both look like PRISMATIK bugs otherwise:
///
/// - The gateway uses a self-signed certificate, so the transport must accept
///   it. That is safe only because the peer is loopback.
/// - A connection failure almost always means the gateway is not running, not
///   that a credential is wrong — there is no credential to get wrong here.
pub struct IbkrGatewayAdapter {
    transport: Arc<dyn HttpTransport>,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for IbkrGatewayAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IbkrGatewayAdapter").finish_non_exhaustive()
    }
}

impl IbkrGatewayAdapter {
    /// Construct using an injected HTTP transport pointed at the gateway.
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            transport,
            entitlements: [Entitlement::InteractiveBrokers].into_iter().collect(),
        }
    }

    /// Ask the gateway whether it holds a live session.
    pub async fn auth_status(&self) -> Result<IbkrAuthStatus, ProviderError> {
        const PROVIDER: &str = "Interactive Brokers";
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/v1/api/iserver/auth/status".into(),
                query: BTreeMap::new(),
                headers: BTreeMap::new(),
                body: None,
            })
            .await
            .map_err(|error| {
                // The most common outcome by far, and it is not a bug here.
                reported(
                    PROVIDER,
                    format!(
                        "could not reach the Client Portal Gateway ({error}). IBKR has no cloud \
                         API — start the gateway locally and sign in through its browser page, \
                         then retry."
                    ),
                )
            })?;
        if response.status == 401 {
            return Err(reported(
                PROVIDER,
                "the gateway is running but not signed in. Open its browser page and \
                 authenticate, then retry.",
            ));
        }
        if response.status != 200 {
            return Err(ProviderError::Status {
                provider: PROVIDER,
                status: response.status,
            });
        }
        let value: serde_json::Value =
            serde_json::from_str(&response.body).map_err(|e| decode(PROVIDER, e.to_string()))?;
        let flag = |name: &str| value.get(name).and_then(|v| v.as_bool()).unwrap_or(false);
        Ok(IbkrAuthStatus {
            authenticated: flag("authenticated"),
            connected: flag("connected"),
            competing: flag("competing"),
        })
    }
}

#[async_trait]
impl Provider for IbkrGatewayAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::INTERACTIVE_BROKERS
    }

    fn capabilities(&self) -> ProviderCapabilities {
        [Capability::Bars].into_iter().collect()
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
    async fn alpha_vantage_surfaces_its_200_level_failures() {
        // It answers 200 for a throttle, a rejected key and a bad symbol
        // alike, so trusting the status code would report every one of them
        // as an empty series.
        for (body, needle) in [
            (r#"{"Note":"call frequency limit"}"#, "frequency"),
            (r#"{"Information":"premium endpoint"}"#, "premium"),
            (r#"{"Error Message":"Invalid API call"}"#, "Invalid"),
        ] {
            let adapter = AlphaVantageAdapter::new(Arc::new(Canned(body, 200)), "k");
            let error = adapter
                .daily_bars("IBM", now())
                .await
                .expect_err("should surface the message");
            assert!(format!("{error}").contains(needle), "{error}");
        }
    }

    #[tokio::test]
    async fn alpha_vantage_returns_bars_in_chronological_order() {
        let adapter = AlphaVantageAdapter::new(
            Arc::new(Canned(
                r#"{"Time Series (Daily)":{
                     "2026-08-13":{"1. open":"2","2. high":"3","3. low":"1","4. close":"2.5","5. volume":"10"},
                     "2026-08-11":{"1. open":"1","2. high":"2","3. low":"0.5","4. close":"1.5","5. volume":"9"}}}"#,
                200,
            )),
            "k",
        );
        let bars = adapter.daily_bars("IBM", now()).await.expect("bars");
        assert_eq!(bars.len(), 2);
        assert!(
            bars[0].bar_start < bars[1].bar_start,
            "series must be chronological"
        );
    }

    #[tokio::test]
    async fn polygon_separates_a_bad_key_from_a_missing_entitlement() {
        // Collapsing these sends someone to regenerate a key that was fine.
        let rejected = PolygonAdapter::new(Arc::new(Canned("{}", 401)), "k")
            .previous_close("AAPL", now())
            .await
            .expect_err("401");
        assert!(
            format!("{rejected}").contains("key was rejected"),
            "{rejected}"
        );

        let entitlement = PolygonAdapter::new(Arc::new(Canned("{}", 403)), "k")
            .previous_close("AAPL", now())
            .await
            .expect_err("403");
        let text = format!("{entitlement}");
        assert!(text.contains("entitlement"), "{text}");
        assert!(text.contains("not a bad credential"), "{text}");
    }

    #[tokio::test]
    async fn gdelt_names_a_throttle_rather_than_a_decode_failure() {
        // 429 comes back with no body; parsing it would blame the decoder.
        let error = GdeltAdapter::new(Arc::new(Canned("", 429)))
            .search("markets", 3)
            .await
            .expect_err("429");
        assert!(format!("{error}").contains("rate limited"), "{error}");
    }

    #[tokio::test]
    async fn ibkr_explains_that_the_gateway_is_local() {
        let status = IbkrGatewayAdapter::new(Arc::new(Canned(
            r#"{"authenticated":true,"connected":true,"competing":false}"#,
            200,
        )))
        .auth_status()
        .await
        .expect("status");
        assert!(status.authenticated && status.connected);

        let unsigned = IbkrGatewayAdapter::new(Arc::new(Canned("", 401)))
            .auth_status()
            .await
            .expect_err("401");
        assert!(
            format!("{unsigned}").contains("not signed in"),
            "{unsigned}"
        );
    }
}
