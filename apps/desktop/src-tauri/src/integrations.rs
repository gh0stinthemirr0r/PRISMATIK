//! Governed, desktop-owned provider validation.
//!
//! A provider is exposed here only after it has a real Layer-2 adapter. Network
//! transport is injected from the application layer and every interactive
//! probe must first receive a BudgetGovernor permit.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, LazyLock, RwLock},
};

use prismatik_application::ReqwestTransport;
use prismatik_determinism::{Clock, SystemClock};
use prismatik_market_data::{
    adapters::{CoinGeckoAdapter, CoinGeckoAuth, FinnhubAdapter, FredAdapter, SecEdgarAdapter},
    AdmissionDecision, BudgetGovernor, GcraBudgetGovernor, PriorityClass,
};
use serde::Serialize;

#[derive(Clone)]
#[allow(
    dead_code,
    reason = "validated macro/filing sessions are consumed by the next scheduled-ingest slice"
)]
pub(crate) enum ActiveIntegration {
    CoinGecko { api_key: String },
    Finnhub { token: String },
    Fred { api_key: String },
    SecEdgar { contact: String },
}

static ACTIVE_INTEGRATIONS: LazyLock<RwLock<BTreeMap<String, ActiveIntegration>>> =
    LazyLock::new(|| RwLock::new(BTreeMap::new()));

/// Unofficial sources an operator has explicitly turned on.
///
/// Kept separate from `ACTIVE_INTEGRATIONS`, which holds credentialed provider
/// sessions. An unofficial source has no credential to hold — the only state
/// is consent — and conflating "I supplied a key" with "I accepted an
/// undocumented, unlicensed endpoint" would let the second happen as a side
/// effect of the first.
static UNOFFICIAL_SOURCES: LazyLock<RwLock<BTreeSet<String>>> =
    LazyLock::new(|| RwLock::new(BTreeSet::new()));

/// Whether an unofficial source is enabled. Defaults to off, and an
/// unreadable lock reads as off.
pub(crate) fn unofficial_source_enabled(source: &str) -> bool {
    UNOFFICIAL_SOURCES
        .read()
        .map(|set| set.contains(source))
        .unwrap_or(false)
}

/// One unofficial source and its current consent state.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UnofficialSource {
    pub(crate) id: String,
    pub(crate) label: String,
    /// What it does and what the operator is accepting by enabling it.
    pub(crate) caveat: String,
    pub(crate) enabled: bool,
}

/// The unofficial sources this build knows about.
fn unofficial_catalog() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![(
        crate::screener::SOURCE_ID,
        "TradingView screener",
        "Undocumented endpoint with no published API terms. Used for instrument discovery only:          its prices rank candidates and are never cited as observations. May change or stop          working without notice.",
    )]
}

/// List unofficial sources and whether each is enabled.
#[tauri::command]
pub(crate) fn list_unofficial_sources() -> Vec<UnofficialSource> {
    unofficial_catalog()
        .into_iter()
        .map(|(id, label, caveat)| UnofficialSource {
            id: id.to_owned(),
            label: label.to_owned(),
            caveat: caveat.to_owned(),
            enabled: unofficial_source_enabled(id),
        })
        .collect()
}

/// Turn an unofficial source on or off.
#[tauri::command]
pub(crate) fn set_unofficial_source(
    source: String,
    enabled: bool,
) -> Result<Vec<UnofficialSource>, String> {
    // Only sources this build declares can be enabled, so a caller cannot
    // invent an identifier and have it treated as consented.
    if !unofficial_catalog().iter().any(|(id, _, _)| *id == source) {
        return Err(format!("{source} is not a known unofficial source"));
    }
    let mut set = UNOFFICIAL_SOURCES
        .write()
        .map_err(|_| "unofficial source state unavailable")?;
    if enabled {
        set.insert(source);
    } else {
        set.remove(&source);
    }
    drop(set);
    Ok(list_unofficial_sources())
}

pub(crate) fn active(provider: &str) -> Option<ActiveIntegration> {
    ACTIVE_INTEGRATIONS.read().ok()?.get(provider).cloned()
}

