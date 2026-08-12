//! Regime analytics over tracked instruments.
//!
//! This module is the bridge between the free daily history PRISMATIK already
//! fetches (Yahoo for equities, CoinGecko for crypto) and the pure analysis in
//! `prismatik-regime`. It owns no maths of its own — it fetches bars, hands
//! them to the crate, and shapes the result for the UI — so the numbers on
//! screen are produced by code that is unit-tested in isolation.

use prismatik_regime::{
    bootstrap_paths, classify, empirical_forecast, realized_vol_surface, regime_survival,
    transitions, Bar, EmpiricalForecast, Regime, RegimeParams, ScenarioPaths, SurvivalCurve,
    TransitionMatrix, VolSurface,
};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::tracking::{self, InstrumentKind};

/// Lookback windows for the realized-volatility surface, in daily bars.
const VOL_LOOKBACKS: [usize; 4] = [10, 21, 63, 126];
/// Forward windows the surface is checked against, in daily bars.
const VOL_HORIZONS: [usize; 3] = [5, 21, 63];
/// Bars projected by the scenario fan.
const SCENARIO_HORIZON: usize = 30;
/// Paths in the scenario fan.
const SCENARIO_PATHS: usize = 120;
/// Block length for the bootstrap, long enough to carry volatility clustering.
const SCENARIO_BLOCK: usize = 5;

/// A regime label paired with its display metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegimeLabel {
    pub(crate) id: String,
    pub(crate) t: i64,
}

/// Everything derivable from one instrument's price history.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstrumentAnalytics {
    pub(crate) symbol: String,
    pub(crate) kind: InstrumentKind,
    /// Bars actually used, after provider trimming.
    pub(crate) bar_count: usize,
    /// First and last bar timestamps, ISO-8601.
    pub(crate) first_bar: Option<String>,
    pub(crate) last_bar: Option<String>,
    /// Where the history came from.
    pub(crate) source: String,
    /// Current regime id, or `None` when there is not enough history.
    pub(crate) current_regime: Option<String>,
    /// Bars the current regime has held.
    pub(crate) current_run_length: usize,
    /// Latest annualized realized volatility, as a fraction.
    pub(crate) realized_vol: f64,
    /// Where that sits in the instrument's own history, 0..1.
    pub(crate) vol_percentile: f64,
    /// Variance ratio behind the trend axis.
    pub(crate) variance_ratio: f64,
    /// Drift t-statistic behind the trend axis.
    pub(crate) drift_t: f64,
    /// Regime label per classified bar, for timeline rendering.
    pub(crate) timeline: Vec<RegimeLabel>,
    pub(crate) transitions: TransitionMatrix,
    pub(crate) survival: SurvivalCurve,
    pub(crate) vol_surface: VolSurface,
    pub(crate) scenarios: ScenarioPaths,
    pub(crate) forecasts: Vec<EmpiricalForecast>,
    /// Why analysis is incomplete, when it is.
    pub(crate) message: String,
}

/// Convert a provider bar envelope into the crate's bar type.
fn to_bars(result: &crate::historical_data::HistoricalDataResult) -> Vec<Bar> {
    result
        .bars
        .iter()
        .filter_map(|row| {
            let t = time::OffsetDateTime::parse(
                &row.timestamp,
                &time::format_description::well_known::Rfc3339,
            )
            .ok()
            .map(|dt| dt.unix_timestamp() * 1000)?;
            // A provider gap arrives as a zero close; carrying it into a return
            // series would manufacture a -100% bar.
            (row.close > 0.0 && row.close.is_finite()).then_some(Bar {
                t,
                o: if row.open > 0.0 { row.open } else { row.close },
                h: if row.high > 0.0 { row.high } else { row.close },
                l: if row.low > 0.0 { row.low } else { row.close },
                c: row.close,
                v: row.volume.max(0.0),
            })
        })
        .collect()
}

/// Fetch the longest daily history available for a tracked instrument.
pub(crate) async fn fetch_daily_bars(
    kind: InstrumentKind,
    provider_id: &str,
) -> Result<(Vec<Bar>, String), String> {
    let result = match kind {
        InstrumentKind::Crypto => {
            crate::historical_data::get_crypto_historical(
                provider_id.to_owned(),
                Some("usd".to_owned()),
                // CoinGecko's free tier returns daily granularity beyond 90 days.
                Some(1825),
            )
            .await?
        },
        InstrumentKind::Equity => {
            crate::historical_data::get_historical_ohlcv(
                provider_id.to_owned(),
                Some("1d".to_owned()),
                Some("5y".to_owned()),
            )
            .await?
        },
    };
    let source = result.source.clone();
    Ok((to_bars(&result), source))
}

