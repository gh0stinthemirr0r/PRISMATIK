//! SEC EDGAR adapter. SEC fair-access rate (10 requests/second) is governed
//! externally through the provider's one-unit request cost.

use crate::http::{HttpMethod, HttpRequest, HttpTransport, TransportError};
use crate::provider::{
    Capability, Entitlement, EntitlementSet, Provider, ProviderCapabilities, ProviderHealth,
};
use crate::request::{CostUnits, ProviderRequest};
use crate::types::{CompanyFact, FilingSummary};
use async_trait::async_trait;
use prismatik_domain::ProviderId;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;
use time::{macros::format_description, Date};

/// SEC EDGAR adapter errors.
#[derive(Debug, Error)]
pub enum SecEdgarError {
    /// User-Agent does not satisfy the application policy.
    #[error("SEC EDGAR requires a Mythos-PRISMATIK/0.1 User-Agent with contact information")]
    InvalidUserAgent,
    /// HTTP transport failed.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// SEC returned an unsuccessful response.
    #[error("SEC EDGAR returned status {0}")]
    Status(u16),
    /// Response decoding failed.
    #[error("SEC EDGAR decode: {0}")]
    Decode(String),
}

/// Cassette/live SEC EDGAR provider adapter.
pub struct SecEdgarAdapter {
    transport: Arc<dyn HttpTransport>,
    user_agent: String,
    entitlements: EntitlementSet,
}

impl std::fmt::Debug for SecEdgarAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecEdgarAdapter")
            .field("user_agent", &self.user_agent)
            .field("entitlements", &self.entitlements)
            .finish_non_exhaustive()
    }
}

impl SecEdgarAdapter {
    /// Construct an adapter with the SEC-required declared User-Agent.
    pub fn new(
        transport: Arc<dyn HttpTransport>,
        user_agent: impl Into<String>,
    ) -> Result<Self, SecEdgarError> {
        let user_agent = user_agent.into();
        if !user_agent.starts_with("Mythos-PRISMATIK/0.1") || !user_agent.contains('@') {
            return Err(SecEdgarError::InvalidUserAgent);
        }
        Ok(Self {
            transport,
            user_agent,
            entitlements: [Entitlement::SecEdgar].into_iter().collect(),
        })
    }

    async fn get(
        &self,
        path: String,
        query: BTreeMap<String, String>,
    ) -> Result<String, SecEdgarError> {
        let mut headers = BTreeMap::new();
        headers.insert("user-agent".into(), self.user_agent.clone());
        // No Accept-Encoding header. Asking for gzip here broke every EDGAR
        // call: reqwest is built without its `gzip`/`deflate` features, and it
        // only decompresses responses whose Accept-Encoding *it* negotiated —
        // setting the header by hand opts out of that path entirely. SEC
        // honoured the request, returned compressed bytes, and every response
        // failed to parse at byte one with a JSON decode error that looked
        // like a malformed payload rather than a transport misconfiguration.
        let response = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path,
                query,
                headers,
                body: None,
            })
            .await?;
        if response.status != 200 {
            return Err(SecEdgarError::Status(response.status));
        }
        serde_json::from_str::<Value>(&response.body)
            .map_err(|e| SecEdgarError::Decode(e.to_string()))?;
        Ok(response.body)
    }

    /// Fetch recent submissions for a CIK.
    pub async fn submissions(&self, cik: u64) -> Result<Vec<FilingSummary>, SecEdgarError> {
        let body = self
            .get(format!("/submissions/CIK{cik:010}.json"), BTreeMap::new())
            .await?;
        let envelope: SubmissionsEnvelope =
            serde_json::from_str(&body).map_err(|e| SecEdgarError::Decode(e.to_string()))?;
        let recent = envelope.filings.recent;
        let mut rows = Vec::with_capacity(recent.form.len());
        for index in 0..recent.form.len() {
            rows.push(FilingSummary {
                cik: envelope.cik,
                accession_number: at(&recent.accession_number, index)?,
                form: at(&recent.form, index)?,
                filing_date: parse_date(&at(&recent.filing_date, index)?)?,
                report_date: recent
                    .report_date
                    .get(index)
                    .filter(|date| !date.is_empty())
                    .map(|date| parse_date(date))
                    .transpose()?,
                primary_document: recent.primary_document.get(index).cloned(),
            });
        }
        Ok(rows)
    }

    /// Fetch the supported subset of XBRL company facts for a CIK.
    pub async fn company_facts(&self, cik: u64) -> Result<Vec<CompanyFact>, SecEdgarError> {
        let body = self
            .get(
                format!("/api/xbrl/companyfacts/CIK{cik:010}.json"),
                BTreeMap::new(),
            )
            .await?;
        let root: Value =
            serde_json::from_str(&body).map_err(|e| SecEdgarError::Decode(e.to_string()))?;
        let actual_cik = root.get("cik").and_then(Value::as_u64).unwrap_or(cik);
        let mut output = Vec::new();
        let concepts = root
            .pointer("/facts/us-gaap")
            .and_then(Value::as_object)
            .ok_or_else(|| SecEdgarError::Decode("missing facts.us-gaap".into()))?;
        for (concept, fact) in concepts {
            let Some(units) = fact.get("units").and_then(Value::as_object) else {
                continue;
            };
            for (unit, observations) in units {
                let Some(observations) = observations.as_array() else {
                    continue;
                };
                for observation in observations {
                    let end = required_str(observation, "end")?;
                    let filed = required_str(observation, "filed")?;
                    output.push(CompanyFact {
                        cik: actual_cik,
                        concept: concept.clone(),
                        unit: unit.clone(),
                        value: json_number_string(
                            observation
                                .get("val")
                                .ok_or_else(|| SecEdgarError::Decode("missing val".into()))?,
                        ),
                        period_start: observation
                            .get("start")
                            .and_then(Value::as_str)
                            .map(parse_date)
                            .transpose()?,
                        period_end: parse_date(end)?,
                        filing_date: parse_date(filed)?,
                        form: required_str(observation, "form")?.to_string(),
                    });
                }
            }
        }
        Ok(output)
    }

    /// Search the filing index by company query and form type.
    pub async fn filing_index(
        &self,
        company: &str,
        form: &str,
    ) -> Result<Vec<FilingSummary>, SecEdgarError> {
        let mut query = BTreeMap::new();
        query.insert("action".into(), "getcompany".into());
        query.insert("company".into(), company.into());
        query.insert("type".into(), form.into());
        query.insert("output".into(), "json".into());
        let body = self.get("/cgi-bin/browse-edgar".into(), query).await?;
        serde_json::from_str(&body).map_err(|e| SecEdgarError::Decode(e.to_string()))
    }
}

