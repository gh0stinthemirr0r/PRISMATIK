//! Unusual Whales flow adapter.

use crate::http::{HttpMethod, HttpRequest, HttpTransport, TransportError};
use prismatik_identity::AssetId;
use prismatik_options::{FlowSide, OccSymbolError, OptionContract, OptionsFlowPrint};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// Unusual Whales adapter failures.
#[derive(Debug, Error)]
pub enum UnusualWhalesError {
    /// HTTP transport failure.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// Provider rejected credentials.
    #[error("Unusual Whales authentication failure")]
    Auth,
    /// Provider rate limit was exceeded.
    #[error("Unusual Whales rate limited")]
    RateLimited,
    /// Provider returned an unsuccessful status.
    #[error("Unusual Whales upstream status {0}")]
    Upstream(u16),
    /// Provider returned no flow rows.
    #[error("Unusual Whales returned no flow prints")]
    Empty,
    /// Response body or timestamp could not be decoded.
    #[error("Unusual Whales decode: {0}")]
    Decode(String),
    /// Provider option symbol was invalid.
    #[error(transparent)]
    InvalidContract(#[from] OccSymbolError),
}

/// HTTP adapter that normalizes Unusual Whales option flow.
pub struct UnusualWhalesAdapter {
    transport: Arc<dyn HttpTransport>,
    api_token: String,
}

impl std::fmt::Debug for UnusualWhalesAdapter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UnusualWhalesAdapter")
            .field("api_token", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl UnusualWhalesAdapter {
    /// Construct an adapter over a live or cassette HTTP transport.
    pub fn new(transport: Arc<dyn HttpTransport>, api_token: impl Into<String>) -> Self {
        Self {
            transport,
            api_token: api_token.into(),
        }
    }

    /// Fetch and normalize option flow alerts for one underlying ticker.
    pub async fn flow_alerts(
        &self,
        underlying: AssetId,
        ticker: &str,
    ) -> Result<Vec<OptionsFlowPrint>, UnusualWhalesError> {
        let mut query = BTreeMap::new();
        query.insert("ticker".into(), ticker.to_owned());
        let mut headers = BTreeMap::new();
        headers.insert("authorization".into(), format!("Bearer {}", self.api_token));
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/api/option-trades/flow-alerts".into(),
                query,
                headers,
                body: None,
            })
            .await?;
        match response.status {
            200 => {},
            401 | 403 => return Err(UnusualWhalesError::Auth),
            429 => return Err(UnusualWhalesError::RateLimited),
            status => return Err(UnusualWhalesError::Upstream(status)),
        }
        let envelope: FlowEnvelope = serde_json::from_str(&response.body)
            .map_err(|error| UnusualWhalesError::Decode(error.to_string()))?;
        if envelope.data.is_empty() {
            return Err(UnusualWhalesError::Empty);
        }
        envelope
            .data
            .into_iter()
            .map(|row| row.normalize(underlying))
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct FlowEnvelope {
    data: Vec<RawFlowPrint>,
}

#[derive(Debug, Deserialize)]
struct RawFlowPrint {
    option_symbol: String,
    executed_at: String,
    size: u64,
    premium_cents: u64,
    side: String,
    open_interest: Option<u64>,
    volume: Option<u64>,
    opening: Option<bool>,
}

impl RawFlowPrint {
    fn normalize(self, underlying: AssetId) -> Result<OptionsFlowPrint, UnusualWhalesError> {
        let executed_at = OffsetDateTime::parse(&self.executed_at, &Rfc3339)
            .map_err(|error| UnusualWhalesError::Decode(error.to_string()))?;
        let side = match self.side.to_ascii_lowercase().as_str() {
            "ask" => FlowSide::Ask,
            "bid" => FlowSide::Bid,
            "mid" => FlowSide::Mid,
            _ => FlowSide::Unknown,
        };
        Ok(OptionsFlowPrint {
            contract: OptionContract::from_occ_symbol(underlying, &self.option_symbol)?,
            executed_at,
            quantity: self.size,
            premium_cents: self.premium_cents,
            side,
            open_interest: self.open_interest,
            volume: self.volume,
            is_opening: self.opening,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CassetteTransport;
    use prismatik_identity::AssetId;
    use prismatik_options::{FlowSide, OptionRight};
    use std::sync::Arc;

    #[tokio::test]
    async fn cassette_flow_normalizes_to_domain_print() {
        let cassette = include_str!("../../cassettes/unusual_whales/flow_alerts.json");
        let transport = Arc::new(CassetteTransport::from_json(cassette).unwrap());
        let adapter = UnusualWhalesAdapter::new(transport, "test-token");
        let underlying = AssetId::from_canonical_bytes(b"AAPL");

        let prints = adapter.flow_alerts(underlying, "AAPL").await.unwrap();

        assert_eq!(prints.len(), 1);
        assert_eq!(prints[0].contract.underlying, underlying);
        assert_eq!(prints[0].contract.right, OptionRight::Call);
        assert_eq!(prints[0].contract.strike_millis, 150_000);
        assert_eq!(prints[0].side, FlowSide::Ask);
        assert_eq!(prints[0].premium_cents, 1_000_000);
    }
}
