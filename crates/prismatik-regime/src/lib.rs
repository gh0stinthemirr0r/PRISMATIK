//! Regime classification and regime-conditional forecasting over OHLCV bars.
//!
//! Everything in this crate is a pure function of the bars handed to it. There
//! is no I/O, no clock and no unseeded randomness, so a given series always
//! produces the same classification, the same transition matrix and the same
//! scenario paths. That determinism is what makes a forecast produced here
//! auditable: the inputs and the seed fully explain the output.
//!
//! The five regimes are the two axes a desk actually trades — how violent the
//! tape is, and whether moves continue or revert — plus a crisis state that
//! overrides both when volatility and drawdown break down together.
//!
//! ```
//! use prismatik_regime::{classify, Bar, RegimeParams};
//! let bars: Vec<Bar> = (0..400)
//!     .map(|i| {
//!         let c = 100.0 + f64::from(i) * 0.1;
//!         Bar { t: i64::from(i), o: c, h: c * 1.01, l: c * 0.99, c, v: 1000.0 }
//!     })
//!     .collect();
//! let result = classify(&bars, &RegimeParams::daily());
//! assert!(result.current.is_some());
//! ```

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

mod forecast;
mod scenario;
mod survival;
mod volatility;

pub use forecast::{
    climatology_for, empirical_forecast, Climatology, EmpiricalForecast, ForecastDirection,
};
pub use scenario::{bootstrap_paths, ScenarioPath, ScenarioPaths, ScenarioPoint};
pub use survival::{regime_survival, SurvivalCurve, SurvivalPoint};
pub use volatility::{realized_vol_surface, VolCell, VolSurface};

/// One OHLCV bar. `t` is epoch milliseconds at bar start.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bar {
    /// Bar start, epoch milliseconds.
    pub t: i64,
    /// Open.
    pub o: f64,
    /// High.
    pub h: f64,
    /// Low.
    pub l: f64,
    /// Close.
    pub c: f64,
    /// Volume.
    pub v: f64,
}

/// Market state over the classification window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Regime {
    /// Low volatility, moves persist.
    CalmTrending,
    /// Low volatility, moves revert.
    CalmMeanRevert,
    /// High volatility, moves persist.
    VolatileTrending,
    /// High volatility, moves revert.
    VolatileMeanRevert,
    /// Volatility spike coinciding with a deep drawdown.
    Crisis,
}

impl Regime {
    /// Every regime, in a stable order suitable for matrix axes.
    pub const ALL: [Self; 5] = [
        Self::CalmTrending,
        Self::CalmMeanRevert,
        Self::VolatileTrending,
        Self::VolatileMeanRevert,
        Self::Crisis,
    ];

    /// Stable snake_case identifier, matching the serialized form.
    pub fn id(self) -> &'static str {
        match self {
            Self::CalmTrending => "calm_trending",
            Self::CalmMeanRevert => "calm_mean_revert",
            Self::VolatileTrending => "volatile_trending",
            Self::VolatileMeanRevert => "volatile_mean_revert",
            Self::Crisis => "crisis",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::CalmTrending => 0,
            Self::CalmMeanRevert => 1,
            Self::VolatileTrending => 2,
            Self::VolatileMeanRevert => 3,
            Self::Crisis => 4,
        }
    }
}

