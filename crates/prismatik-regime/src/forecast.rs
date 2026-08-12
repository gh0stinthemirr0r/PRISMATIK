//! Regime-conditional empirical forecasting.
//!
//! The forecast asks one narrow question: historically, when this instrument
//! was in the regime it is in now, what happened over the next `horizon` bars?
//! The answer is the empirical distribution of those forward returns — no
//! model is fitted and no distribution is assumed, so the output cannot be
//! more confident than the sample behind it.
//!
//! Every forecast also carries its own *climatology*: the same probability
//! computed over the whole series, ignoring regime. The difference between the
//! two — the edge — is the only part that is actually information. A 58% "up"
//! call against a 58% base rate has told you nothing, and without the
//! comparison sitting next to it, it reads as conviction.

use serde::{Deserialize, Serialize};

use crate::{Bar, Regime, RegimeClassification};

/// Direction of the central forecast.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ForecastDirection {
    /// Median forward move is meaningfully positive.
    Up,
    /// Median forward move is meaningfully negative.
    Down,
    /// Median forward move is inside the flat band.
    Flat,
}

/// Empirical forward-return distribution conditional on the current regime.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmpiricalForecast {
    /// Regime the forecast is conditioned on.
    pub regime: Regime,
    /// Forward horizon in bars.
    pub horizon_bars: usize,
    /// Historical episodes of this regime with a full forward window.
    pub sample_size: usize,
    /// Share of those episodes that closed above the flat band, in ppm.
    pub probability_up_ppm: u32,
    /// Share that closed below the flat band, in ppm.
    pub probability_down_ppm: u32,
    /// Share that closed inside the flat band, in ppm.
    pub probability_flat_ppm: u32,
    /// The most likely of the three outcomes.
    pub direction: ForecastDirection,
    /// Probability of `direction` specifically, in ppm.
    ///
    /// This — not `probability_up_ppm` — is what a scorer must use. The Brier
    /// score asks how confident the forecast was *in the claim it made*, so a
    /// "down" call must be scored against P(down), never against P(up).
    pub probability_ppm: u32,
    /// Median forward move, basis points.
    pub median_move_bps: f64,
    /// 10th percentile forward move, basis points.
    pub p10_move_bps: f64,
    /// 90th percentile forward move, basis points.
    pub p90_move_bps: f64,
    /// Unconditional probability of `direction` over the whole series, in ppm.
    ///
    /// The base rate this forecast has to beat to be worth anything.
    pub climatology_ppm: u32,
    /// `probability_ppm` − `climatology_ppm`, in ppm. Signed: negative means
    /// the regime conditioning made the call *worse* than the base rate.
    pub edge_ppm: i32,
    /// Windows behind the climatology estimate.
    pub climatology_sample_size: usize,
    /// Share of the whole series spent in this regime, 0..1.
    pub regime_prevalence: f64,
    /// Whether the sample is large enough to be worth acting on.
    pub sufficient: bool,
}

/// Minimum episodes before a forecast is considered actionable.
///
/// Below this the distribution is reported but flagged insufficient — the
/// pipeline still scores it, which is how the threshold gets validated instead
/// of merely asserted.
pub const MIN_SAMPLE: usize = 30;

/// Moves inside this band count as flat rather than directional.
///
/// This must match the band the resolver uses to decide outcomes. If the
/// forecaster called "flat" at ±25 bps while the resolver settled it at ±5 bps,
/// every flat forecast would be scored against a claim it never made.
pub const FLAT_BAND_BPS: f64 = 5.0;

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let pos = q * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi {
        return sorted[lo];
    }
    let weight = pos - lo as f64;
    sorted[lo] * (1.0 - weight) + sorted[hi] * weight
}

