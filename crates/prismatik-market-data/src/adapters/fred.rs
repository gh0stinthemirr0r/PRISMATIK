//! FRED macroeconomic-series adapter with vintage metadata.

use crate::http::{HttpMethod, HttpRequest, HttpTransport, TransportError};
use crate::provider::{
    Capability, Entitlement, EntitlementSet, Provider, ProviderCapabilities, ProviderHealth,
};
use crate::request::{CostUnits, ProviderRequest};
use crate::types::MacroSeriesPoint;
use async_trait::async_trait;
use prismatik_domain::ProviderId;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;
use time::{macros::format_description, Date};

/// FRED adapter errors.
#[derive(Debug, Error)]
pub enum FredError {
    /// HTTP transport failed.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// FRED returned an unsuccessful response.
    #[error("FRED returned status {0}")]
    Status(u16),
    /// Response decoding failed.
    #[error("FRED decode: {0}")]
    Decode(String),
}

/// FRED series adapter.
pub struct FredAdapter {
    transport: Arc<dyn HttpTransport>,
    api_key: String,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for FredAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FredAdapter")
            .field("api_key", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl FredAdapter {
    /// Construct using an injected HTTP transport and FRED API key.
    pub fn new(transport: Arc<dyn HttpTransport>, api_key: impl Into<String>) -> Self {
        Self {
            transport,
            api_key: api_key.into(),
            entitlements: [Entitlement::Fred].into_iter().collect(),
        }
    }

    /// Fetch observations including their real-time vintage bounds.
    pub async fn observations(&self, series_id: &str) -> Result<Vec<MacroSeriesPoint>, FredError> {
        let mut query = BTreeMap::new();
        query.insert("series_id".into(), series_id.into());
        query.insert("api_key".into(), self.api_key.clone());
        query.insert("file_type".into(), "json".into());
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/series/observations".into(),
                query,
                headers: BTreeMap::new(),
                body: None,
            })
            .await?;
        if response.status != 200 {
            return Err(FredError::Status(response.status));
        }
        let envelope: FredEnvelope = serde_json::from_str(&response.body)
            .map_err(|error| FredError::Decode(error.to_string()))?;
        envelope
            .observations
            .into_iter()
            .map(|row| {
                let realtime_start = parse_date(&row.realtime_start)?;
                Ok(MacroSeriesPoint {
                    series_id: series_id.to_string(),
                    date: parse_date(&row.date)?,
                    value: (row.value != ".").then_some(row.value),
                    realtime_start,
                    realtime_end: row
                        .realtime_end
                        .filter(|value| value != "9999-12-31")
                        .map(|value| parse_date(&value))
                        .transpose()?,
                    available_at: realtime_start,
                })
            })
            .collect()
    }
}

#[async_trait]
impl Provider for FredAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::FRED
    }

    fn capabilities(&self) -> ProviderCapabilities {
        [Capability::MacroSeries].into_iter().collect()
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
struct FredEnvelope {
    observations: Vec<FredObservation>,
}

#[derive(Deserialize)]
struct FredObservation {
    realtime_start: String,
    realtime_end: Option<String>,
    date: String,
    value: String,
}

fn parse_date(value: &str) -> Result<Date, FredError> {
    Date::parse(value, format_description!("[year]-[month]-[day]"))
        .map_err(|error| FredError::Decode(error.to_string()))
}

/// Built-in Wave 2 FRED cassette.
pub fn demo_cassette_json() -> &'static str {
    include_str!("../../cassettes/fred/demo.json")
}