fn activate(provider: &str, integration: ActiveIntegration) -> Result<(), String> {
    ACTIVE_INTEGRATIONS
        .write()
        .map_err(|_| "integration runtime lock is unavailable".to_owned())?
        .insert(provider.to_owned(), integration);
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationRuntimeStatus {
    provider_id: String,
    active: bool,
}

#[tauri::command]
pub(crate) fn integration_runtime_status() -> Vec<IntegrationRuntimeStatus> {
    let active = ACTIVE_INTEGRATIONS.read().ok();
    ["coingecko", "finnhub", "fred", "sec-edgar"]
        .into_iter()
        .map(|provider| IntegrationRuntimeStatus {
            provider_id: provider.to_owned(),
            active: active
                .as_ref()
                .is_some_and(|map| map.contains_key(provider)),
        })
        .collect()
}

#[tauri::command]
pub(crate) fn disconnect_integration(provider_id: String) -> Result<(), String> {
    ACTIVE_INTEGRATIONS
        .write()
        .map_err(|_| "integration runtime lock is unavailable".to_owned())?
        .remove(&provider_id);
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationTestResult {
    provider_id: String,
    status: &'static str,
    message: String,
    evidence: String,
}

fn required<'a>(
    credentials: &'a BTreeMap<String, String>,
    field: &str,
    provider: &str,
) -> Result<&'a str, String> {
    credentials
        .get(field)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{provider} requires {field}"))
}

static INTEGRATION_GOVERNORS: LazyLock<BTreeMap<&'static str, GcraBudgetGovernor>> =
    LazyLock::new(|| {
        ["coingecko", "fred", "sec-edgar", "finnhub"]
            .into_iter()
            .map(|provider| (provider, GcraBudgetGovernor::desktop_default()))
            .collect()
    });

fn admit_interactive(provider: &str) -> Result<(), String> {
    let governor = INTEGRATION_GOVERNORS
        .get(provider)
        .ok_or_else(|| "This provider does not have a governed desktop adapter.".to_owned())?;
    match governor.admit(PriorityClass::Interactive) {
        AdmissionDecision::Admit { .. } => Ok(()),
        AdmissionDecision::Defer { retry_at, .. } => Err(format!(
            "provider budget deferred this probe until {retry_at}"
        )),
        AdmissionDecision::BudgetExhausted { resets_at, .. } => {
            Err(format!("provider budget is exhausted until {resets_at}"))
        },
        AdmissionDecision::NotEntitled { required } => {
            Err(format!("missing provider entitlement: {}", required.0))
        },
    }
}

pub(crate) fn admit_background(provider: &str) -> Result<(), String> {
    let governor = INTEGRATION_GOVERNORS
        .get(provider)
        .ok_or_else(|| "This provider does not have a governed desktop adapter.".to_owned())?;
    match governor.admit(PriorityClass::Background) {
        AdmissionDecision::Admit { .. } => Ok(()),
        AdmissionDecision::Defer { retry_at, .. } => {
            Err(format!("provider budget deferred refresh until {retry_at}"))
        },
        AdmissionDecision::BudgetExhausted { resets_at, .. } => {
            Err(format!("provider budget exhausted until {resets_at}"))
        },
        AdmissionDecision::NotEntitled { required } => {
            Err(format!("missing provider entitlement: {}", required.0))
        },
    }
}