/// Forward returns after every historical occurrence of `regime`.
///
/// Each classified bar in the regime contributes one observation, provided a
/// full `horizon` of bars follows it. Overlapping windows are kept: dropping
/// them would shrink an already small sample far more than it would reduce the
/// dependence between observations.
/// Passing `None` for `regime` yields the unconditional (climatology) sample.
/// It is drawn over the same classified span as the conditional one, so the two
/// are directly comparable — sampling climatology over the full series
/// including warmup bars would compare against a different period.
fn forward_returns(
    bars: &[Bar],
    classification: &RegimeClassification,
    regime: Option<Regime>,
    horizon: usize,
) -> Vec<f64> {
    let mut out = Vec::new();
    if horizon == 0 || bars.len() != classification.samples.len() {
        return out;
    }
    for (i, sample) in classification.samples.iter().enumerate() {
        // Unclassified warmup bars are excluded from both samples.
        let Some(bar_regime) = sample.regime else {
            continue;
        };
        if regime.is_some_and(|target| target != bar_regime) {
            continue;
        }
        let target = i + horizon;
        if target >= bars.len() {
            break;
        }
        let (from, to) = (bars[i].c, bars[target].c);
        if from > 0.0 && to > 0.0 && from.is_finite() && to.is_finite() {
            out.push((to / from - 1.0) * 10_000.0);
        }
    }
    out
}

