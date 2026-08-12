//! Instrument discovery — find things worth tracking, then measure them.
//!
//! Until now PRISMATIK could only analyse what an operator had already thought
//! to track. That is a real ceiling: the regime classifier and the edge
//! measurement are only as good as the universe they are pointed at, and a
//! hand-picked watchlist is a biased universe by construction.
//!
//! This module sweeps thousands of instruments, then runs the *same*
//! deterministic analysis over the top candidates. The screener supplies
//! candidates; it never supplies evidence. A hit becomes something PRISMATIK
//! reasons about only once it has been tracked and quoted by a provider the
//! app has terms with — the screener's own prices are shown for ranking and
//! are labelled as unofficial.
//!
//! The source is opt-in and off by default. See `tradingview_screener` in
//! `prismatik-market-data` for why.

use std::sync::Arc;

use prismatik_application::ReqwestTransport;
use prismatik_determinism::{Clock, SystemClock};
use prismatik_market_data::adapters::{ScreenerMarket, ScreenerSort, TradingViewScreener};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::tracking::{self, InstrumentKind};

const SCANNER_BASE: &str = "https://scanner.tradingview.com";

/// Identifier the integrations plane uses for the opt-in.
pub(crate) const SOURCE_ID: &str = "tradingview-screener";

/// Instruments analysed per sweep.
///
/// Each one costs a history fetch and a full classification, so this is
/// deliberately small: the screener ranks hundreds, the classifier measures a
/// handful, and the operator tracks the few that survive both.
const ANALYSE_TOP_N: usize = 8;

/// One screened instrument, optionally with PRISMATIK's own measurement.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScreenedInstrument {
    pub(crate) symbol: String,
    pub(crate) qualified: String,
    pub(crate) exchange: String,
    /// Screener price. Unofficial — for ranking, never cited as an observation.
    pub(crate) screener_price: f64,
    pub(crate) change_pct: f64,
    pub(crate) volume: f64,
    pub(crate) market_cap: Option<f64>,
    /// Already in the tracked list.
    pub(crate) tracked: bool,
    /// Measured regime, when this hit was analysed.
    pub(crate) regime: Option<String>,
    /// Edge over climatology in ppm, when analysed.
    pub(crate) edge_ppm: Option<i32>,
    /// Whether PRISMATIK's own gate would act on it.
    pub(crate) actionable: Option<bool>,
    /// Why it was or was not analysed, and what the analysis said.
    pub(crate) note: String,
}

/// Result of one sweep.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScreenerResult {
    pub(crate) market: String,
    pub(crate) hits: Vec<ScreenedInstrument>,
    /// Total instruments matching upstream, before paging.
    pub(crate) total_count: usize,
    pub(crate) analysed: usize,
    pub(crate) undecodable_rows: usize,
    pub(crate) retrieved_at: String,
    pub(crate) message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScreenerRequest {
    pub(crate) market: String,
    /// `volume`, `change_desc`, `change_asc` or `market_cap`.
    pub(crate) sort: Option<String>,
    pub(crate) limit: Option<usize>,
    /// Minimum turnover (price x volume) in quote currency.
    pub(crate) min_turnover: Option<f64>,
    /// Minimum price, to exclude the sub-penny tail.
    pub(crate) min_price: Option<f64>,
    /// Run the regime classifier over the top hits. Costs a history fetch each.
    pub(crate) analyse: Option<bool>,
}

fn parse_sort(value: Option<&str>) -> ScreenerSort {
    // Defaults to the empty string, not "volume": an absent sort must fall
    // through to the turnover default rather than matching the explicit
    // volume arm.
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "change_desc" | "gainers" => ScreenerSort::ChangeDesc,
        "change_asc" | "losers" => ScreenerSort::ChangeAsc,
        "market_cap" => ScreenerSort::MarketCap,
        "volume" => ScreenerSort::Volume,
        // Turnover is the default: unit volume ranks a sub-penny shell above
        // SPY, which is not a screener anyone can use.
        _ => ScreenerSort::Turnover,
    }
}

/// A screener market maps onto the provider plane that can actually quote it.
fn kind_for(market: ScreenerMarket) -> Option<InstrumentKind> {
    match market {
        ScreenerMarket::America => Some(InstrumentKind::Equity),
        ScreenerMarket::Crypto => Some(InstrumentKind::Crypto),
        // FX has no quote provider wired, so an FX hit can be screened but not
        // tracked. Saying so is better than offering a row that cannot work.
        ScreenerMarket::Forex => None,
    }
}

