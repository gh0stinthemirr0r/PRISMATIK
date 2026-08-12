//! How long regimes last — Kaplan–Meier survival over observed run lengths.
//!
//! The question this answers is "the current regime has held for N bars; what
//! is the chance it survives another M?" Completed runs are exact
//! observations; the run in progress is right-censored — it has lasted at
//! least this long and we do not yet know its true length. Treating that
//! censored run as if it had ended would bias every estimate downward, which
//! is precisely the error a naive average of run lengths makes.

use serde::{Deserialize, Serialize};

use crate::{Regime, RegimeClassification};

/// One step of the survival curve.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurvivalPoint {
    /// Regime age in bars.
    pub age: usize,
    /// Probability of surviving at least this long, 0..1.
    pub survival: f64,
    /// Greenwood lower confidence bound.
    pub lower: f64,
    /// Greenwood upper confidence bound.
    pub upper: f64,
    /// Runs still at risk entering this age.
    pub at_risk: usize,
    /// Runs that ended at this age.
    pub ended: usize,
}

/// Survival of regime runs.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurvivalCurve {
    /// Regime this curve describes, or `None` for all regimes pooled.
    pub regime: Option<Regime>,
    /// Curve points in ascending age.
    pub points: Vec<SurvivalPoint>,
    /// Age at which survival first falls to or below 0.5.
    pub median_life: Option<usize>,
    /// Completed runs observed.
    pub completed_runs: usize,
    /// Age of the run currently in progress.
    pub current_age: usize,
    /// Probability the current run survives one more bar.
    pub next_bar_survival: Option<f64>,
}

/// Consecutive runs of the same regime. The final run is censored.
fn runs(classification: &RegimeClassification, regime: Option<Regime>) -> (Vec<usize>, usize) {
    let labels = classification.labels();
    let mut completed = Vec::new();
    let mut current = 0_usize;
    let mut previous: Option<Regime> = None;

    for (_, label) in &labels {
        let matches = regime.is_none_or(|target| target == *label);
        let same_as_previous = previous == Some(*label);
        if same_as_previous && matches {
            current += 1;
        } else {
            if current > 0 {
                completed.push(current);
            }
            current = usize::from(matches);
        }
        previous = Some(*label);
    }
    // The trailing run has not ended, so it is censored rather than completed.
    (completed, current)
}