/// Classification thresholds.
///
/// Volatility is judged against the instrument's *own* history rather than an
/// absolute number, because "high vol" means something different for a treasury
/// ETF and for a small-cap token.
#[derive(Clone, Copy, Debug)]
pub struct RegimeParams {
    /// Bars used for the realized-volatility estimate.
    pub vol_window: usize,
    /// Bars used for the variance-ratio (persistence) estimate.
    pub trend_window: usize,
    /// Aggregation lag for the variance ratio.
    pub variance_ratio_lag: usize,
    /// Quantile of the instrument's own volatility above which it is "volatile".
    pub vol_quantile: f64,
    /// Volatility quantile above which crisis becomes possible.
    pub crisis_vol_quantile: f64,
    /// Drawdown from running peak that, with crisis-level vol, means crisis.
    pub crisis_drawdown: f64,
    /// Drift t-statistic above which a window counts as directional.
    pub drift_t_threshold: f64,
    /// Hysteresis half-width on the variance-ratio axis.
    ///
    /// Without this the trend axis is a knife-edge at 1.0 and the label flips
    /// every couple of bars as the estimate jitters across it — producing 400
    /// "regime changes" in five years of daily data, a median run length of two
    /// bars, and a regime-conditional forecast that is conditional on nothing.
    /// A state only flips once the evidence clears the band; inside it, the
    /// previous state persists.
    pub variance_ratio_band: f64,
    /// Hysteresis half-width on the volatility-percentile axis.
    pub vol_quantile_band: f64,
    /// Bars used for the running peak that drawdown is measured from.
    pub drawdown_window: usize,
    /// Bars per year, for annualizing volatility.
    pub periods_per_year: f64,
}

impl RegimeParams {
    /// Defaults tuned for daily bars.
    pub fn daily() -> Self {
        Self {
            vol_window: 21,
            trend_window: 63,
            variance_ratio_lag: 5,
            vol_quantile: 0.60,
            crisis_vol_quantile: 0.90,
            crisis_drawdown: 0.15,
            drift_t_threshold: 3.0,
            variance_ratio_band: 0.15,
            vol_quantile_band: 0.10,
            drawdown_window: 252,
            periods_per_year: 252.0,
        }
    }

    /// Minimum bars needed before the first classification can be produced.
    pub fn warmup(&self) -> usize {
        self.vol_window.max(self.trend_window) + self.variance_ratio_lag + 1
    }
}

impl Default for RegimeParams {
    fn default() -> Self {
        Self::daily()
    }
}

/// Per-bar diagnostics behind one classification.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimeSample {
    /// Bar start, epoch milliseconds.
    pub t: i64,
    /// Classified regime, if the bar had enough lookback.
    pub regime: Option<Regime>,
    /// Annualized realized volatility over `vol_window`.
    pub realized_vol: f64,
    /// Where that volatility sits in the instrument's own history, 0..1.
    pub vol_percentile: f64,
    /// Variance ratio; >1 persists, <1 reverts.
    pub variance_ratio: f64,
    /// t-statistic of mean return over the trend window; large means directional.
    pub drift_t: f64,
    /// Drawdown from the running peak, as a positive fraction.
    pub drawdown: f64,
}

/// Result of classifying a series.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimeClassification {
    /// One sample per input bar, in the same order.
    pub samples: Vec<RegimeSample>,
    /// Regime of the most recent classifiable bar.
    pub current: Option<Regime>,
    /// How many consecutive bars the current regime has held.
    pub current_run_length: usize,
    /// Bars that were classified at all.
    pub classified_count: usize,
}

impl RegimeClassification {
    /// The classified regimes in order, dropping warmup bars.
    pub fn labels(&self) -> Vec<(i64, Regime)> {
        self.samples
            .iter()
            .filter_map(|s| s.regime.map(|r| (s.t, r)))
            .collect()
    }
}

/// Log returns between consecutive closes. Non-finite bars are skipped.
pub(crate) fn log_returns(bars: &[Bar]) -> Vec<f64> {
    let mut out = Vec::with_capacity(bars.len().saturating_sub(1));
    for pair in bars.windows(2) {
        let (prev, next) = (pair[0].c, pair[1].c);
        if prev > 0.0 && next > 0.0 && prev.is_finite() && next.is_finite() {
            out.push((next / prev).ln());
        } else {
            out.push(0.0);
        }
    }
    out
}

pub(crate) fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().sum::<f64>() / xs.len() as f64
}

pub(crate) fn variance(xs: &[f64]) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    let m = mean(xs);
    xs.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (xs.len() - 1) as f64
}

