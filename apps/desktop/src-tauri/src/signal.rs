//! The trading signal: what PRISMATIK actually believes about an instrument.
//!
//! This module exists because the autonomous trader previously had no view at
//! all — it went long on everything the risk gate permitted, which meant the
//! risk layer was doing all the work and the "trader" contributed nothing.
//!
//! A signal here is deliberately hard to earn. It requires a classified regime,
//! a large enough conditional sample, and — critically — an *edge over
//! climatology*, because a 58% probability against a 58% base rate is not a
//! view, it is a restatement of what the instrument does anyway. Conviction is
//! then scaled by the cohort's **measured** skill, not by the model's stated
//! confidence, so a forecaster that has never beaten the base rate cannot size
//! a position on self-assurance.

use serde::{Deserialize, Serialize};

use crate::{
    analytics,
    forecast_candidates::{self, CohortSkill},
    tracking::InstrumentKind,
};

/// Horizon, in trading days, the trader acts on.
///
/// Long enough for a directional view to be meaningful, short enough that
/// forecasts resolve often and the cohort accumulates a score.
pub(crate) const DECISION_HORIZON_DAYS: usize = 5;

/// Minimum |edge| over climatology, in ppm, before a view is actionable.
///
/// 2 percentage points. Below this the conditional and unconditional
/// distributions are not distinguishable at the sample sizes involved.
pub(crate) const MIN_EDGE_PPM: i32 = 20_000;

/// Cap on how much measured skill can scale a position.
const MAX_SKILL_MULTIPLIER: f64 = 1.0;

/// Multiplier applied when the cohort has no score yet.
///
/// Deliberately small rather than zero: an unproven forecaster must be able to
/// take positions small enough to generate the resolved outcomes that would
/// prove or disprove it, but never large enough to matter if it is wrong.
const UNPROVEN_MULTIPLIER: f64 = 0.25;

/// Direction of an actionable view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Side {
    Long,
    Short,
}

/// The trader's view on one instrument.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstrumentSignal {
    pub(crate) symbol: String,
    pub(crate) regime: Option<String>,
    pub(crate) regime_run_length: usize,
    /// Direction, or `None` when the evidence does not support acting.
    pub(crate) side: Option<Side>,
    /// Probability of the stated direction, in ppm.
    pub(crate) probability_ppm: u32,
    /// Unconditional base rate for the same claim, in ppm.
    pub(crate) climatology_ppm: u32,
    /// Signed gap between the two. The part that is information.
    pub(crate) edge_ppm: i32,
    /// Historical episodes behind the conditional estimate.
    pub(crate) sample_size: usize,
    /// Measured skill of this cohort, once it has resolved forecasts.
    pub(crate) skill_ppm: Option<i64>,
    pub(crate) skill_sample_count: usize,
    /// Position-size multiplier in 0..=1 derived from measured skill.
    pub(crate) conviction: f64,
    /// Whether this signal may open a position at all.
    pub(crate) actionable: bool,
    /// Why, in the operator's language. Always populated, including on abstain.
    pub(crate) rationale: String,
}

impl InstrumentSignal {
    pub(crate) fn abstain(symbol: &str, rationale: impl Into<String>) -> Self {
        Self {
            symbol: symbol.to_owned(),
            regime: None,
            regime_run_length: 0,
            side: None,
            probability_ppm: 0,
            climatology_ppm: 0,
            edge_ppm: 0,
            sample_size: 0,
            skill_ppm: None,
            skill_sample_count: 0,
            conviction: 0.0,
            actionable: false,
            rationale: rationale.into(),
        }
    }
}

/// Convert measured skill into a position-size multiplier.
///
/// A cohort that has been *scored and lost* to the base rate is cut to zero: it
/// has demonstrated negative information, and sizing it down rather than off
/// would be paying to keep being wrong.
fn conviction_from_skill(skill: Option<CohortSkill>) -> (f64, String) {
    match skill {
        Some(CohortSkill {
            skill_ppm: Some(ppm),
            sample_count,
            state,
        }) if sample_count >= 20 => {
            if ppm <= 0 {
                (
                    0.0,
                    format!(
                        "cohort has negative measured skill ({:.1}% over {sample_count} resolved) — sized to zero",
                        ppm as f64 / 10_000.0
                    ),
                )
            } else {
                // Full size at +20% skill; linear below that.
                let multiplier = ((ppm as f64 / 200_000.0).clamp(0.0, MAX_SKILL_MULTIPLIER)).max(0.1);
                (
                    multiplier,
                    format!(
                        "cohort skill {:.1}% over {sample_count} resolved ({state}) — size x{multiplier:.2}",
                        ppm as f64 / 10_000.0
                    ),
                )
            }
        },
        Some(CohortSkill { sample_count, .. }) => (
            UNPROVEN_MULTIPLIER,
            format!(
                "cohort not yet scored ({sample_count} resolved) — probe size x{UNPROVEN_MULTIPLIER:.2}"
            ),
        ),
        None => (
            UNPROVEN_MULTIPLIER,
            format!("no scored history for this cohort — probe size x{UNPROVEN_MULTIPLIER:.2}"),
        ),
    }
}

