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
    adapters::{
        AlpacaAdapter, AlpacaCredentials, CftcAdapter, CoinGeckoAdapter, CoinGeckoAuth,
        CoinbaseAdapter, FinnhubAdapter, FredAdapter, KalshiAdapter, KrakenAdapter,
        PolymarketAdapter, SecEdgarAdapter,
    },
    AdmissionDecision, BudgetGovernor, GcraBudgetGovernor, PriorityClass,
};
use serde::Serialize;

#[derive(Clone)]
#[allow(
    dead_code,
    reason = "validated macro/filing sessions are consumed by the next scheduled-ingest slice"
)]
pub(crate) enum ActiveIntegration {
    CoinGecko {
        api_key: String,
    },
    Finnhub {
        token: String,
    },
    Fred {
        api_key: String,
    },
    SecEdgar {
        contact: String,
    },
    Alpaca {
        key_id: String,
        secret_key: String,
    },
    /// Public data. Connected state is consent to poll, not a credential.
    Cftc,
    /// Public venue. As with CFTC, there is no secret to hold.
    Kraken,
    /// Public venue.
    Coinbase,
    /// Public prediction venue.
    Polymarket,
    /// Public prediction venue.
    Kalshi,
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

/// Providers with a real, credentialed adapter behind them.
///
/// One list, because this is exactly what drifted: the frontend catalog
/// carried its own hand-maintained readiness flag, so a provider could look
/// connectable while `test_integration` had no arm for it. The user walked a
/// three-step wizard and hit "catalogued but does not yet have a governed
/// desktop adapter" at the end. The catalog now asks the backend instead of
/// asserting, and a test below keeps this list honest against the match arms.
pub(crate) const LIVE_ADAPTERS: [&str; 10] = [
    "coingecko",
    "fred",
    "sec-edgar",
    "finnhub",
    "alpaca",
    "cftc",
    "kraken",
    "coinbase",
    "polymarket",
    "kalshi",
];

/// The providers this build can actually connect.
///
/// The frontend uses this to decide which catalog entries offer a credential
/// form, so the two can no longer disagree.
#[tauri::command]
pub(crate) fn list_live_adapters() -> Vec<String> {
    LIVE_ADAPTERS.iter().map(|id| (*id).to_owned()).collect()
}

static INTEGRATION_GOVERNORS: LazyLock<BTreeMap<&'static str, GcraBudgetGovernor>> =
    LazyLock::new(|| {
        LIVE_ADAPTERS
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
        "alpaca" => {
            admit_interactive("alpaca")?;
            let key_id = required(&credentials, "apiKey", "Alpaca")?.to_owned();
            let secret_key = required(&credentials, "apiSecret", "Alpaca")?.to_owned();
            // Market data lives on its own host, separate from the trading
            // API. Validating against data rather than the account endpoint
            // proves the entitlement the desk actually needs.
            let transport = ReqwestTransport::new("https://data.alpaca.markets")
                .map_err(|error| format!("Alpaca transport configuration failed: {error}"))?;
            let adapter = AlpacaAdapter::with_credentials(
                Arc::new(transport),
                AlpacaCredentials {
                    key_id: key_id.clone(),
                    secret_key: secret_key.clone(),
                },
            );
            let rows = adapter
                .bars("AAPL", "1Day", SystemClock::new().now())
                .await
                .map_err(|error| format!("Alpaca validation failed: {error}"))?;
            if rows.is_empty() {
                return Err(
                    "Alpaca authenticated but returned no bars. The key is valid; the account \
                     may lack a market-data entitlement for the IEX feed."
                        .to_owned(),
                );
            }
            activate("alpaca", ActiveIntegration::Alpaca { key_id, secret_key })?;
            Ok(IntegrationTestResult {
                provider_id,
                status: "connected",
                message: format!(
                    "Alpaca authenticated; {} AAPL daily bars normalized.",
                    rows.len()
                ),
                evidence:
                    "Provider adapter · BudgetGovernor permit · GET /v2/stocks/AAPL/bars · IEX feed"
                        .to_owned(),
            })
        },
        "cftc" => {
            admit_interactive("cftc")?;
            // No credential: the Commitments of Traders dataset is public.
            // "Connected" here records that the operator turned the poll on.
            let transport = ReqwestTransport::new("https://publicreporting.cftc.gov")
                .map_err(|error| format!("CFTC transport configuration failed: {error}"))?;
            let adapter = CftcAdapter::new(Arc::new(transport));
            // CBOT wheat: continuous weekly history, so a failure here is the
            // dataset or the network rather than a thin contract.
            let report = adapter
                .commitments("001602")
                .await
                .map_err(|error| format!("CFTC validation failed: {error}"))?;
            activate("cftc", ActiveIntegration::Cftc)?;
            Ok(IntegrationTestResult {
                provider_id,
                status: "connected",
                message: format!(
                    "CFTC public dataset reachable; {} as of {} normalized ({} long / {} short).",
                    report.market_name, report.as_of, report.long_positions, report.short_positions,
                ),
                evidence:
                    "Provider adapter · BudgetGovernor permit · Socrata 6dca-aqww · CBOT wheat"
                        .to_owned(),
            })
        },
        "kraken" => {
            admit_interactive("kraken")?;
            let transport = ReqwestTransport::new("https://api.kraken.com")
                .map_err(|error| format!("Kraken transport configuration failed: {error}"))?;
            let adapter = KrakenAdapter::new(Arc::new(transport));
            let candles = adapter
                .candles("XBTUSD", 1_440, SystemClock::new().now())
                .await
                .map_err(|error| format!("Kraken validation failed: {error}"))?;
            activate("kraken", ActiveIntegration::Kraken)?;
            Ok(IntegrationTestResult {
                provider_id,
                status: "connected",
                message: format!(
                    "Kraken public API reachable; {} XBTUSD daily candles normalized.",
                    candles.len()
                ),
                evidence: "Provider adapter · BudgetGovernor permit · GET /0/public/OHLC · XBTUSD"
                    .to_owned(),
            })
        },
        "coinbase" => {
            admit_interactive("coinbase")?;
            let transport = ReqwestTransport::new("https://api.exchange.coinbase.com")
                .map_err(|error| format!("Coinbase transport configuration failed: {error}"))?;
            let adapter = CoinbaseAdapter::new(Arc::new(transport));
            let candles = adapter
                .candles("BTC-USD", 86_400, SystemClock::new().now())
                .await
                .map_err(|error| format!("Coinbase validation failed: {error}"))?;
            activate("coinbase", ActiveIntegration::Coinbase)?;
            Ok(IntegrationTestResult {
                provider_id,
                status: "connected",
                message: format!(
                    "Coinbase Exchange public API reachable; {} BTC-USD daily candles normalized.",
                    candles.len()
                ),
                evidence:
                    "Provider adapter · BudgetGovernor permit · GET /products/BTC-USD/candles"
                        .to_owned(),
            })
        },
        "polymarket" => {
            admit_interactive("polymarket")?;
            let transport = ReqwestTransport::new("https://gamma-api.polymarket.com")
                .map_err(|error| format!("Polymarket transport configuration failed: {error}"))?;
            let adapter = PolymarketAdapter::new(Arc::new(transport));
            let markets = adapter
                .open_markets(20, SystemClock::new().now())
                .await
                .map_err(|error| format!("Polymarket validation failed: {error}"))?;
            let quoted = markets.iter().filter(|m| m.yes_price.is_some()).count();
            activate("polymarket", ActiveIntegration::Polymarket)?;
            Ok(IntegrationTestResult {
                provider_id,
                status: "connected",
                message: format!(
                    "Polymarket public API reachable; {} open markets normalized, {quoted} carrying a quote.",
                    markets.len()
                ),
                evidence: "Provider adapter · BudgetGovernor permit · GET /markets".to_owned(),
            })
        },
        "kalshi" => {
            admit_interactive("kalshi")?;
            let transport = ReqwestTransport::new("https://api.elections.kalshi.com")
                .map_err(|error| format!("Kalshi transport configuration failed: {error}"))?;
            let adapter = KalshiAdapter::new(Arc::new(transport));
            // Validated against a named series rather than the open listing.
            // That listing is almost entirely auto-generated sports parlays
            // with no bid on either side — a thousand-row sweep returned not
            // one quoted market — so it cannot show a working connection.
            // The Fed decision series is stable and continuously quoted.
            let markets = adapter
                .markets_in_series("KXFEDDECISION", 20, SystemClock::new().now())
                .await
                .map_err(|error| format!("Kalshi validation failed: {error}"))?;
            let quoted = markets.iter().filter(|m| m.yes_price.is_some()).count();
            if markets.is_empty() {
                return Err(
                    "Kalshi answered but returned no markets for the Fed decision series."
                        .to_owned(),
                );
            }
            activate("kalshi", ActiveIntegration::Kalshi)?;
            Ok(IntegrationTestResult {
                provider_id,
                status: "connected",
                message: format!(
                    "Kalshi public API reachable; {} Fed-decision markets normalized, {quoted} \
                     carrying a quote.",
                    markets.len()
                ),
                evidence: "Provider adapter · BudgetGovernor permit · GET /trade-api/v2/markets · \
                     KXFEDDECISION"
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
    use super::{required, LIVE_ADAPTERS};
    use std::collections::BTreeMap;

    /// Every advertised adapter must have a real match arm in
    /// `test_integration`, and every arm must be advertised.
    ///
    /// Reading the source is crude, but it catches the exact drift that made
    /// a tester report that nothing works: a provider listed as connectable
    /// with no code behind it, or code behind a provider nobody can reach.
    #[test]
    fn advertised_adapters_and_implemented_arms_agree() {
        let source = include_str!("integrations.rs");
        // The arms of the `match provider_id.as_str()` in test_integration.
        let implemented: Vec<&str> = source
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                line.strip_suffix("\" => {")
                    .and_then(|rest| rest.strip_prefix('"'))
            })
            .collect();

        for provider in LIVE_ADAPTERS {
            assert!(
                implemented.contains(&provider),
                "{provider} is advertised as a live adapter but test_integration has no arm \
                 for it — the setup wizard would dead-end",
            );
        }
        for provider in &implemented {
            assert!(
                LIVE_ADAPTERS.contains(provider),
                "{provider} has an adapter arm but is not advertised, so nothing can reach it",
            );
        }
    }

    #[test]
    fn every_live_adapter_has_a_rate_governor() {
        for provider in LIVE_ADAPTERS {
            assert!(
                super::INTEGRATION_GOVERNORS.contains_key(provider),
                "{provider} would be refused by admit_interactive",
            );
        }
    }

    #[test]
    fn required_credentials_fail_closed() {
        assert_eq!(
            required(&BTreeMap::new(), "apiKey", "Example"),
            Err("Example requires apiKey".to_owned())
        );
    }
}