/// Fraction of `sorted` strictly below `value`, in 0..1.
fn percentile_of(sorted: &[f64], value: f64) -> f64 {
    if sorted.is_empty() {
        return 0.5;
    }
    let count = sorted.partition_point(|&x| x < value);
    count as f64 / sorted.len() as f64
}

/// t-statistic of the mean return: mean / (stddev / √n).
///
/// This is the second half of the trend axis. The variance ratio only sees
/// *autocorrelation of deviations* from the mean, so a market grinding steadily
/// upward — the most recognisable trend there is — contributes almost no
/// variance and scores a ratio of ~1, landing on whichever side noise puts it.
/// The drift t-statistic catches exactly that case: a persistent directional
/// move large relative to its own dispersion.
fn drift_t_stat(returns: &[f64]) -> f64 {
    if returns.len() < 3 {
        return 0.0;
    }
    let sd = variance(returns).sqrt();
    if sd <= f64::EPSILON {
        // A perfectly constant non-zero return is maximally directional.
        return if mean(returns).abs() > f64::EPSILON {
            f64::INFINITY
        } else {
            0.0
        };
    }
    let t = mean(returns) / (sd / (returns.len() as f64).sqrt());
    if t.is_finite() {
        t
    } else {
        0.0
    }
}

/// Variance ratio over `lag`, Lo–MacKinlay style with overlapping windows.
///
/// Above 1 the series trends (moves compound); below 1 it mean-reverts. This is
/// preferred to a lag-1 autocorrelation because it is far less sensitive to a
/// single outlier bar.
///
/// The aggregated variance uses *overlapping* k-period sums. Non-overlapping
/// chunks would realign every time the rolling window advances by one bar,
/// making the estimate jump for reasons that have nothing to do with the
/// market — which in turn shreds the regime labels built on top of it.
fn variance_ratio(returns: &[f64], lag: usize) -> f64 {
    let n = returns.len();
    if lag < 2 || n < lag * 3 {
        return 1.0;
    }
    let single = variance(returns);
    if single <= f64::EPSILON {
        return 1.0;
    }

    // Rolling sums of `lag` consecutive returns.
    let mut sums = Vec::with_capacity(n - lag + 1);
    let mut running: f64 = returns[..lag].iter().sum();
    sums.push(running);
    for i in lag..n {
        running += returns[i] - returns[i - lag];
        sums.push(running);
    }

    // Overlapping samples are correlated, so the unbiased denominator is
    // m = lag·(n − lag + 1)·(1 − lag/n) rather than the sample count.
    let mean_sum = mean(&sums);
    let sq: f64 = sums.iter().map(|s| (s - mean_sum) * (s - mean_sum)).sum();
    let m = lag as f64 * (n - lag + 1) as f64 * (1.0 - lag as f64 / n as f64);
    if m <= 0.0 {
        return 1.0;
    }
    let aggregated = sq / m;
    let ratio = aggregated / single;
    if ratio.is_finite() {
        ratio.clamp(0.0, 10.0)
    } else {
        1.0
    }
}