/// Horizons, in trading days, that forecasts are produced for.
pub(crate) const FORECAST_HORIZONS_DAYS: [usize; 3] = [1, 5, 21];

/// Analyze one tracked instrument end to end.
#[tauri::command]
pub(crate) async fn analyze_instrument(
    kind: InstrumentKind,
    provider_id: String,
    symbol: String,
) -> Result<InstrumentAnalytics, String> {
    let (bars, source) = fetch_daily_bars(kind, &provider_id).await?;
    let params = RegimeParams::daily();
    let classification = classify(&bars, &params);

    let latest = classification
        .samples
        .iter()
        .rev()
        .find(|s| s.regime.is_some());

    let message = if bars.is_empty() {
        "Provider returned no daily bars".to_owned()
    } else if classification.current.is_none() {
        format!(
            "{} bars available; {} needed before a regime can be classified",
            bars.len(),
            params.warmup() + 1
        )
    } else {
        format!("{} daily bars from {source}", bars.len())
    };

    let forecasts = FORECAST_HORIZONS_DAYS
        .iter()
        .filter_map(|&h| empirical_forecast(&bars, &classification, h))
        .collect();

    Ok(InstrumentAnalytics {
        symbol,
        kind,
        bar_count: bars.len(),
        first_bar: bars.first().map(|b| iso(b.t)),
        last_bar: bars.last().map(|b| iso(b.t)),
        source,
        current_regime: classification.current.map(|r| r.id().to_owned()),
        current_run_length: classification.current_run_length,
        realized_vol: latest.map_or(0.0, |s| s.realized_vol),
        vol_percentile: latest.map_or(0.0, |s| s.vol_percentile),
        variance_ratio: latest.map_or(1.0, |s| s.variance_ratio),
        drift_t: latest.map_or(0.0, |s| s.drift_t),
        timeline: classification
            .labels()
            .into_iter()
            .map(|(t, r)| RegimeLabel {
                id: r.id().to_owned(),
                t,
            })
            .collect(),
        transitions: transitions(&classification, 60),
        survival: regime_survival(&classification, classification.current),
        vol_surface: realized_vol_surface(&bars, &VOL_LOOKBACKS, &VOL_HORIZONS, 252.0),
        // Seeded from the last bar so the fan is stable for a given history and
        // changes only when new data actually arrives.
        scenarios: bootstrap_paths(
            &bars,
            &classification,
            SCENARIO_HORIZON,
            SCENARIO_PATHS,
            SCENARIO_BLOCK,
            bars.last().map_or(1, |b| b.t as u64),
        ),
        forecasts,
        message,
    })
}

fn iso(epoch_ms: i64) -> String {
    time::OffsetDateTime::from_unix_timestamp(epoch_ms / 1000)
        .map(|dt| dt.to_string())
        .unwrap_or_default()
}

/// Regime of every tracked instrument, for the correlation field's colouring.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegimeSnapshot {
    pub(crate) symbol: String,
    pub(crate) regime: Option<String>,
    pub(crate) run_length: usize,
    pub(crate) realized_vol: f64,
    pub(crate) vol_percentile: f64,
    pub(crate) bar_count: usize,
}

/// Classify every tracked instrument.
///
/// Failures are per-instrument: one unreachable symbol reports a `None` regime
/// rather than failing the whole sweep, because a partial map is still useful
/// and a total failure is not.
#[tauri::command]
pub(crate) async fn regime_snapshot(app: AppHandle) -> Result<Vec<RegimeSnapshot>, String> {
    let tracked = tracking::read_tracked(&app)?;
    let params = RegimeParams::daily();
    let mut out = Vec::with_capacity(tracked.len());
    for row in tracked {
        let (regime, run_length, realized_vol, vol_percentile, bar_count) =
            match fetch_daily_bars(row.kind, &row.provider_id).await {
                Ok((bars, _)) => {
                    let classification = classify(&bars, &params);
                    let latest = classification
                        .samples
                        .iter()
                        .rev()
                        .find(|s| s.regime.is_some());
                    (
                        classification.current.map(|r| r.id().to_owned()),
                        classification.current_run_length,
                        latest.map_or(0.0, |s| s.realized_vol),
                        latest.map_or(0.0, |s| s.vol_percentile),
                        bars.len(),
                    )
                },
                Err(_) => (None, 0, 0.0, 0.0, 0),
            };
        out.push(RegimeSnapshot {
            symbol: row.symbol,
            regime,
            run_length,
            realized_vol,
            vol_percentile,
            bar_count,
        });
    }
    Ok(out)
}

/// Produce and file empirical forecast candidates for every tracked instrument.
///
/// This is what closes the loop: it needs no model provider and costs nothing,
/// so it can run on a schedule, and every candidate it files is resolved and
/// Brier-scored by the same machinery that scores the LLM forecasts. The two
/// forecasters land in separate cohorts and can therefore be compared directly.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmpiricalSweepReport {
    pub(crate) filed: usize,
    pub(crate) skipped: usize,
    pub(crate) reasons: Vec<String>,
}