#[async_trait]
impl Provider for SecEdgarAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::SEC_EDGAR
    }

    fn capabilities(&self) -> ProviderCapabilities {
        [
            Capability::Filings,
            Capability::FilingIndex,
            Capability::CompanyFacts,
        ]
        .into_iter()
        .collect()
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
struct SubmissionsEnvelope {
    /// EDGAR sends the CIK as a zero-padded *string* ("0000320193"), not a
    /// number. Typing it as `u64` made every submissions call fail to
    /// deserialise at the first field, long before any filing was read.
    #[serde(deserialize_with = "cik_from_string")]
    cik: u64,
    filings: Filings,
}

/// Accept EDGAR's zero-padded string CIK, and a bare number if it ever sends
/// one, so a format change in either direction does not break the adapter.
fn cik_from_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;
    match Value::deserialize(deserializer)? {
        Value::String(text) => text.trim_start_matches('0').parse::<u64>().or_else(|_| {
            // "0000000000" trims to empty, which is a real CIK of zero rather
            // than a parse failure.
            if text.chars().all(|c| c == '0') {
                Ok(0)
            } else {
                Err(D::Error::custom(format!("unparsable CIK {text:?}")))
            }
        }),
        Value::Number(number) => number
            .as_u64()
            .ok_or_else(|| D::Error::custom("CIK is not a non-negative integer")),
        other => Err(D::Error::custom(format!(
            "CIK should be a string or number, got {other}"
        ))),
    }
}

#[derive(Deserialize)]
struct Filings {
    recent: RecentFilings,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecentFilings {
    accession_number: Vec<String>,
    filing_date: Vec<String>,
    report_date: Vec<String>,
    form: Vec<String>,
    primary_document: Vec<String>,
}

fn at(values: &[String], index: usize) -> Result<String, SecEdgarError> {
    values
        .get(index)
        .cloned()
        .ok_or_else(|| SecEdgarError::Decode("misaligned submissions arrays".into()))
}

fn parse_date(value: &str) -> Result<Date, SecEdgarError> {
    Date::parse(value, format_description!("[year]-[month]-[day]"))
        .map_err(|e| SecEdgarError::Decode(e.to_string()))
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, SecEdgarError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| SecEdgarError::Decode(format!("missing {key}")))
}

fn json_number_string(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

/// Built-in Wave 2 SEC cassette.
pub fn demo_cassette_json() -> &'static str {
    include_str!("../../cassettes/sec_edgar/demo.json")
}