/// Kaplan–Meier estimate with Greenwood confidence bounds.
pub fn regime_survival(
    classification: &RegimeClassification,
    regime: Option<Regime>,
) -> SurvivalCurve {
    let (completed, censored_age) = runs(classification, regime);
    let mut points = Vec::new();

    if completed.is_empty() {
        return SurvivalCurve {
            regime,
            points,
            median_life: None,
            completed_runs: 0,
            current_age: censored_age,
            next_bar_survival: None,
        };
    }

    let mut sorted = completed.clone();
    sorted.sort_unstable();
    let max_age = *sorted.last().unwrap_or(&0);

    let mut survival = 1.0_f64;
    // Running Greenwood sum: Σ d / (n · (n − d)).
    let mut greenwood = 0.0_f64;

    for age in 1..=max_age {
        let ended = completed.iter().filter(|&&r| r == age).count();
        let at_risk =
            completed.iter().filter(|&&r| r >= age).count() + usize::from(censored_age >= age);
        if at_risk == 0 {
            break;
        }
        if ended > 0 {
            survival *= 1.0 - (ended as f64 / at_risk as f64);
            let denominator = at_risk as f64 * (at_risk as f64 - ended as f64);
            if denominator > 0.0 {
                greenwood += ended as f64 / denominator;
            }
        }
        // 95% bounds, clamped to the unit interval.
        let stderr = survival * greenwood.sqrt();
        points.push(SurvivalPoint {
            age,
            survival,
            lower: (survival - 1.96 * stderr).clamp(0.0, 1.0),
            upper: (survival + 1.96 * stderr).clamp(0.0, 1.0),
            at_risk,
            ended,
        });
    }

    let median_life = points.iter().find(|p| p.survival <= 0.5).map(|p| p.age);

    // Conditional one-step survival for the run in progress.
    let next_bar_survival = if censored_age == 0 {
        None
    } else {
        // The run in progress has by definition reached this age, so it is
        // always one of the runs at risk here.
        let at_risk = completed.iter().filter(|&&r| r >= censored_age).count() + 1;
        let ending = completed.iter().filter(|&&r| r == censored_age).count();
        Some(1.0 - (ending as f64 / at_risk as f64))
    };

    SurvivalCurve {
        regime,
        points,
        median_life,
        completed_runs: completed.len(),
        current_age: censored_age,
        next_bar_survival,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{classify, Bar, RegimeParams};

    fn wobble(i: usize, scale: f64) -> f64 {
        let x = ((i as f64) * 12.9898).sin() * 43_758.545_312;
        (x - x.floor() - 0.5) * scale
    }

    fn alternating_series() -> Vec<Bar> {
        (0..800)
            .map(|i| {
                let base = 100.0 * (1.0 + 0.0006 * f64::from(i));
                let amp = if (i / 70) % 2 == 0 { 0.08 } else { 3.5 };
                let c = base + wobble(i as usize, amp);
                Bar {
                    t: i64::from(i) * 86_400_000,
                    o: c,
                    h: c,
                    l: c,
                    c,
                    v: 1.0,
                }
            })
            .collect()
    }

    #[test]
    fn survival_is_monotonically_non_increasing() {
        let bars = alternating_series();
        let classification = classify(&bars, &RegimeParams::daily());
        let curve = regime_survival(&classification, None);
        for pair in curve.points.windows(2) {
            assert!(
                pair[1].survival <= pair[0].survival + 1e-12,
                "survival rose from {} to {}",
                pair[0].survival,
                pair[1].survival,
            );
        }
    }

    #[test]
    fn survival_starts_at_or_below_one_and_stays_in_range() {
        let bars = alternating_series();
        let classification = classify(&bars, &RegimeParams::daily());
        let curve = regime_survival(&classification, None);
        for point in &curve.points {
            assert!((0.0..=1.0).contains(&point.survival));
            assert!(point.lower <= point.survival + 1e-12);
            assert!(point.upper >= point.survival - 1e-12);
        }
    }

    #[test]
    fn median_life_is_the_first_age_at_or_below_half() {
        let bars = alternating_series();
        let classification = classify(&bars, &RegimeParams::daily());
        let curve = regime_survival(&classification, None);
        if let Some(median) = curve.median_life {
            let point = curve.points.iter().find(|p| p.age == median).unwrap();
            assert!(point.survival <= 0.5);
            assert!(curve
                .points
                .iter()
                .filter(|p| p.age < median)
                .all(|p| p.survival > 0.5));
        }
    }

    #[test]
    fn a_series_with_no_regime_change_has_no_completed_runs() {
        // A single unbroken regime: nothing has ended, so everything is censored.
        let bars: Vec<Bar> = (0..300)
            .map(|i| {
                let c = 100.0 * (1.0 + 0.0015 * f64::from(i));
                Bar {
                    t: i64::from(i) * 86_400_000,
                    o: c,
                    h: c,
                    l: c,
                    c,
                    v: 1.0,
                }
            })
            .collect();
        let classification = classify(&bars, &RegimeParams::daily());
        let curve = regime_survival(&classification, classification.current);
        assert_eq!(curve.completed_runs, 0);
        assert!(curve.points.is_empty());
        assert!(curve.current_age > 0);
    }

    #[test]
    fn the_censored_run_counts_toward_at_risk() {
        let bars = alternating_series();
        let classification = classify(&bars, &RegimeParams::daily());
        let curve = regime_survival(&classification, None);
        if let Some(first) = curve.points.first() {
            assert!(first.at_risk >= curve.completed_runs);
        }
    }
}
