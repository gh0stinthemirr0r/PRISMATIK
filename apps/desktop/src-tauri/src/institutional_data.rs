//! Read-only institutional and macro snapshots from explicitly active providers.

use std::sync::Arc;

use prismatik_application::IntelligenceObservationKind;
use prismatik_application::ReqwestTransport;
use prismatik_determinism::{Clock, SystemClock};
use prismatik_market_data::adapters::{FredAdapter, SecEdgarAdapter};
use serde::Serialize;
use time::{PrimitiveDateTime, Time};

use crate::evidence_store::{EvidenceRecord, PersistenceReport};
use crate::integrations::{self, ActiveIntegration};

const MAX_SERIES: usize = 12;
const MAX_POINTS_PER_SERIES: usize = 120;
const MAX_FILINGS: usize = 200;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MacroPointView {
    series_id: String,
    date: String,
    value: Option<String>,
    realtime_start: String,
    realtime_end: Option<String>,
    available_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MacroSnapshot {
    provider: &'static str,
    retrieved_at: String,
    series: Vec<String>,
    points: Vec<MacroPointView>,
    evidence: &'static str,
    persistence: PersistenceReport,
}

fn validate_series_id(value: &str) -> Result<String, String> {
    let value = value.trim().to_ascii_uppercase();
    if value.is_empty()
        || value.len() > 48
        || !value.bytes().all(|byte| {
            byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
    {
        return Err(format!("invalid FRED series identifier: {value}"));
    }
    Ok(value)
}

/// Fetch recent, vintage-aware observations for a bounded set of FRED series.
#[tauri::command]
pub(crate) async fn get_macro_series(series_ids: Vec<String>) -> Result<MacroSnapshot, String> {
    if series_ids.is_empty() || series_ids.len() > MAX_SERIES {
        return Err(format!("select between 1 and {MAX_SERIES} FRED series"));
    }
    let ActiveIntegration::Fred { api_key } = integrations::active("fred")
        .ok_or("FRED is not connected. Configure it in Settings > Integrations.")?
    else {
        return Err("FRED session state is invalid".to_owned());
    };
    let transport = ReqwestTransport::new("https://api.stlouisfed.org/fred")
        .map_err(|error| format!("FRED transport configuration failed: {error}"))?;
    let adapter = FredAdapter::new(Arc::new(transport), api_key);
    let mut series = Vec::new();
    let mut points = Vec::new();
    let mut evidence_records = Vec::new();
    for requested in series_ids {
        let series_id = validate_series_id(&requested)?;
        integrations::admit_background("fred")?;
        let rows = adapter
            .observations(&series_id)
            .await
            .map_err(|error| format!("FRED {series_id} retrieval failed: {error}"))?;
        let start = rows.len().saturating_sub(MAX_POINTS_PER_SERIES);
        for row in rows.into_iter().skip(start) {
            evidence_records.push(EvidenceRecord {
                source_uri: format!(
                    "https://api.stlouisfed.org/fred/series/observations?series_id={series_id}"
                ),
                publisher_record_id: format!(
                    "{}:{}:{}",
                    row.series_id, row.date, row.realtime_start
                ),
                event_time: Some(PrimitiveDateTime::new(row.date, Time::MIDNIGHT).assume_utc()),
                publication_time: Some(
                    PrimitiveDateTime::new(row.available_at, Time::MIDNIGHT).assume_utc(),
                ),
                normalized_payload: serde_json::to_vec(&row).map_err(|error| error.to_string())?,
            });
            points.push(MacroPointView {
                series_id: row.series_id,
                date: row.date.to_string(),
                value: row.value,
                realtime_start: row.realtime_start.to_string(),
                realtime_end: row.realtime_end.map(|date| date.to_string()),
                available_at: row.available_at.to_string(),
            });
        }
        series.push(series_id);
    }
    let persistence = crate::evidence_store::persist(
        "fred",
        evidence_records,
        IntelligenceObservationKind::MacroSeries,
    )
    .unwrap_or_else(PersistenceReport::failure);
    Ok(MacroSnapshot {
        provider: "FRED",
        retrieved_at: SystemClock::new().now().to_string(),
        series,
        points,
        evidence: "FRED /series/observations · vintage bounds preserved · no fixture fallback",
        persistence,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FilingView {
    cik: u64,
    accession_number: String,
    form: String,
    filing_date: String,
    report_date: Option<String>,
    primary_document: Option<String>,
    source_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FilingSnapshot {
    provider: &'static str,
    retrieved_at: String,
    cik: u64,
    filings: Vec<FilingView>,
    evidence: &'static str,
    persistence: PersistenceReport,
}

/// Fetch recent SEC submissions for one company using the declared contact identity.
#[tauri::command]
pub(crate) async fn get_filing_summaries(
    cik: u64,
    forms: Vec<String>,
) -> Result<FilingSnapshot, String> {
    if cik == 0 || cik > 9_999_999_999 {
        return Err("CIK must be a positive number of at most 10 digits".to_owned());
    }
    if forms.len() > 20 {
        return Err("at most 20 filing-form filters are allowed".to_owned());
    }
    let ActiveIntegration::SecEdgar { contact } = integrations::active("sec-edgar")
        .ok_or("SEC EDGAR is not connected. Configure it in Settings > Integrations.")?
    else {
        return Err("SEC EDGAR session state is invalid".to_owned());
    };
    integrations::admit_background("sec-edgar")?;
    let transport = ReqwestTransport::new("https://data.sec.gov")
        .map_err(|error| format!("SEC EDGAR transport configuration failed: {error}"))?;
    let adapter = SecEdgarAdapter::new(
        Arc::new(transport),
        format!("Mythos-PRISMATIK/0.1 {contact}"),
    )
    .map_err(|error| format!("SEC EDGAR identity validation failed: {error}"))?;
    let filters: Vec<String> = forms
        .into_iter()
        .map(|form| form.trim().to_ascii_uppercase())
        .filter(|form| !form.is_empty())
        .collect();
    let rows = adapter
        .submissions(cik)
        .await
        .map_err(|error| format!("SEC EDGAR retrieval failed: {error}"))?;
    let mut evidence_records = Vec::new();
    let filings = rows
        .into_iter()
        .filter(|row| filters.is_empty() || filters.contains(&row.form.to_ascii_uppercase()))
        .take(MAX_FILINGS)
        .map(|row| {
            let normalized_payload = serde_json::to_vec(&row).map_err(|error| error.to_string())?;
            let accession_path = row.accession_number.replace('-', "");
            let source_url = row.primary_document.as_ref().map_or_else(
                || format!("https://www.sec.gov/Archives/edgar/data/{cik}/{accession_path}/"),
                |document| {
                    format!(
                        "https://www.sec.gov/Archives/edgar/data/{cik}/{accession_path}/{document}"
                    )
                },
            );
            evidence_records.push(EvidenceRecord {
                source_uri: source_url.clone(),
                publisher_record_id: row.accession_number.clone(),
                event_time: Some(
                    PrimitiveDateTime::new(row.filing_date, Time::MIDNIGHT).assume_utc(),
                ),
                publication_time: Some(
                    PrimitiveDateTime::new(row.filing_date, Time::MIDNIGHT).assume_utc(),
                ),
                normalized_payload,
            });
            Ok(FilingView {
                cik: row.cik,
                accession_number: row.accession_number,
                form: row.form,
                filing_date: row.filing_date.to_string(),
                report_date: row.report_date.map(|date| date.to_string()),
                primary_document: row.primary_document,
                source_url,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let persistence = crate::evidence_store::persist(
        "sec-edgar",
        evidence_records,
        IntelligenceObservationKind::Filings,
    )
    .unwrap_or_else(PersistenceReport::failure);
    Ok(FilingSnapshot {
        provider: "SEC EDGAR",
        retrieved_at: SystemClock::new().now().to_string(),
        cik,
        filings,
        evidence: "SEC submissions API · declared User-Agent · accession-level source links · no fixture fallback",
        persistence,
    })
}

#[cfg(test)]
mod tests {
    use super::validate_series_id;

    #[test]
    fn series_ids_are_canonical_and_bounded() {
        assert_eq!(validate_series_id(" dgs10 ").unwrap(), "DGS10");
        assert!(validate_series_id("../secret").is_err());
        assert!(validate_series_id("").is_err());
    }
}
