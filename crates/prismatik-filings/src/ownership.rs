//! Native parsers for SEC 13F holdings and Form 4 transactions.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{format_description::well_known::Iso8601, Date};

/// Normalized 13F holding observable from its public filing date.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Form13fHolding {
    /// Filing manager CIK.
    pub filer_cik: String,
    /// Reporting period end.
    pub period_end: Date,
    /// Public filing date.
    pub filing_date: Date,
    /// Earliest lawful point-in-time visibility.
    pub observable_at: Date,
    /// Issuer name.
    pub issuer_name: String,
    /// CUSIP retained exactly as filed.
    pub cusip: String,
    /// Share quantity retained without floating-point conversion.
    pub shares: String,
    /// Reported value retained without floating-point conversion.
    pub value_usd: String,
}

/// Normalized Form 4 non-derivative transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Form4Transaction {
    /// Issuer CIK.
    pub issuer_cik: String,
    /// Reporting owner name.
    pub owner_name: String,
    /// Security title.
    pub security_title: String,
    /// Transaction date.
    pub transaction_date: Date,
    /// Public filing date and observability hinge.
    pub filing_date: Date,
    /// SEC transaction code.
    pub transaction_code: String,
    /// Share quantity.
    pub shares: String,
    /// Price per share when reported.
    pub price_per_share: Option<String>,
    /// Acquired/disposed code.
    pub acquired_disposed_code: String,
}

/// SEC ownership parsing failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum FilingParseError {
    /// Required field was absent.
    #[error("missing filing field: {0}")]
    MissingField(&'static str),
    /// Date was malformed.
    #[error("invalid filing date in {0}")]
    InvalidDate(&'static str),
    /// JSON payload was malformed.
    #[error("invalid filing JSON: {0}")]
    InvalidJson(String),
    /// Payload format was not recognized.
    #[error("unsupported filing payload format")]
    UnsupportedFormat,
}

/// Parse a 13F information table from native JSON or SEC-style XML.
pub fn parse_13f(raw: &str) -> Result<Vec<Form13fHolding>, FilingParseError> {
    let trimmed = raw.trim_start();
    if trimmed.starts_with('{') {
        return parse_13f_json(trimmed);
    }
    if !trimmed.starts_with('<') {
        return Err(FilingParseError::UnsupportedFormat);
    }
    let filer_cik = tag(trimmed, "filerCik")?.to_owned();
    let period_end = parse_date(tag(trimmed, "periodEnd")?, "periodEnd")?;
    let filing_date = parse_date(tag(trimmed, "filingDate")?, "filingDate")?;
    let mut rows = Vec::new();
    for block in blocks(trimmed, "infoTable") {
        rows.push(Form13fHolding {
            filer_cik: filer_cik.clone(),
            period_end,
            filing_date,
            observable_at: filing_date,
            issuer_name: tag(block, "nameOfIssuer")?.to_owned(),
            cusip: tag(block, "cusip")?.to_owned(),
            shares: tag(block, "sshPrnamt")?.to_owned(),
            value_usd: tag(block, "value")?.to_owned(),
        });
    }
    Ok(rows)
}

#[derive(Deserialize)]
struct Json13f {
    filer_cik: serde_json::Value,
    period_end: String,
    filing_date: String,
    holdings: Vec<JsonHolding>,
}

#[derive(Deserialize)]
struct JsonHolding {
    issuer_name: String,
    cusip: String,
    shares: String,
    value_usd: String,
}

fn parse_13f_json(raw: &str) -> Result<Vec<Form13fHolding>, FilingParseError> {
    let filing: Json13f = serde_json::from_str(raw)
        .map_err(|error| FilingParseError::InvalidJson(error.to_string()))?;
    let period_end = parse_date(&filing.period_end, "period_end")?;
    let filing_date = parse_date(&filing.filing_date, "filing_date")?;
    let filer_cik = filing
        .filer_cik
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| filing.filer_cik.to_string());
    Ok(filing
        .holdings
        .into_iter()
        .map(|holding| Form13fHolding {
            filer_cik: filer_cik.clone(),
            period_end,
            filing_date,
            observable_at: filing_date,
            issuer_name: holding.issuer_name,
            cusip: holding.cusip,
            shares: holding.shares,
            value_usd: holding.value_usd,
        })
        .collect())
}

/// Parse SEC Form 4 non-derivative transactions from XML.
pub fn parse_form4(raw: &str) -> Result<Vec<Form4Transaction>, FilingParseError> {
    if !raw.trim_start().starts_with('<') {
        return Err(FilingParseError::UnsupportedFormat);
    }
    let issuer_cik = tag(raw, "issuerCik")?.to_owned();
    let owner_name = tag(raw, "rptOwnerName")?.to_owned();
    let filing_date = parse_date(tag(raw, "filingDate")?, "filingDate")?;
    let mut rows = Vec::new();
    for block in blocks(raw, "nonDerivativeTransaction") {
        rows.push(Form4Transaction {
            issuer_cik: issuer_cik.clone(),
            owner_name: owner_name.clone(),
            security_title: nested_value(block, "securityTitle")?.to_owned(),
            transaction_date: parse_date(
                nested_value(block, "transactionDate")?,
                "transactionDate",
            )?,
            filing_date,
            transaction_code: tag(block, "transactionCode")?.to_owned(),
            shares: nested_value(block, "transactionShares")?.to_owned(),
            price_per_share: optional_nested_value(block, "transactionPricePerShare")
                .map(str::to_owned),
            acquired_disposed_code: nested_value(block, "transactionAcquiredDisposedCode")?
                .to_owned(),
        });
    }
    Ok(rows)
}

/// Return only holdings publicly observable on or before `as_of`.
pub fn holdings_observable_as_of(holdings: &[Form13fHolding], as_of: Date) -> Vec<&Form13fHolding> {
    holdings
        .iter()
        .filter(|holding| holding.observable_at <= as_of)
        .collect()
}

fn parse_date(raw: &str, field: &'static str) -> Result<Date, FilingParseError> {
    Date::parse(raw.trim(), &Iso8601::DATE).map_err(|_| FilingParseError::InvalidDate(field))
}

fn tag<'a>(raw: &'a str, name: &'static str) -> Result<&'a str, FilingParseError> {
    optional_tag(raw, name).ok_or(FilingParseError::MissingField(name))
}

fn optional_tag<'a>(raw: &'a str, name: &str) -> Option<&'a str> {
    let start_marker = format!("<{name}>");
    let end_marker = format!("</{name}>");
    let start = raw.find(&start_marker)? + start_marker.len();
    let end = raw[start..].find(&end_marker)? + start;
    Some(raw[start..end].trim())
}

fn nested_value<'a>(raw: &'a str, parent: &'static str) -> Result<&'a str, FilingParseError> {
    let parent_block = tag(raw, parent)?;
    tag(parent_block, "value")
}

fn optional_nested_value<'a>(raw: &'a str, parent: &str) -> Option<&'a str> {
    optional_tag(raw, parent).and_then(|block| optional_tag(block, "value"))
}

fn blocks<'a>(raw: &'a str, name: &str) -> Vec<&'a str> {
    let start_marker = format!("<{name}>");
    let end_marker = format!("</{name}>");
    let mut remainder = raw;
    let mut output = Vec::new();
    while let Some(start_offset) = remainder.find(&start_marker) {
        let content_start = start_offset + start_marker.len();
        let Some(end_offset) = remainder[content_start..].find(&end_marker) else {
            break;
        };
        let content_end = content_start + end_offset;
        output.push(&remainder[content_start..content_end]);
        remainder = &remainder[content_end + end_marker.len()..];
    }
    output
}
