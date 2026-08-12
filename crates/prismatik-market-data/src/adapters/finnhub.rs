//! Finnhub-shaped equity candle adapter for public/demo cassette replay.
//!
//! Live HTTP stays in the application layer via injected [`HttpTransport`].
//! Finnhub is the OSS-friendly equity OHLCV fallback behind Alpaca licensing.

use crate::http::{HttpMethod, HttpRequest, HttpTransport, TransportError};
use crate::provider::{
    Capability, Entitlement, EntitlementSet, Provider, ProviderCapabilities, ProviderHealth,
};
use crate::request::{CostUnits, ProviderRequest};
use crate::types::{EquityBar, EquitySearchHit};
use async_trait::async_trait;
use prismatik_domain::ProviderId;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;
use time::OffsetDateTime;

/// Finnhub market-data adapter errors.
#[derive(Debug, Error)]
pub enum FinnhubError {
    /// HTTP transport failed.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// Finnhub returned an unsuccessful response.
    #[error("Finnhub returned status {0}")]
    Status(u16),
    /// Response decoding failed.
    #[error("Finnhub decode: {0}")]
    Decode(String),
}

/// Read-only Finnhub-shaped equity candle adapter.
pub struct FinnhubAdapter {
    transport: Arc<dyn HttpTransport>,
    token: String,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for FinnhubAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FinnhubAdapter")
            .field("token", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl FinnhubAdapter {
    /// Construct using an injected transport and Finnhub API token.
    pub fn new(transport: Arc<dyn HttpTransport>, token: impl Into<String>) -> Self {
        Self {
            transport,
            token: token.into(),
            entitlements: [Entitlement::Finnhub].into_iter().collect(),
        }
    }

    /// Fetch OHLCV candles for one symbol.
    ///
    /// `resolution` follows Finnhub (`D`, `1`, `5`, …). `from` / `to` are UNIX seconds.
    pub async fn bars(
        &self,
        symbol: &str,
        resolution: &str,
        from: i64,
        to: i64,
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<EquityBar>, FinnhubError> {
        let mut query = BTreeMap::new();
        query.insert("symbol".into(), symbol.to_uppercase());
        query.insert("resolution".into(), resolution.into());
        query.insert("from".into(), from.to_string());
        query.insert("to".into(), to.to_string());
        query.insert("token".into(), self.token.clone());
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/stock/candle".into(),
                query,
                headers: BTreeMap::new(),
                body: None,
            })
            .await?;
        if response.status != 200 {
            return Err(FinnhubError::Status(response.status));
        }
        let envelope: CandleEnvelope = serde_json::from_str(&response.body)
            .map_err(|error| FinnhubError::Decode(error.to_string()))?;
        if envelope.s == "no_data" {
            return Ok(Vec::new());
        }
        if envelope.s != "ok" {
            return Err(FinnhubError::Decode(format!(
                "unexpected status {}",
                envelope.s
            )));
        }
        let len = envelope.t.len();
        if envelope.o.len() != len
            || envelope.h.len() != len
            || envelope.l.len() != len
            || envelope.c.len() != len
            || envelope.v.len() != len
        {
            return Err(FinnhubError::Decode(
                "candle arrays have unequal lengths".into(),
            ));
        }
        let symbol = symbol.to_uppercase();
        (0..len)
            .map(|i| {
                Ok(EquityBar {
                    symbol: symbol.clone(),
                    bar_start: OffsetDateTime::from_unix_timestamp(envelope.t[i])
                        .map_err(|error| FinnhubError::Decode(error.to_string()))?,
                    open: envelope.o[i].to_string(),
                    high: envelope.h[i].to_string(),
                    low: envelope.l[i].to_string(),
                    close: envelope.c[i].to_string(),
                    volume: envelope.v[i],
                    trade_count: None,
                    vwap: None,
                    provider: ProviderId::FINNHUB,
                    retrieved_at,
                })
            })
            .collect()
    }

    /// Resolve free text to tradable equity symbols.
    ///
    /// Finnhub returns instrument types (`Common Stock`, `ETP`, …) rather than a
    /// listing venue, so that is what lands in [`EquitySearchHit::exchange`];
    /// it is the label a desk actually needs to disambiguate two same-named
    /// rows. Entries without a symbol are dropped — they cannot be tracked.
    pub async fn search(
        &self,
        query_text: &str,
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<EquitySearchHit>, FinnhubError> {
        let mut query = BTreeMap::new();
        query.insert("q".into(), query_text.to_owned());
        query.insert("token".into(), self.token.clone());
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/search".into(),
                query,
                headers: BTreeMap::new(),
                body: None,
            })
            .await?;
        if response.status != 200 {
            return Err(FinnhubError::Status(response.status));
        }
        let envelope: SearchEnvelope = serde_json::from_str(&response.body)
            .map_err(|error| FinnhubError::Decode(error.to_string()))?;
        Ok(envelope
            .result
            .into_iter()
            .filter(|row| !row.symbol.trim().is_empty())
            .map(|row| EquitySearchHit {
                symbol: row.symbol.to_uppercase(),
                description: row.description,
                exchange: row.kind,
                provider: ProviderId::FINNHUB,
                retrieved_at,
            })
            .collect())
    }
}

#[async_trait]
impl Provider for FinnhubAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::FINNHUB
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

#[derive(Deserialize)]
struct CandleEnvelope {
    s: String,
    #[serde(default)]
    o: Vec<serde_json::Number>,
    #[serde(default)]
    h: Vec<serde_json::Number>,
    #[serde(default)]
    l: Vec<serde_json::Number>,
    #[serde(default)]
    c: Vec<serde_json::Number>,
    #[serde(default)]
    v: Vec<u64>,
    #[serde(default)]
    t: Vec<i64>,
}

#[derive(Deserialize)]
struct SearchEnvelope {
    #[serde(default)]
    result: Vec<SearchRow>,
}

#[derive(Deserialize)]
struct SearchRow {
    #[serde(default)]
    symbol: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "type", default)]
    kind: String,
}

/// Built-in Wave 2 AAPL daily-candle cassette.
pub fn demo_cassette_json() -> &'static str {
    include_str!("../../cassettes/finnhub/demo.json")
}
