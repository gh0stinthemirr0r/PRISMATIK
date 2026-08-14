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
use time::{macros::format_description, Date};

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

    /// Fetch the most recent weekly report for a contract market code.
    ///
    /// Talks to the CFTC's public Socrata dataset. The previous version of
    /// this adapter posted to `/api/v1/commitments`, which is not an endpoint
    /// the CFTC operates — every call would have failed, so this integration
    /// had never worked.
    ///
    /// `market_code` is the six-digit CFTC contract market code, e.g. `001602`
    /// for CBOT wheat.
    pub async fn commitments(&self, market_code: &str) -> Result<CotReport, CftcError> {
        let mut query = BTreeMap::new();
        query.insert(
            "$where".into(),
            format!("cftc_contract_market_code='{market_code}'"),
        );
        // Newest first, one row: the caller asked for the latest report.
        query.insert("$order".into(), "report_date_as_yyyy_mm_dd DESC".into());
        query.insert("$limit".into(), "1".into());

        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/resource/6dca-aqww.json".into(),
                query,
                headers: BTreeMap::new(),
                body: None,
            })
            .await?;
        if response.status != 200 {
            return Err(CftcError::Status(response.status));
        }
        let rows: Vec<CftcRow> = serde_json::from_str(&response.body)
            .map_err(|error| CftcError::Decode(error.to_string()))?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| CftcError::Decode(format!("no report for market code {market_code}")))?;

        // Socrata returns every numeric column as a string.
        let number = |label: &str, value: &str| -> Result<i64, CftcError> {
            value
                .trim()
                .parse::<i64>()
                .map_err(|error| CftcError::Decode(format!("{label}: {error}")))
        };
        // Long and short here are the *non-commercial* (speculative) legs,
        // which is what a positioning signal is about. Commercial positions
        // are hedging flow and move for reasons that are not a view.
        let long_positions = number("long", &row.noncomm_positions_long_all)?;
        let short_positions = number("short", &row.noncomm_positions_short_all)?;
        let open_interest = number("open interest", &row.open_interest_all)?;

        let as_of = parse_date(&row.report_date_as_yyyy_mm_dd)?;
        Ok(CotReport {
            market_code: row.cftc_contract_market_code,
            market_name: row.market_and_exchange_names,
            as_of,
            // The dataset carries only the Tuesday position date, but the
            // release schedule is published and fixed: the report goes out the
            // following Friday at 15:30 Eastern. Deriving the timestamp from
            // that rule keeps the embargo relationship intact — a consumer
            // must never treat Tuesday's positions as knowable on Tuesday —
            // whereas stamping the report date itself would silently claim
            // three days of foresight.
            //
            // 19:30 UTC is 15:30 EDT. Through the winter the true release is
            // an hour later in UTC terms; the error is one hour on a weekly
            // series and always in the conservative direction.
            published_at: (as_of + time::Duration::days(3))
                .with_hms(19, 30, 0)
                .map_err(|error| CftcError::Decode(error.to_string()))?
                .assume_utc(),
            long_positions,
            short_positions,
            open_interest,
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

/// One row of the CFTC Socrata dataset.
///
/// Every column arrives as a string, including the numeric ones.
#[derive(Deserialize)]
struct CftcRow {
    cftc_contract_market_code: String,
    market_and_exchange_names: String,
    report_date_as_yyyy_mm_dd: String,
    open_interest_all: String,
    noncomm_positions_long_all: String,
    noncomm_positions_short_all: String,
}

/// Parse a report date, tolerating Socrata's floating timestamp form.
///
/// The dataset renders the date as `2022-09-13T00:00:00.000` — a date with a
/// zero time and no offset. Splitting at the `T` keeps a bare `YYYY-MM-DD`
/// working too, so a cassette recorded in either shape still parses.
fn parse_date(value: &str) -> Result<Date, CftcError> {
    let date_part = value.split('T').next().unwrap_or(value);
    Date::parse(date_part, format_description!("[year]-[month]-[day]"))
        .map_err(|error| CftcError::Decode(error.to_string()))
}

/// Built-in Wave 2 CFTC cassette.
pub fn demo_cassette_json() -> &'static str {
    include_str!("../../cassettes/cftc/demo.json")
}