#[tauri::command]
pub(crate) async fn test_integration(
    provider_id: String,
    credentials: BTreeMap<String, String>,
) -> Result<IntegrationTestResult, String> {
    match provider_id.as_str() {
        "coingecko" => {
            admit_interactive("coingecko")?;
            let api_key = required(&credentials, "apiKey", "CoinGecko")?.to_owned();
            let transport = ReqwestTransport::new("https://api.coingecko.com/api/v3")
                .map_err(|error| format!("CoinGecko transport configuration failed: {error}"))?;
            let adapter = CoinGeckoAdapter::new(
                Arc::new(transport),
                CoinGeckoAuth::Demo {
                    api_key: api_key.clone(),
                },
            );
            let rows = adapter
                .trending()
                .await
                .map_err(|error| format!("CoinGecko validation failed: {error}"))?;
            activate("coingecko", ActiveIntegration::CoinGecko { api_key })?;
            Ok(IntegrationTestResult {
                provider_id,
                status: "connected",
                message: format!(
                    "CoinGecko authenticated; {} trending assets normalized.",
                    rows.len()
                ),
                evidence: "Provider adapter · BudgetGovernor permit · GET /search/trending"
                    .to_owned(),
            })
        },
        "fred" => {
            admit_interactive("fred")?;
            let api_key = required(&credentials, "apiKey", "FRED")?.to_owned();
            let transport = ReqwestTransport::new("https://api.stlouisfed.org/fred")
                .map_err(|error| format!("FRED transport configuration failed: {error}"))?;
            let adapter = FredAdapter::new(Arc::new(transport), api_key.clone());
            let rows = adapter
                .observations("DGS10")
                .await
                .map_err(|error| format!("FRED validation failed: {error}"))?;
            activate("fred", ActiveIntegration::Fred { api_key })?;
            Ok(IntegrationTestResult {
                provider_id,
                status: "connected",
                message: format!(
                    "FRED authenticated; {} vintage-aware DGS10 observations normalized.",
                    rows.len()
                ),
                evidence:
                    "Provider adapter · BudgetGovernor permit · GET /series/observations · DGS10"
                        .to_owned(),
            })
        },
        "sec-edgar" => {
            admit_interactive("sec-edgar")?;
            let contact = required(&credentials, "contactEmail", "SEC EDGAR")?.to_owned();
            let transport = ReqwestTransport::new("https://data.sec.gov")
                .map_err(|error| format!("SEC EDGAR transport configuration failed: {error}"))?;
            let adapter = SecEdgarAdapter::new(
                Arc::new(transport),
                format!("Mythos-PRISMATIK/0.1 {contact}"),
            )
            .map_err(|error| format!("SEC EDGAR identity validation failed: {error}"))?;
            let rows = adapter
                .submissions(320193)
                .await
                .map_err(|error| format!("SEC EDGAR validation failed: {error}"))?;
            activate("sec-edgar", ActiveIntegration::SecEdgar { contact })?;
            Ok(IntegrationTestResult {
                provider_id,
                status: "connected",
                message: format!(
                    "SEC EDGAR accepted the declared contact; {} Apple filing records normalized.",
                    rows.len()
                ),
                evidence:
                    "Provider adapter · BudgetGovernor permit · submissions/CIK0000320193.json"
                        .to_owned(),
            })
        },
        "finnhub" => {
            admit_interactive("finnhub")?;
            let token = required(&credentials, "apiKey", "Finnhub")?.to_owned();
            let transport = ReqwestTransport::new("https://finnhub.io/api/v1")
                .map_err(|error| format!("Finnhub transport configuration failed: {error}"))?;
            let adapter = FinnhubAdapter::new(Arc::new(transport), token.clone());
            let now = SystemClock::new().now();
            let to = now.unix_timestamp();
            let from = to - 7 * 86_400;
            let rows = adapter
                .bars("AAPL", "D", from, to, now)
                .await
                .map_err(|error| format!("Finnhub validation failed: {error}"))?;
            activate("finnhub", ActiveIntegration::Finnhub { token })?;
            Ok(IntegrationTestResult {
                provider_id,
                status: "connected",
                message: format!(
                    "Finnhub authenticated; {} AAPL daily bars normalized.",
                    rows.len()
                ),
                evidence: "Provider adapter · BudgetGovernor permit · GET /stock/candle · AAPL"
                    .to_owned(),
            })
        },
        _ => Err(
            "This provider is catalogued but does not yet have a governed desktop adapter."
                .to_owned(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::required;
    use std::collections::BTreeMap;

    #[test]
    fn required_credentials_fail_closed() {
        assert_eq!(
            required(&BTreeMap::new(), "apiKey", "Example"),
            Err("Example requires apiKey".to_owned())
        );
    }
}