/// Classify every bar that has enough lookback.
///
/// Volatility percentiles are computed against the volatilities observed *up to
/// and including* each bar, never against the whole series. Using the full
/// series would leak future information into a historical label, which would
/// then flatter every backtest and every regime-conditional forecast built on
/// top of it.
pub fn classify(bars: &[Bar], params: &RegimeParams) -> RegimeClassification {
    let mut samples: Vec<RegimeSample> = Vec::with_capacity(bars.len());
    if bars.is_empty() {
        return RegimeClassification {
            samples,
            current: None,
            current_run_length: 0,
            classified_count: 0,
        };
    }

    let returns = log_returns(bars);
    let annualize = params.periods_per_year.sqrt();

    // Volatilities seen so far, kept sorted for streaming percentile lookups.
    let mut seen_vols: Vec<f64> = Vec::new();
    // Latched axis states, carried across bars to give regimes persistence.
    let mut previous_volatile = false;
    let mut previous_trending = false;

    for (i, bar) in bars.iter().enumerate() {
        // `returns[j]` is the move from bar j to bar j+1, so the window ending
        // at bar i is returns[i-vol_window .. i].
        let have = i;
        if have < params.warmup() {
            samples.push(RegimeSample {
                t: bar.t,
                regime: None,
                realized_vol: 0.0,
                vol_percentile: 0.0,
                variance_ratio: 1.0,
                drift_t: 0.0,
                drawdown: 0.0,
            });
            continue;
        }

        let vol_slice = &returns[i - params.vol_window..i];
        let realized_vol = variance(vol_slice).sqrt() * annualize;

        let insert_at = seen_vols.partition_point(|&x| x < realized_vol);
        seen_vols.insert(insert_at, realized_vol);
        let vol_percentile = percentile_of(&seen_vols, realized_vol);

        let trend_start = i.saturating_sub(params.trend_window);
        let trend_slice = &returns[trend_start..i];
        let vr = variance_ratio(trend_slice, params.variance_ratio_lag);
        let drift_t = drift_t_stat(trend_slice);

        let peak_start = i.saturating_sub(params.drawdown_window);
        let peak = bars[peak_start..=i]
            .iter()
            .map(|b| b.c)
            .fold(f64::MIN, f64::max);
        let drawdown = if peak > 0.0 {
            ((peak - bar.c) / peak).max(0.0)
        } else {
            0.0
        };

        // Both axes are latched: they flip only when the evidence clears the
        // hysteresis band, and otherwise hold whatever the previous bar said.
        // `previous_*` carry that state forward.
        let volatile = if vol_percentile >= params.vol_quantile + params.vol_quantile_band {
            true
        } else if vol_percentile <= params.vol_quantile - params.vol_quantile_band {
            false
        } else {
            previous_volatile
        };

        // Either kind of persistence counts: compounding deviations (variance
        // ratio) or a statistically significant directional drift. A clearly
        // significant drift overrides the band outright.
        // Two independent kinds of evidence flip the axis to trending: a
        // statistically significant drift, or a variance ratio clearing the
        // hysteresis band. Only a ratio below the band flips it back; inside
        // the band with no significant drift, the previous state holds.
        let trending =
            if drift_t.abs() > params.drift_t_threshold || vr >= 1.0 + params.variance_ratio_band {
                true
            } else if vr <= 1.0 - params.variance_ratio_band {
                false
            } else {
                previous_trending
            };

        previous_volatile = volatile;
        previous_trending = trending;
        let regime =
            if vol_percentile >= params.crisis_vol_quantile && drawdown >= params.crisis_drawdown {
                Regime::Crisis
            } else {
                match (volatile, trending) {
                    (false, true) => Regime::CalmTrending,
                    (false, false) => Regime::CalmMeanRevert,
                    (true, true) => Regime::VolatileTrending,
                    (true, false) => Regime::VolatileMeanRevert,
                }
            };

        samples.push(RegimeSample {
            t: bar.t,
            regime: Some(regime),
            realized_vol,
            vol_percentile,
            variance_ratio: vr,
            drift_t,
            drawdown,
        });
    }

    let classified_count = samples.iter().filter(|s| s.regime.is_some()).count();
    let current = samples.iter().rev().find_map(|s| s.regime);
    let mut current_run_length = 0;
    if let Some(current) = current {
        for sample in samples.iter().rev() {
            match sample.regime {
                Some(r) if r == current => current_run_length += 1,
                Some(_) => break,
                None => break,
            }
        }
    }

    RegimeClassification {
        samples,
        current,
        current_run_length,
        classified_count,
    }
}

/// One observed regime-to-regime transition.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    /// Source regime.
    pub from: Regime,
    /// Destination regime.
    pub to: Regime,
    /// Times this transition was observed.
    pub count: u32,
    /// Share of transitions leaving `from`, 0..1.
    pub probability: f64,
    /// Occurred within the recency window at the end of the series.
    pub recent: bool,
}