/// Sweep a market and optionally measure the top candidates.
#[tauri::command]
pub(crate) async fn screen_instruments(
    app: AppHandle,
    request: ScreenerRequest,
) -> Result<ScreenerResult, String> {
    if !crate::integrations::unofficial_source_enabled(SOURCE_ID) {
        return Err(format!(
            "the {SOURCE_ID} source is unofficial and disabled. Enable it in Integrations to use \
             the screener; its results are discovery candidates, not citable observations."
        ));
    }

    let market = ScreenerMarket::parse(&request.market).map_err(|error| error.to_string())?;
    let now = SystemClock::new().now();
    let transport = ReqwestTransport::new(SCANNER_BASE)
        .map_err(|error| format!("screener transport failed: {error}"))?;
    let screener = TradingViewScreener::new(Arc::new(transport));

    let page = screener
        .scan(
            market,
            parse_sort(request.sort.as_deref()),
            request.limit.unwrap_or(50),
            // Defaults chosen to return instruments a desk could actually
            // trade: $50m of turnover and a $1 price floor.
            request.min_turnover.unwrap_or(50_000_000.0),
            request.min_price.unwrap_or(1.0),
            now,
        )
        .await
        .map_err(|error| error.to_string())?;

    let tracked = tracking::read_tracked(&app)?;
    let is_tracked = |symbol: &str| {
        tracked
            .iter()
            .any(|row| row.symbol.eq_ignore_ascii_case(symbol))
    };

    let analyse = request.analyse.unwrap_or(false);
    let kind = kind_for(market);
    let mut analysed = 0_usize;
    let mut hits = Vec::with_capacity(page.hits.len());

    for (index, hit) in page.hits.iter().enumerate() {
        let mut row = ScreenedInstrument {
            symbol: hit.symbol.clone(),
            qualified: hit.qualified.clone(),
            exchange: hit.exchange.clone(),
            screener_price: hit.price,
            change_pct: hit.change_pct,
            volume: hit.volume,
            market_cap: hit.market_cap,
            tracked: is_tracked(&hit.symbol),
            regime: None,
            edge_ppm: None,
            actionable: None,
            note: String::new(),
        };

        match kind {
            None => row.note = "no quote provider for this market — screening only".into(),
            Some(kind) if analyse && index < ANALYSE_TOP_N => {
                // The classifier reads real history from the *provider*, not
                // from the screener, so the measurement is trustworthy even
                // though the candidate came from an unofficial source.
                let signal = crate::signal::evaluate(&hit.symbol, kind, &hit.symbol).await;
                row.regime = signal.regime.clone();
                row.edge_ppm = Some(signal.edge_ppm);
                row.actionable = Some(signal.actionable);
                row.note = signal.rationale.clone();
                analysed += 1;
            },
            Some(_) => {
                row.note = if analyse {
                    format!("outside the top {ANALYSE_TOP_N} — not analysed")
                } else {
                    "not analysed".into()
                }
            },
        }

        hits.push(row);
    }

    let message = format!(
        "{} of {} matches shown from the unofficial TradingView screener{}{}. Prices here rank \
         candidates; they are not observations and are never cited.",
        hits.len(),
        page.total_count,
        if analysed > 0 {
            format!("; {analysed} measured against real provider history")
        } else {
            String::new()
        },
        if page.undecodable_rows > 0 {
            format!("; {} rows could not be decoded", page.undecodable_rows)
        } else {
            String::new()
        },
    );

    Ok(ScreenerResult {
        market: market.slug().to_owned(),
        hits,
        total_count: page.total_count,
        analysed,
        undecodable_rows: page.undecodable_rows,
        retrieved_at: now.to_string(),
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_aliases_map_to_the_intended_ranking() {
        assert_eq!(parse_sort(Some("gainers")), ScreenerSort::ChangeDesc);
        assert_eq!(parse_sort(Some("losers")), ScreenerSort::ChangeAsc);
        assert_eq!(parse_sort(Some("MARKET_CAP")), ScreenerSort::MarketCap);
        assert_eq!(parse_sort(Some("volume")), ScreenerSort::Volume);
        // Anything unrecognised ranks by turnover rather than erroring: a bad
        // sort should not lose the sweep, and turnover is the only ranking
        // that means the same thing across price scales.
        assert_eq!(parse_sort(Some("nonsense")), ScreenerSort::Turnover);
        assert_eq!(parse_sort(None), ScreenerSort::Turnover);
    }

    #[test]
    fn forex_has_no_trackable_provider() {
        // Screening FX is fine; offering to track it would create a row that
        // can never be quoted.
        assert!(kind_for(ScreenerMarket::Forex).is_none());
        assert_eq!(
            kind_for(ScreenerMarket::America),
            Some(InstrumentKind::Equity)
        );
        assert_eq!(
            kind_for(ScreenerMarket::Crypto),
            Some(InstrumentKind::Crypto)
        );
    }

    #[test]
    fn the_analysis_budget_is_small_enough_to_stay_bounded() {
        // Each analysed hit costs a full history fetch; a sweep must not turn
        // into hundreds of provider calls.
        assert!(ANALYSE_TOP_N <= 10);
    }
}