#[tauri::command]
pub(crate) async fn sweep_empirical_forecasts(
    app: AppHandle,
) -> Result<EmpiricalSweepReport, String> {
    let tracked = tracking::read_tracked(&app)?;
    if tracked.is_empty() {
        return Ok(EmpiricalSweepReport {
            filed: 0,
            skipped: 0,
            reasons: vec!["No instruments tracked".to_owned()],
        });
    }

    // Baselines must be a real, current quote — the same source the resolver
    // will later read — or the forecast could never be scored honestly.
    let snapshot = crate::terminal_feed::get_terminal_feed(app.clone()).await?;
    let params = RegimeParams::daily();
    let mut filed = 0_usize;
    let mut skipped = 0_usize;
    let mut reasons = Vec::new();

    for row in tracked {
        let Some(quote) = snapshot
            .quotes
            .iter()
            .find(|q| q.symbol.eq_ignore_ascii_case(&row.symbol))
        else {
            skipped += 1;
            reasons.push(format!("{}: no current quote", row.symbol));
            continue;
        };

        let bars = match fetch_daily_bars(row.kind, &row.provider_id).await {
            Ok((bars, _)) => bars,
            Err(error) => {
                skipped += 1;
                reasons.push(format!("{}: {error}", row.symbol));
                continue;
            },
        };
        let classification = classify(&bars, &params);
        if classification.current.is_none() {
            skipped += 1;
            reasons.push(format!("{}: insufficient history to classify", row.symbol));
            continue;
        }

        for horizon_days in FORECAST_HORIZONS_DAYS {
            let Some(forecast) = empirical_forecast(&bars, &classification, horizon_days) else {
                continue;
            };
            if !forecast.sufficient {
                skipped += 1;
                reasons.push(format!(
                    "{} {}d: only {} historical episodes",
                    row.symbol, horizon_days, forecast.sample_size
                ));
                continue;
            }
            match crate::forecast_candidates::file_empirical_candidate(
                &row.symbol,
                &forecast,
                horizon_days,
                quote.price,
                &quote.observed_at,
                bars.len(),
            ) {
                Ok(true) => filed += 1,
                Ok(false) => skipped += 1,
                Err(error) => {
                    skipped += 1;
                    reasons.push(format!("{} {}d: {error}", row.symbol, horizon_days));
                },
            }
        }
    }

    Ok(EmpiricalSweepReport {
        filed,
        skipped,
        reasons,
    })
}

/// Regimes the UI knows how to colour, exported so the frontend never invents
/// a label the classifier cannot produce.
#[tauri::command]
pub(crate) fn regime_catalog() -> Vec<String> {
    Regime::ALL.into_iter().map(|r| r.id().to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::historical_data::{HistoricalDataResult, OhlcvBar};

    fn row(timestamp: &str, close: f64) -> OhlcvBar {
        OhlcvBar {
            timestamp: timestamp.to_owned(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
            adjusted_close: None,
        }
    }

    fn envelope(bars: Vec<OhlcvBar>) -> HistoricalDataResult {
        let count = bars.len();
        HistoricalDataResult {
            symbol: "TEST".into(),
            interval: "1d".into(),
            bars,
            source: "test".into(),
            count,
            first_date: String::new(),
            last_date: String::new(),
        }
    }

    #[test]
    fn zero_closes_are_dropped_rather_than_becoming_minus_one_hundred_percent() {
        let result = envelope(vec![
            row("2024-01-01T00:00:00Z", 100.0),
            row("2024-01-02T00:00:00Z", 0.0),
            row("2024-01-03T00:00:00Z", 102.0),
        ]);
        let bars = to_bars(&result);
        assert_eq!(bars.len(), 2);
        assert!(bars.iter().all(|b| b.c > 0.0));
    }

    #[test]
    fn unparseable_timestamps_are_dropped() {
        let result = envelope(vec![
            row("not-a-date", 100.0),
            row("2024-01-03T00:00:00Z", 102.0),
        ]);
        assert_eq!(to_bars(&result).len(), 1);
    }

    #[test]
    fn missing_ohlc_falls_back_to_the_close() {
        let mut bar = row("2024-01-03T00:00:00Z", 102.0);
        bar.open = 0.0;
        bar.high = 0.0;
        bar.low = 0.0;
        let bars = to_bars(&envelope(vec![bar]));
        assert_eq!(bars[0].o, 102.0);
        assert_eq!(bars[0].h, 102.0);
        assert_eq!(bars[0].l, 102.0);
    }
}
