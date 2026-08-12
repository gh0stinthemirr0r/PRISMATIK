//! CFTC Commitments of Traders adapter.

use crate::http::{HttpMethod, HttpRequest, HttpTransport, TransportError};
use crate::provider::{
    Capability, Entitlement, EntitlementSet, Provider, ProviderCapabilities, ProviderHealth,
};
use crate::request::{CostUnits, ProviderRequest};
use crate::types::CotReport;
use async_trait::async_trait;
use prismatik_domain::ProviderId;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;
use time::{macros::format_description, Date, OffsetDateTime};

/// CFTC adapter errors.
#[derive(Debug, Error)]
pub enum CftcError {
    /// HTTP transport failed.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// CFTC returned an unsuccessful response.
    #[error("CFTC returned status {0}")]
    Status(u16),
    /// Response decoding failed.
    #[error("CFTC decode: {0}")]
    Decode(String),
}

/// Public CFTC weekly-report adapter.
pub struct CftcAdapter {
    transport: Arc<dyn HttpTransport>,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for CftcAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CftcAdapter").finish_non_exhaustive()
    }
}

impl CftcAdapter {
    /// Construct using an injected HTTP transport.
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            transport,
            entitlements: [Entitlement::CftcPublic].into_iter().collect(),
        }
    }

    /// Fetch the latest weekly report for a market code.
    pub async fn commitments(&self, market_code: &str) -> Result<CotReport, CftcError> {
        let mut query = BTreeMap::new();
        query.insert("market_code".into(), market_code.into());
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/api/v1/commitments".into(),
                query,
                headers: BTreeMap::new(),
                body: None,
            })
            .await?;
        if response.status != 200 {
            return Err(CftcError::Status(response.status));
        }
        let row: CftcRow = serde_json::from_str(&response.body)
            .map_err(|error| CftcError::Decode(error.to_string()))?;
        Ok(CotReport {
            market_code: row.market_code,
            market_name: row.market_name,
            as_of: parse_date(&row.as_of)?,
            published_at: OffsetDateTime::parse(
                &row.published_at,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|error| CftcError::Decode(error.to_string()))?,
            long_positions: row.long_positions,
            short_positions: row.short_positions,
            open_interest: row.open_interest,
        })
    }
}

#[async_trait]
impl Provider for CftcAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::CFTC
    }

    fn capabilities(&self) -> ProviderCapabilities {
        [Capability::CotReports].into_iter().collect()
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
struct CftcRow {
    market_code: String,
    market_name: String,
    as_of: String,
    published_at: String,
    long_positions: i64,
    short_positions: i64,
    open_interest: i64,
}

fn parse_date(value: &str) -> Result<Date, CftcError> {
    Date::parse(value, format_description!("[year]-[month]-[day]"))
        .map_err(|error| CftcError::Decode(error.to_string()))
}

/// Built-in Wave 2 CFTC cassette.
pub fn demo_cassette_json() -> &'static str {
    include_str!("../../cassettes/cftc/demo.json")
}
