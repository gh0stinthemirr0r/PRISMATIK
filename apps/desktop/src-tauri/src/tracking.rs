//! Tracked-instrument store — the single symbol universe the terminal follows.
//!
//! Everything the market surface renders is derived from this list: the ticker
//! tape, the watchlist, the chart's symbol picker and the quote poll in
//! `terminal_feed`. Equities and crypto live in one list because a desk does
//! not think in provider silos; the `kind` discriminant is what routes a symbol
//! to Finnhub or CoinGecko when quotes are fetched.
//!
//! There is no seeded default universe. An empty list means the user has not
//! tracked anything yet, and every consumer says so rather than inventing rows.

use std::{fs, path::PathBuf, sync::Arc};

use prismatik_application::ReqwestTransport;
use prismatik_determinism::{Clock, SystemClock};
use prismatik_market_data::adapters::{CoinGeckoAdapter, CoinGeckoAuth, FinnhubAdapter};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::integrations::{self, ActiveIntegration};

const STORE_FILE: &str = "tracked-instruments.json";

/// Which provider plane a tracked symbol resolves against.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum InstrumentKind {
    Equity,
    Crypto,
}

/// One tracked instrument.
///
/// `provider_id` is the identifier the *provider* uses (a CoinGecko slug such
/// as `bitcoin`, or a Finnhub ticker such as `AAPL`). `symbol` is what the desk
/// reads. Keeping both means a rename upstream never silently repoints a row.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrackedInstrument {
    pub(crate) kind: InstrumentKind,
    pub(crate) symbol: String,
    pub(crate) provider_id: String,
    pub(crate) name: String,
    /// Venue or asset-class label shown next to the symbol.
    pub(crate) market: String,
    /// Price decimals for display. Crypto majors and FX need more than 2.
    pub(crate) decimals: u8,
    pub(crate) added_at: String,
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| format!("resolve app data directory: {error}"))?;
    fs::create_dir_all(&dir).map_err(|error| format!("create app data directory: {error}"))?;
    Ok(dir.join(STORE_FILE))
}

pub(crate) fn read_tracked(app: &AppHandle) -> Result<Vec<TrackedInstrument>, String> {
    let path = store_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn write_tracked(app: &AppHandle, rows: &[TrackedInstrument]) -> Result<(), String> {
    let path = store_path(app)?;
    let bytes =
        serde_json::to_vec_pretty(rows).map_err(|error| format!("serialize tracked: {error}"))?;
    fs::write(&path, bytes).map_err(|error| format!("write {}: {error}", path.display()))
}

/// The instruments the terminal is currently following, in display order.
#[tauri::command]
pub(crate) fn get_tracked_instruments(app: AppHandle) -> Result<Vec<TrackedInstrument>, String> {
    read_tracked(&app)
}

/// Track a new instrument. Idempotent on (kind, provider_id).
#[tauri::command]
pub(crate) fn add_tracked_instrument(
    app: AppHandle,
    instrument: TrackedInstrument,
) -> Result<Vec<TrackedInstrument>, String> {
    let mut rows = read_tracked(&app)?;
    if rows
        .iter()
        .any(|row| row.kind == instrument.kind && row.provider_id == instrument.provider_id)
    {
        return Ok(rows);
    }
    let mut instrument = instrument;
    instrument.added_at = SystemClock::new().now().to_string();
    rows.push(instrument);
    write_tracked(&app, &rows)?;
    Ok(rows)
}

/// Stop tracking an instrument.
#[tauri::command]
pub(crate) fn remove_tracked_instrument(
    app: AppHandle,
    kind: InstrumentKind,
    provider_id: String,
) -> Result<Vec<TrackedInstrument>, String> {
    let mut rows = read_tracked(&app)?;
    rows.retain(|row| !(row.kind == kind && row.provider_id == provider_id));
    write_tracked(&app, &rows)?;
    Ok(rows)
}

/// Persist a user-defined display order.
#[tauri::command]
pub(crate) fn reorder_tracked_instruments(
    app: AppHandle,
    provider_ids: Vec<String>,
) -> Result<Vec<TrackedInstrument>, String> {
    let rows = read_tracked(&app)?;
    let mut ordered: Vec<TrackedInstrument> = Vec::with_capacity(rows.len());
    for id in &provider_ids {
        if let Some(row) = rows.iter().find(|row| &row.provider_id == id) {
            ordered.push(row.clone());
        }
    }
    // Anything the client did not mention keeps its relative position at the end,
    // so a stale reorder can never silently drop a tracked instrument.
    for row in &rows {
        if !ordered
            .iter()
            .any(|kept| kept.provider_id == row.provider_id)
        {
            ordered.push(row.clone());
        }
    }
    write_tracked(&app, &ordered)?;
    Ok(ordered)
}

/// A search hit that can be promoted into the tracked list.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstrumentSearchHit {
    pub(crate) kind: InstrumentKind,
    pub(crate) symbol: String,
    pub(crate) provider_id: String,
    pub(crate) name: String,
    pub(crate) market: String,
    pub(crate) decimals: u8,
    /// Already in the tracked list — the UI renders these as added, not addable.
    pub(crate) tracked: bool,
}