/// Derive the trader's view on one instrument from real price history.
pub(crate) async fn evaluate(
    symbol: &str,
    kind: InstrumentKind,
    provider_id: &str,
) -> InstrumentSignal {
    let analysis = match analytics::analyze_instrument(
        kind,
        provider_id.to_owned(),
        symbol.to_owned(),
    )
    .await
    {
        Ok(analysis) => analysis,
        Err(error) => return InstrumentSignal::abstain(symbol, format!("no analysis: {error}")),
    };

    let Some(regime) = analysis.current_regime.clone() else {
        return InstrumentSignal::abstain(symbol, analysis.message);
    };

    let Some(forecast) = analysis
        .forecasts
        .iter()
        .find(|f| f.horizon_bars == DECISION_HORIZON_DAYS)
    else {
        return InstrumentSignal::abstain(
            symbol,
            format!("no {DECISION_HORIZON_DAYS}-day forecast for the current regime"),
        );
    };

    let skill = forecast_candidates::empirical_skill(symbol, DECISION_HORIZON_DAYS);
    let (conviction, skill_note) = conviction_from_skill(skill);

    let mut signal = InstrumentSignal {
        symbol: symbol.to_owned(),
        regime: Some(regime.clone()),
        regime_run_length: analysis.current_run_length,
        side: None,
        probability_ppm: forecast.probability_ppm,
        climatology_ppm: forecast.climatology_ppm,
        edge_ppm: forecast.edge_ppm,
        sample_size: forecast.sample_size,
        skill_ppm: skill.and_then(|s| s.skill_ppm),
        skill_sample_count: skill.map_or(0, |s| s.sample_count),
        conviction,
        actionable: false,
        rationale: String::new(),
    };

    // Each gate states its own reason, so an abstain is always explainable.
    if !forecast.sufficient {
        signal.rationale = format!(
            "only {} historical episodes of {regime} — too few to act on",
            forecast.sample_size
        );
        return signal;
    }
    if signal.edge_ppm.abs() < MIN_EDGE_PPM {
        signal.rationale = format!(
            "{:.1}% vs a {:.1}% base rate — edge of {:+.1}pp is inside the noise band",
            f64::from(forecast.probability_ppm) / 10_000.0,
            f64::from(forecast.climatology_ppm) / 10_000.0,
            f64::from(signal.edge_ppm) / 10_000.0,
        );
        return signal;
    }
    if conviction <= 0.0 {
        signal.rationale = skill_note;
        return signal;
    }

    // The edge carries the direction: a positive edge on a "down" call is a
    // short, not a long. Reading direction off the raw probability instead
    // would take positions the evidence does not support.
    let side = match (forecast.direction, signal.edge_ppm > 0) {
        (prismatik_regime::ForecastDirection::Up, true) => Some(Side::Long),
        (prismatik_regime::ForecastDirection::Down, true) => Some(Side::Short),
        // A negative edge means the regime made this call *worse* than the base
        // rate. That is evidence against the call, not evidence for its
        // opposite, so it is never inverted into a trade.
        _ => None,
    };

    let Some(side) = side else {
        signal.rationale = format!(
            "{:?} call with a {:+.1}pp edge — a negative or flat edge is not a tradeable view",
            forecast.direction,
            f64::from(signal.edge_ppm) / 10_000.0,
        );
        return signal;
    };

    signal.side = Some(side);
    signal.actionable = true;
    signal.rationale = format!(
        "{regime} held {} bars; {:?} {:.1}% vs {:.1}% base ({:+.1}pp edge, n={}); {skill_note}",
        analysis.current_run_length,
        forecast.direction,
        f64::from(forecast.probability_ppm) / 10_000.0,
        f64::from(forecast.climatology_ppm) / 10_000.0,
        f64::from(signal.edge_ppm) / 10_000.0,
        forecast.sample_size,
    );
    signal
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(ppm: Option<i64>, sample_count: usize) -> Option<CohortSkill> {
        Some(CohortSkill {
            skill_ppm: ppm,
            sample_count,
            state: "stable",
        })
    }

    #[test]
    fn a_cohort_that_lost_to_the_base_rate_is_sized_to_zero() {
        let (conviction, note) = conviction_from_skill(skill(Some(-50_000), 40));
        assert_eq!(conviction, 0.0);
        assert!(note.contains("negative measured skill"));
    }

    #[test]
    fn an_unscored_cohort_gets_a_probe_size_not_a_full_one() {
        let (conviction, _) = conviction_from_skill(None);
        assert_eq!(conviction, UNPROVEN_MULTIPLIER);
        assert!(conviction > 0.0 && conviction < 1.0);
    }

    #[test]
    fn a_thin_sample_is_treated_as_unproven_however_good_the_score() {
        // Fewer than 20 resolved forecasts cannot establish skill, even if the
        // few that resolved happened to look excellent.
        let (conviction, note) = conviction_from_skill(skill(Some(900_000), 3));
        assert_eq!(conviction, UNPROVEN_MULTIPLIER);
        assert!(note.contains("not yet scored"));
    }

    #[test]
    fn skill_scales_conviction_monotonically_and_is_capped() {
        let (low, _) = conviction_from_skill(skill(Some(20_000), 40));
        let (mid, _) = conviction_from_skill(skill(Some(100_000), 40));
        let (high, _) = conviction_from_skill(skill(Some(900_000), 40));
        assert!(low < mid, "{low} !< {mid}");
        assert!(mid < high || (mid - high).abs() < f64::EPSILON);
        assert!(high <= MAX_SKILL_MULTIPLIER);
    }

    #[test]
    fn an_abstaining_signal_always_explains_itself() {
        let signal = InstrumentSignal::abstain("SPY", "no analysis: provider down");
        assert!(!signal.actionable);
        assert!(signal.side.is_none());
        assert!(!signal.rationale.is_empty());
        assert_eq!(signal.conviction, 0.0);
    }
}
