//! Read-only terminal snapshots sourced only from explicitly validated adapters.
//!
//! The symbol universe comes from the user's tracked list (`tracking`), never
//! from a built-in default. A symbol the user has not tracked is never quoted,
//! and a symbol that is tracked but whose provider is unreachable is reported
//! as missing rather than filled in from any other source.

use std::sync::Arc;

use prismatik_application::ReqwestTransport;
use prismatik_determinism::{Clock, SystemClock};
use prismatik_market_data::adapters::{CoinGeckoAdapter, CoinGeckoAuth, FinnhubAdapter};
use serde::Serialize;
use tauri::AppHandle;

use crate::{
    integrations::{self, ActiveIntegration},
    tracking::{self, InstrumentKind},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminalQuote {
    pub(crate) symbol: String,
    pub(crate) price: f64,
    pub(crate) change_pct: Option<f64>,
    pub(crate) volume: Option<f64>,
    pub(crate) provider: String,
    pub(crate) observed_at: String,
}

/// Feed state.
///
/// `unavailable` means nothing could be quoted at all — it is the honest
/// replacement for the old `simulation` mode, which fabricated a market when
/// no provider answered.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminalFeedSnapshot {
    pub(crate) mode: &'static str,
    pub(crate) providers: Vec<String>,
    pub(crate) quotes: Vec<TerminalQuote>,
    /// Tracked symbols that could not be quoted this cycle.
    pub(crate) missing: Vec<String>,
    pub(crate) tracked_count: usize,
    pub(crate) retrieved_at: String,
    pub(crate) message: String,
}

#[tauri::command]
pub(crate) async fn get_terminal_feed(app: AppHandle) -> Result<TerminalFeedSnapshot, String> {
    let now = SystemClock::new().now();
    let tracked = tracking::read_tracked(&app)?;
    let tracked_count = tracked.len();

    if tracked.is_empty() {
        return Ok(TerminalFeedSnapshot {
            mode: "empty",
            providers: Vec::new(),
            quotes: Vec::new(),
            missing: Vec::new(),
            tracked_count: 0,
            retrieved_at: now.to_string(),
            message: "No instruments tracked yet".to_owned(),
        });
    }

    let crypto_ids: Vec<String> = tracked
        .iter()
        .filter(|row| row.kind == InstrumentKind::Crypto)
        .map(|row| row.provider_id.clone())
        .collect();
    let equity_symbols: Vec<String> = tracked
        .iter()
        .filter(|row| row.kind == InstrumentKind::Equity)
        .map(|row| row.provider_id.clone())
        .collect();

    let mut providers = Vec::new();
    let mut quotes = Vec::new();
    let mut failures = Vec::new();

    if !crypto_ids.is_empty() {
        match integrations::active("coingecko") {
            Some(ActiveIntegration::CoinGecko { api_key }) => {
                let ids: Vec<&str> = crypto_ids.iter().map(String::as_str).collect();
                let result = match integrations::admit_background("coingecko") {
                    Ok(()) => match ReqwestTransport::new("https://api.coingecko.com/api/v3") {
                        Ok(transport) => CoinGeckoAdapter::new(
                            Arc::new(transport),
                            CoinGeckoAuth::Demo { api_key },
                        )
                        .markets("usd", &ids, now)
                        .await
                        .map_err(|error| error.to_string()),
                        Err(error) => Err(format!("transport configuration failed: {error}")),
                    },
                    Err(error) => Err(error),
                };
                match result {
                    Ok(rows) => {
                        providers.push("CoinGecko".to_owned());
                        quotes.extend(rows.into_iter().filter_map(|row| {
                            Some(TerminalQuote {
                                symbol: row.symbol.to_uppercase(),
                                price: row.price.parse().ok()?,
                                change_pct: row.change_24h_pct.and_then(|v| v.parse().ok()),
                                volume: row.volume_24h.and_then(|v| v.parse().ok()),
                                provider: "CoinGecko".to_owned(),
                                observed_at: row.event_time.to_string(),
                            })
                        }));
                    },
                    Err(error) => failures.push(format!("CoinGecko: {error}")),
                }
            },
            _ => failures.push("CoinGecko: not connected".to_owned()),
        }
    }

    if !equity_symbols.is_empty() {
        match integrations::active("finnhub") {
            Some(ActiveIntegration::Finnhub { token }) => {
                match integrations::admit_background("finnhub").and_then(|()| {
                    ReqwestTransport::new("https://finnhub.io/api/v1")
                        .map_err(|error| format!("transport configuration failed: {error}"))
                }) {
                    Ok(transport) => {
                        let adapter = FinnhubAdapter::new(Arc::new(transport), token);
                        let to = now.unix_timestamp();
                        let from = to - 7 * 86_400;
                        let mut added = false;
                        for symbol in &equity_symbols {
                            match adapter.bars(symbol, "D", from, to, now).await {
                                Ok(rows) => {
                                    if let Some(row) = rows.last() {
                                        let Ok(close) = row.close.parse::<f64>() else {
                                            continue;
                                        };
                                        let open: f64 = row.open.parse().unwrap_or(close);
                                        quotes.push(TerminalQuote {
                                            symbol: row.symbol.clone(),
                                            price: close,
                                            change_pct: (open != 0.0)
                                                .then_some((close - open) / open * 100.0),
                                            volume: Some(row.volume as f64),
                                            provider: "Finnhub".to_owned(),
                                            observed_at: row.bar_start.to_string(),
                                        });
                                        added = true;
                                    }
                                },
                                Err(error) => failures.push(format!("Finnhub {symbol}: {error}")),
                            }
                        }
                        if added {
                            providers.push("Finnhub".to_owned());
                        }
                    },
                    Err(error) => failures.push(format!("Finnhub: {error}")),
                }
            },
            _ => failures.push("Finnhub: not connected".to_owned()),
        }
    }

    let missing: Vec<String> = tracked
        .iter()
        .filter(|row| {
            !quotes
                .iter()
                .any(|quote| quote.symbol.eq_ignore_ascii_case(&row.symbol))
        })
        .map(|row| row.symbol.clone())
        .collect();

    let mode = if quotes.is_empty() {
        "unavailable"
    } else if missing.is_empty() && failures.is_empty() {
        "live"
    } else {
        "degraded"
    };

    let message = match mode {
        "live" => format!(
            "{} of {tracked_count} tracked · {}",
            quotes.len(),
            providers.join(" + ")
        ),
        "degraded" => {
            if failures.is_empty() {
                format!("{} of {tracked_count} tracked quoted", quotes.len())
            } else {
                format!(
                    "{} of {tracked_count} tracked quoted · {}",
                    quotes.len(),
                    failures.join(" · ")
                )
            }
        },
        _ => {
            if failures.is_empty() {
                "No market provider connected".to_owned()
            } else {
                failures.join(" · ")
            }
        },
    };

    Ok(TerminalFeedSnapshot {
        mode,
        providers,
        quotes,
        missing,
        tracked_count,
        retrieved_at: now.to_string(),
        message,
    })
}
