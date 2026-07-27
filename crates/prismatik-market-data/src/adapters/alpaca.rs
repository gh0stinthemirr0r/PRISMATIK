//! Alpaca-shaped equity bars adapter for public/demo cassette replay.

use crate::http::{HttpMethod, HttpRequest, HttpTransport, TransportError};
use crate::provider::{
    Capability, Entitlement, EntitlementSet, Provider, ProviderCapabilities, ProviderHealth,
};
use crate::request::{CostUnits, ProviderRequest};
use crate::types::EquityBar;
use async_trait::async_trait;
use prismatik_domain::ProviderId;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;
use time::OffsetDateTime;

/// Alpaca market-data adapter errors.
#[derive(Debug, Error)]
pub enum AlpacaError {
    /// HTTP transport failed.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// Alpaca returned an unsuccessful response.
    #[error("Alpaca returned status {0}")]
    Status(u16),
    /// Response decoding failed.
    #[error("Alpaca decode: {0}")]
    Decode(String),
}

/// Read-only Alpaca-shaped equity bars adapter.
pub struct AlpacaAdapter {
    transport: Arc<dyn HttpTransport>,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for AlpacaAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlpacaAdapter").finish_non_exhaustive()
    }
}

impl AlpacaAdapter {
    /// Construct the public/demo adapter using an injected transport.
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            transport,
            entitlements: [Entitlement::AlpacaMarketData].into_iter().collect(),
        }
    }

    /// Fetch OHLCV bars for one symbol.
    pub async fn bars(
        &self,
        symbol: &str,
        timeframe: &str,
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<EquityBar>, AlpacaError> {
        let mut query = BTreeMap::new();
        query.insert("timeframe".into(), timeframe.into());
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: format!("/v2/stocks/{symbol}/bars"),
                query,
                headers: BTreeMap::new(),
            })
            .await?;
        if response.status != 200 {
            return Err(AlpacaError::Status(response.status));
        }
        let envelope: BarsEnvelope = serde_json::from_str(&response.body)
            .map_err(|error| AlpacaError::Decode(error.to_string()))?;
        envelope
            .bars
            .into_iter()
            .map(|row| {
                Ok(EquityBar {
                    symbol: symbol.to_uppercase(),
                    bar_start: OffsetDateTime::parse(
                        &row.t,
                        &time::format_description::well_known::Rfc3339,
                    )
                    .map_err(|error| AlpacaError::Decode(error.to_string()))?,
                    open: row.o.to_string(),
                    high: row.h.to_string(),
                    low: row.l.to_string(),
                    close: row.c.to_string(),
                    volume: row.v,
                    trade_count: row.n,
                    vwap: row.vw.map(|value| value.to_string()),
                    provider: ProviderId::ALPACA,
                    retrieved_at,
                })
            })
            .collect()
    }
}

#[async_trait]
impl Provider for AlpacaAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::ALPACA
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
struct BarsEnvelope {
    bars: Vec<AlpacaBar>,
}

#[derive(Deserialize)]
struct AlpacaBar {
    t: String,
    o: serde_json::Number,
    h: serde_json::Number,
    l: serde_json::Number,
    c: serde_json::Number,
    v: u64,
    n: Option<u64>,
    vw: Option<serde_json::Number>,
}

/// Built-in Wave 2 AAPL daily-bars cassette.
pub fn demo_cassette_json() -> &'static str {
    include_str!("../../cassettes/alpaca/demo.json")
}