/// Observed transition structure of a classified series.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionMatrix {
    /// Only genuine state changes; self-transitions are excluded.
    pub transitions: Vec<Transition>,
    /// Bars spent in each regime, for sizing the nodes.
    pub occupancy: Vec<(Regime, u32)>,
    /// Total state changes observed.
    pub total_changes: u32,
    /// Regime the series currently sits in.
    pub current: Option<Regime>,
}

/// Build the observed transition matrix.
///
/// Self-transitions are excluded because a regime holding for 40 bars would
/// otherwise dominate every probability and render the off-diagonal structure —
/// the part that carries information — invisible. Persistence is reported
/// separately as occupancy.
pub fn transitions(
    classification: &RegimeClassification,
    recent_window: usize,
) -> TransitionMatrix {
    let labels = classification.labels();
    let mut counts = [[0_u32; 5]; 5];
    let mut recent = [[false; 5]; 5];
    let mut occupancy = [0_u32; 5];
    let mut total_changes = 0_u32;

    let recency_start = labels.len().saturating_sub(recent_window);
    for (i, (_, regime)) in labels.iter().enumerate() {
        occupancy[regime.index()] += 1;
        if i == 0 {
            continue;
        }
        let previous = labels[i - 1].1;
        if previous == *regime {
            continue;
        }
        counts[previous.index()][regime.index()] += 1;
        total_changes += 1;
        if i >= recency_start {
            recent[previous.index()][regime.index()] = true;
        }
    }

    let mut out = Vec::new();
    for from in Regime::ALL {
        let row_total: u32 = counts[from.index()].iter().sum();
        if row_total == 0 {
            continue;
        }
        for to in Regime::ALL {
            let count = counts[from.index()][to.index()];
            if count == 0 {
                continue;
            }
            out.push(Transition {
                from,
                to,
                count,
                probability: f64::from(count) / f64::from(row_total),
                recent: recent[from.index()][to.index()],
            });
        }
    }

    TransitionMatrix {
        transitions: out,
        occupancy: Regime::ALL
            .into_iter()
            .map(|r| (r, occupancy[r.index()]))
            .collect(),
        total_changes,
        current: classification.current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(closes: &[f64]) -> Vec<Bar> {
        closes
            .iter()
            .enumerate()
            .map(|(i, &c)| Bar {
                t: i as i64 * 86_400_000,
                o: c,
                h: c * 1.005,
                l: c * 0.995,
                c,
                v: 1_000.0,
            })
            .collect()
    }

    /// Deterministic pseudo-noise so tests never depend on a RNG.
    fn wobble(i: usize, scale: f64) -> f64 {
        let x = ((i as f64) * 12.9898).sin() * 43_758.545_312;
        (x - x.floor() - 0.5) * scale
    }

    #[test]
    fn empty_input_classifies_to_nothing() {
        let result = classify(&[], &RegimeParams::daily());
        assert!(result.current.is_none());
        assert_eq!(result.classified_count, 0);
    }

    #[test]
    fn warmup_bars_are_left_unclassified() {
        let params = RegimeParams::daily();
        let bars = series(
            &(0..params.warmup() + 50)
                .map(|i| 100.0 + i as f64)
                .collect::<Vec<_>>(),
        );
        let result = classify(&bars, &params);
        assert!(result.samples[..params.warmup()]
            .iter()
            .all(|s| s.regime.is_none()));
        assert!(result.samples[params.warmup()..]
            .iter()
            .all(|s| s.regime.is_some()));
    }

    #[test]
    fn a_series_shorter_than_warmup_never_panics() {
        let params = RegimeParams::daily();
        for n in 0..params.warmup() + 2 {
            let bars = series(&(0..n).map(|i| 100.0 + i as f64).collect::<Vec<_>>());
            let result = classify(&bars, &params);
            assert_eq!(result.samples.len(), n);
        }
    }

    #[test]
    fn a_steady_drift_reads_as_trending() {
        // A market grinding upward has near-zero variance ratio signal — the
        // drift is a constant, so it contributes no variance — and is caught by
        // the drift t-statistic instead. This is the case the variance ratio
        // alone gets wrong.
        let closes: Vec<f64> = (0..400)
            .map(|i| 100.0 * (1.0 + 0.0015 * f64::from(i)) + wobble(i as usize, 0.05))
            .collect();
        let result = classify(&series(&closes), &RegimeParams::daily());
        let current = result.current.expect("classifiable");
        let last = result.samples.last().unwrap();
        assert!(
            last.drift_t.abs() > 2.0,
            "expected a significant drift, got t = {}",
            last.drift_t,
        );
        assert!(
            matches!(current, Regime::CalmTrending | Regime::VolatileTrending),
            "expected a trending regime, got {current:?} (vr = {}, drift_t = {})",
            last.variance_ratio,
            last.drift_t,
        );
    }

    #[test]
    fn momentum_in_the_returns_also_reads_as_trending() {
        // The other kind of persistence: autocorrelated returns, which the
        // variance ratio is designed to catch.
        let mut closes = vec![100.0_f64];
        let mut previous_return = 0.0_f64;
        for i in 0..500 {
            let shock = wobble(i, 0.02);
            let r = 0.6 * previous_return + shock;
            previous_return = r;
            let last = *closes.last().unwrap();
            closes.push(last * (1.0 + r));
        }
        let result = classify(&series(&closes), &RegimeParams::daily());
        let last = result.samples.last().unwrap();
        assert!(
            last.variance_ratio > 1.0,
            "expected a variance ratio above 1, got {}",
            last.variance_ratio,
        );
    }

    #[test]
    fn an_oscillating_series_reads_as_mean_reverting() {
        // Alternating up/down moves cancel under aggregation, so the variance
        // ratio collapses well below 1.
        let mut closes = Vec::new();
        let mut price = 100.0;
        for i in 0..400 {
            price *= if i % 2 == 0 { 1.01 } else { 1.0 / 1.01 };
            closes.push(price);
        }
        let result = classify(&series(&closes), &RegimeParams::daily());
        let current = result.current.expect("classifiable");
        assert!(
            matches!(
                current,
                Regime::CalmMeanRevert | Regime::VolatileMeanRevert | Regime::Crisis
            ),
            "expected mean reversion, got {current:?}",
        );
    }

    #[test]
    fn a_vol_spike_into_deep_drawdown_reads_as_crisis() {
        let mut closes: Vec<f64> = (0..300).map(|i| 100.0 + wobble(i, 0.2)).collect();
        // Sharp sustained selloff: high volatility and a deep drawdown together.
        let mut price = closes[closes.len() - 1];
        for i in 0..60 {
            price *= if i % 3 == 0 { 0.94 } else { 1.01 };
            closes.push(price);
        }
        let result = classify(&series(&closes), &RegimeParams::daily());
        assert_eq!(result.current, Some(Regime::Crisis));
    }

    #[test]
    fn volatility_percentiles_never_use_future_bars() {
        // Truncating the series must not change the labels of the bars that
        // remain — the guarantee that makes historical labels usable.
        let closes: Vec<f64> = (0..500)
            .map(|i| 100.0 * (1.0 + 0.001 * f64::from(i)) + wobble(i as usize, 0.4))
            .collect();
        let params = RegimeParams::daily();
        let full = classify(&series(&closes), &params);
        let truncated = classify(&series(&closes[..400]), &params);
        for i in 0..400 {
            assert_eq!(
                full.samples[i].regime, truncated.samples[i].regime,
                "bar {i} changed label when later bars were added",
            );
        }
    }

    #[test]
    fn transitions_exclude_self_loops_and_normalize_per_row() {
        let closes: Vec<f64> = (0..600)
            .map(|i| {
                let base = 100.0 * (1.0 + 0.0008 * f64::from(i));
                // Alternate calm and violent stretches to force state changes.
                let amp = if (i / 60) % 2 == 0 { 0.1 } else { 3.0 };
                base + wobble(i as usize, amp)
            })
            .collect();
        let classification = classify(&series(&closes), &RegimeParams::daily());
        let matrix = transitions(&classification, 40);
        assert!(matrix.total_changes > 0, "expected regime changes");
        assert!(matrix.transitions.iter().all(|t| t.from != t.to));
        for from in Regime::ALL {
            let row: Vec<_> = matrix
                .transitions
                .iter()
                .filter(|t| t.from == from)
                .collect();
            if row.is_empty() {
                continue;
            }
            let total: f64 = row.iter().map(|t| t.probability).sum();
            assert!((total - 1.0).abs() < 1e-9, "row {from:?} summed to {total}");
        }
    }

    /// A realistic tape: persistent drift, volatility clustering, and a shock.
    fn realistic_series(n: usize) -> Vec<Bar> {
        let mut closes = Vec::with_capacity(n);
        let mut price = 100.0_f64;
        let mut prev = 0.0_f64;
        for i in 0..n {
            // Volatility regimes that last ~120 bars, plus return momentum.
            let vol = if (i / 120) % 3 == 0 { 0.004 } else { 0.016 };
            let shock = wobble(i, vol);
            let r = 0.15 * prev + shock + 0.0004;
            prev = r;
            price *= 1.0 + r;
            closes.push(price);
        }
        series(&closes)
    }

    #[test]
    fn regimes_persist_rather_than_flip_every_other_bar() {
        // Guards the hysteresis and the overlapping variance-ratio estimator.
        // Without both, a five-year daily series produced ~400 regime changes
        // with a median run of two bars — a label that carries no information
        // and makes "regime-conditional" statistics conditional on nothing.
        let bars = realistic_series(1250);
        let classification = classify(&bars, &RegimeParams::daily());
        let labels = classification.labels();
        assert!(labels.len() > 1000, "expected a long classified series");

        let mut runs = Vec::new();
        let mut current = 1_usize;
        for pair in labels.windows(2) {
            if pair[0].1 == pair[1].1 {
                current += 1;
            } else {
                runs.push(current);
                current = 1;
            }
        }
        runs.push(current);
        runs.sort_unstable();
        let median = runs[runs.len() / 2];

        assert!(
            median >= 5,
            "median regime run was {median} bars across {} runs — the classifier is chasing noise",
            runs.len(),
        );
        assert!(
            runs.len() < labels.len() / 10,
            "{} regime changes over {} bars is too many to be meaningful",
            runs.len(),
            labels.len(),
        );
    }

    #[test]
    fn the_variance_ratio_does_not_jump_from_window_realignment() {
        // The overlapping estimator must move smoothly as the window advances;
        // the old non-overlapping version jumped whenever chunk boundaries
        // shifted, purely as an artefact of indexing.
        let bars = realistic_series(600);
        let classification = classify(&bars, &RegimeParams::daily());
        let ratios: Vec<f64> = classification
            .samples
            .iter()
            .filter(|s| s.regime.is_some())
            .map(|s| s.variance_ratio)
            .collect();
        assert!(ratios.len() > 100);
        let biggest_jump = ratios
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            biggest_jump < 0.5,
            "variance ratio jumped by {biggest_jump} between adjacent bars",
        );
    }

    #[test]
    fn run_length_counts_only_the_current_regime() {
        let closes: Vec<f64> = (0..400)
            .map(|i| 100.0 * (1.0 + 0.0015 * f64::from(i)) + wobble(i as usize, 0.05))
            .collect();
        let result = classify(&series(&closes), &RegimeParams::daily());
        let labels = result.labels();
        let current = result.current.unwrap();
        let expected = labels
            .iter()
            .rev()
            .take_while(|(_, r)| *r == current)
            .count();
        assert_eq!(result.current_run_length, expected);
    }
}