/// Build a forecast for the regime the series currently sits in.
///
/// Returns `None` when the series has no current classification or when the
/// regime has never previously occurred with a full forward window — there is
/// nothing to condition on, and inventing a prior would defeat the point.
pub fn empirical_forecast(
    bars: &[Bar],
    classification: &RegimeClassification,
    horizon_bars: usize,
) -> Option<EmpiricalForecast> {
    let regime = classification.current?;
    let mut moves = forward_returns(bars, classification, Some(regime), horizon_bars);
    if moves.is_empty() {
        return None;
    }
    moves.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let sample_size = moves.len();
    let total = sample_size as f64;
    let ups = moves.iter().filter(|m| **m > FLAT_BAND_BPS).count();
    let downs = moves.iter().filter(|m| **m < -FLAT_BAND_BPS).count();
    let flats = sample_size - ups - downs;
    let ppm = |count: usize| ((count as f64 / total) * 1_000_000.0).round() as u32;
    let (probability_up_ppm, probability_down_ppm, probability_flat_ppm) =
        (ppm(ups), ppm(downs), ppm(flats));

    // The call is the most frequent outcome, not the sign of the median: with a
    // skewed distribution the median can sit on the opposite side of the band
    // from where most episodes actually landed.
    let (direction, probability_ppm) = if ups >= downs && ups >= flats {
        (ForecastDirection::Up, probability_up_ppm)
    } else if downs >= ups && downs >= flats {
        (ForecastDirection::Down, probability_down_ppm)
    } else {
        (ForecastDirection::Flat, probability_flat_ppm)
    };
    let median = quantile(&moves, 0.5);

    // Climatology: the same question asked of every classified bar, whatever
    // regime it was in.
    let climatology_moves = forward_returns(bars, classification, None, horizon_bars);
    let climatology_ppm = if climatology_moves.is_empty() {
        0
    } else {
        let matching = climatology_moves
            .iter()
            .filter(|m| match direction {
                ForecastDirection::Up => **m > FLAT_BAND_BPS,
                ForecastDirection::Down => **m < -FLAT_BAND_BPS,
                ForecastDirection::Flat => m.abs() <= FLAT_BAND_BPS,
            })
            .count();
        ((matching as f64 / climatology_moves.len() as f64) * 1_000_000.0).round() as u32
    };
    let edge_ppm = i64::from(probability_ppm) - i64::from(climatology_ppm);

    let occurrences = classification
        .samples
        .iter()
        .filter(|s| s.regime == Some(regime))
        .count();
    let regime_prevalence = if classification.classified_count == 0 {
        0.0
    } else {
        occurrences as f64 / classification.classified_count as f64
    };

    Some(EmpiricalForecast {
        regime,
        horizon_bars,
        sample_size,
        probability_up_ppm,
        probability_down_ppm,
        probability_flat_ppm,
        direction,
        probability_ppm: probability_ppm.min(1_000_000),
        climatology_ppm,
        edge_ppm: i32::try_from(edge_ppm).unwrap_or(0),
        climatology_sample_size: climatology_moves.len(),
        median_move_bps: median,
        p10_move_bps: quantile(&moves, 0.10),
        p90_move_bps: quantile(&moves, 0.90),
        regime_prevalence,
        sufficient: sample_size >= MIN_SAMPLE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{classify, RegimeParams};

    fn series(closes: &[f64]) -> Vec<Bar> {
        closes
            .iter()
            .enumerate()
            .map(|(i, &c)| Bar {
                t: i as i64 * 86_400_000,
                o: c,
                h: c,
                l: c,
                c,
                v: 1.0,
            })
            .collect()
    }

    fn wobble(i: usize, scale: f64) -> f64 {
        let x = ((i as f64) * 12.9898).sin() * 43_758.545_312;
        (x - x.floor() - 0.5) * scale
    }

    #[test]
    fn a_persistently_rising_series_forecasts_up() {
        let closes: Vec<f64> = (0..500)
            .map(|i| 100.0 * (1.0 + 0.0015 * f64::from(i)) + wobble(i as usize, 0.05))
            .collect();
        let bars = series(&closes);
        let classification = classify(&bars, &RegimeParams::daily());
        let forecast = empirical_forecast(&bars, &classification, 10).expect("forecast");
        assert!(
            forecast.probability_up_ppm > 600_000,
            "expected a bullish tilt, got {}",
            forecast.probability_up_ppm,
        );
        assert_eq!(forecast.direction, ForecastDirection::Up);
        assert!(forecast.median_move_bps > 0.0);
    }

    #[test]
    fn quantiles_are_ordered() {
        let closes: Vec<f64> = (0..500)
            .map(|i| 100.0 + wobble(i as usize, 4.0) + 0.01 * f64::from(i))
            .collect();
        let bars = series(&closes);
        let classification = classify(&bars, &RegimeParams::daily());
        let forecast = empirical_forecast(&bars, &classification, 5).expect("forecast");
        assert!(forecast.p10_move_bps <= forecast.median_move_bps);
        assert!(forecast.median_move_bps <= forecast.p90_move_bps);
    }

    #[test]
    fn a_short_series_yields_no_forecast() {
        let bars = series(&(0..30).map(|i| 100.0 + f64::from(i)).collect::<Vec<_>>());
        let classification = classify(&bars, &RegimeParams::daily());
        assert!(empirical_forecast(&bars, &classification, 10).is_none());
    }

    #[test]
    fn small_samples_are_flagged_insufficient() {
        // Just past warmup: a handful of classified bars, so whatever regime is
        // current cannot have accumulated MIN_SAMPLE forward windows.
        let closes: Vec<f64> = (0..105)
            .map(|i| 100.0 * (1.0 + 0.001 * f64::from(i)) + wobble(i as usize, 0.3))
            .collect();
        let bars = series(&closes);
        let classification = classify(&bars, &RegimeParams::daily());
        if let Some(forecast) = empirical_forecast(&bars, &classification, 20) {
            assert_eq!(forecast.sufficient, forecast.sample_size >= MIN_SAMPLE);
        }
    }

    #[test]
    fn the_reported_probability_always_matches_the_stated_direction() {
        // The scoring contract: `probability_ppm` must be the probability of
        // the direction actually claimed. Getting this wrong scores a "down"
        // call against P(up) and silently corrupts every Brier statistic.
        for slope in [-0.002_f64, -0.0005, 0.0, 0.0005, 0.002] {
            let closes: Vec<f64> = (0..500)
                .map(|i| 100.0 * (1.0 + slope * f64::from(i)) + wobble(i as usize, 0.4))
                .collect();
            let bars = series(&closes);
            let classification = classify(&bars, &RegimeParams::daily());
            let Some(f) = empirical_forecast(&bars, &classification, 5) else {
                continue;
            };
            let expected = match f.direction {
                ForecastDirection::Up => f.probability_up_ppm,
                ForecastDirection::Down => f.probability_down_ppm,
                ForecastDirection::Flat => f.probability_flat_ppm,
            };
            assert_eq!(
                f.probability_ppm, expected,
                "slope {slope}: direction {:?} reported the wrong probability",
                f.direction,
            );
        }
    }

    #[test]
    fn the_three_outcome_probabilities_partition_the_sample() {
        let closes: Vec<f64> = (0..500)
            .map(|i| 100.0 + wobble(i as usize, 3.0) + 0.02 * f64::from(i))
            .collect();
        let bars = series(&closes);
        let classification = classify(&bars, &RegimeParams::daily());
        let f = empirical_forecast(&bars, &classification, 5).expect("forecast");
        let total = f.probability_up_ppm + f.probability_down_ppm + f.probability_flat_ppm;
        // Rounding three shares to ppm can drift by at most one unit each.
        assert!(
            total.abs_diff(1_000_000) <= 3,
            "probabilities summed to {total}",
        );
    }

    #[test]
    fn the_chosen_direction_is_the_most_likely_one() {
        let closes: Vec<f64> = (0..500)
            .map(|i| 100.0 * (1.0 - 0.0015 * f64::from(i)) + wobble(i as usize, 0.3))
            .collect();
        let bars = series(&closes);
        let classification = classify(&bars, &RegimeParams::daily());
        let f = empirical_forecast(&bars, &classification, 5).expect("forecast");
        assert!(
            f.probability_ppm
                >= f.probability_up_ppm
                    .max(f.probability_down_ppm)
                    .max(f.probability_flat_ppm)
        );
    }

    #[test]
    fn edge_is_the_difference_between_the_call_and_its_base_rate() {
        for slope in [-0.0015_f64, 0.0, 0.0015] {
            let closes: Vec<f64> = (0..600)
                .map(|i| 100.0 * (1.0 + slope * f64::from(i)) + wobble(i as usize, 0.5))
                .collect();
            let bars = series(&closes);
            let classification = classify(&bars, &RegimeParams::daily());
            let Some(f) = empirical_forecast(&bars, &classification, 5) else {
                continue;
            };
            assert_eq!(
                i64::from(f.edge_ppm),
                i64::from(f.probability_ppm) - i64::from(f.climatology_ppm),
                "edge must be exactly the gap over the base rate",
            );
        }
    }

    #[test]
    fn a_regime_that_explains_nothing_shows_no_edge() {
        // Regime labels that carry no information about forward returns must
        // produce an edge near zero. This is the case the whole comparison
        // exists to expose: a confident-looking probability that is pure
        // climatology. If this test can be made to pass with a large edge,
        // the edge calculation is measuring the wrong thing.
        let closes: Vec<f64> = (0..1200)
            .map(|i| {
                // Volatility cycles (which drive the regime label) are
                // deliberately decoupled from the drift (which drives returns).
                let amp = if (i / 100) % 2 == 0 { 0.3 } else { 2.0 };
                100.0 * (1.0 + 0.0004 * f64::from(i)) + wobble(i as usize, amp)
            })
            .collect();
        let bars = series(&closes);
        let classification = classify(&bars, &RegimeParams::daily());
        let f = empirical_forecast(&bars, &classification, 5).expect("forecast");
        assert!(
            f.edge_ppm.abs() < 150_000,
            "expected a small edge for an uninformative regime, got {} ppm (p={} clim={})",
            f.edge_ppm,
            f.probability_ppm,
            f.climatology_ppm,
        );
    }

    #[test]
    fn climatology_is_drawn_from_the_same_span_as_the_conditional_sample() {
        // Both samples must exclude warmup bars, or the base rate would
        // describe a different period than the forecast it is judging.
        let closes: Vec<f64> = (0..700)
            .map(|i| 100.0 * (1.0 + 0.0008 * f64::from(i)) + wobble(i as usize, 0.6))
            .collect();
        let bars = series(&closes);
        let classification = classify(&bars, &RegimeParams::daily());
        let f = empirical_forecast(&bars, &classification, 5).expect("forecast");
        assert!(
            f.climatology_sample_size >= f.sample_size,
            "climatology ({}) must be a superset of the regime sample ({})",
            f.climatology_sample_size,
            f.sample_size,
        );
        assert!(f.climatology_sample_size <= classification.classified_count);
    }

    #[test]
    fn probability_never_exceeds_one() {
        let closes: Vec<f64> = (0..400).map(|i| 100.0 + f64::from(i)).collect();
        let bars = series(&closes);
        let classification = classify(&bars, &RegimeParams::daily());
        let forecast = empirical_forecast(&bars, &classification, 3).expect("forecast");
        assert!(forecast.probability_up_ppm <= 1_000_000);
        assert!(forecast.probability_ppm <= 1_000_000);
    }
}