/// Search both provider planes for instruments to track.
///
/// Each plane is only queried when its integration is connected, and a failure
/// on one plane never suppresses results from the other — the caller gets what
/// is genuinely available plus a note about what was not reachable.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstrumentSearchResult {
    pub(crate) hits: Vec<InstrumentSearchHit>,
    pub(crate) searched_providers: Vec<String>,
    pub(crate) message: String,
}

#[tauri::command]
pub(crate) async fn search_instruments(
    app: AppHandle,
    query: String,
) -> Result<InstrumentSearchResult, String> {
    let query = query.trim().to_owned();
    if query.is_empty() {
        return Ok(InstrumentSearchResult {
            hits: Vec::new(),
            searched_providers: Vec::new(),
            message: "Enter a symbol or name to search".into(),
        });
    }

    let tracked = read_tracked(&app)?;
    let is_tracked = |kind: InstrumentKind, provider_id: &str| {
        tracked
            .iter()
            .any(|row| row.kind == kind && row.provider_id == provider_id)
    };

    let now = SystemClock::new().now();
    let mut hits = Vec::new();
    let mut searched = Vec::new();
    let mut failures = Vec::new();

    if let Some(ActiveIntegration::CoinGecko { api_key }) = integrations::active("coingecko") {
        match integrations::admit_background("coingecko").and_then(|()| {
            ReqwestTransport::new("https://api.coingecko.com/api/v3")
                .map_err(|error| format!("transport configuration failed: {error}"))
        }) {
            Ok(transport) => {
                let adapter =
                    CoinGeckoAdapter::new(Arc::new(transport), CoinGeckoAuth::Demo { api_key });
                match adapter.search(&query).await {
                    Ok(rows) => {
                        searched.push("CoinGecko".to_owned());
                        for row in rows.into_iter().take(15) {
                            let provider_id = row.coingecko_id.clone();
                            hits.push(InstrumentSearchHit {
                                kind: InstrumentKind::Crypto,
                                symbol: row.symbol.to_uppercase(),
                                tracked: is_tracked(InstrumentKind::Crypto, &provider_id),
                                provider_id,
                                name: row.name,
                                market: "CRYPTO".to_owned(),
                                decimals: 4,
                            });
                        }
                    },
                    Err(error) => failures.push(format!("CoinGecko: {error}")),
                }
            },
            Err(error) => failures.push(format!("CoinGecko: {error}")),
        }
    }

    if let Some(ActiveIntegration::Finnhub { token }) = integrations::active("finnhub") {
        match integrations::admit_background("finnhub").and_then(|()| {
            ReqwestTransport::new("https://finnhub.io/api/v1")
                .map_err(|error| format!("transport configuration failed: {error}"))
        }) {
            Ok(transport) => {
                let adapter = FinnhubAdapter::new(Arc::new(transport), token);
                match adapter.search(&query, now).await {
                    Ok(rows) => {
                        searched.push("Finnhub".to_owned());
                        for row in rows.into_iter().take(15) {
                            let provider_id = row.symbol.clone();
                            hits.push(InstrumentSearchHit {
                                kind: InstrumentKind::Equity,
                                symbol: row.symbol.clone(),
                                tracked: is_tracked(InstrumentKind::Equity, &provider_id),
                                provider_id,
                                name: row.description,
                                market: row.exchange,
                                decimals: 2,
                            });
                        }
                    },
                    Err(error) => failures.push(format!("Finnhub: {error}")),
                }
            },
            Err(error) => failures.push(format!("Finnhub: {error}")),
        }
    }

    let message = if searched.is_empty() {
        "No market provider connected. Connect CoinGecko or Finnhub in Integrations to search."
            .to_owned()
    } else if hits.is_empty() && failures.is_empty() {
        format!("No match for \"{query}\" on {}", searched.join(" + "))
    } else if failures.is_empty() {
        format!("{} results from {}", hits.len(), searched.join(" + "))
    } else {
        format!(
            "{} results from {} · unavailable: {}",
            hits.len(),
            searched.join(" + "),
            failures.join("; ")
        )
    };

    Ok(InstrumentSearchResult {
        hits,
        searched_providers: searched,
        message,
    })
}
