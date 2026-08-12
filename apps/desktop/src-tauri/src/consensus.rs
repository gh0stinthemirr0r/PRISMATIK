//! Consensus — what three independent estimators say about the same question.
//!
//! PRISMATIK now carries three forecasters that reach a directional claim by
//! genuinely different routes:
//!
//! - **Regime** reasons from history: what happened the last time this
//!   instrument was in this state.
//! - **Tape** reasons from sequence: what a transformer trained on 12B bars
//!   continues this series into.
//! - **Crowd** reasons from population: what a swarm of seeded agents talks
//!   itself into.
//!
//! Their independence is the point. Three estimators sharing a method would
//! agree for uninteresting reasons; these three can only agree because the
//! same signal is visible from three directions, and can only disagree
//! because at least one of them is reading something the others cannot.
//!
//! **This module states agreement; it does not forecast.** It files nothing,
//! and it deliberately does not blend the three into a single probability.
//! A blended number would look like a fourth, better forecast while having no
//! cohort, no resolution and no measured skill — precisely the kind of
//! authoritative-looking figure the rest of this codebase exists to refuse.
//! What it reports instead is the shape of the agreement, and each
//! estimator's *own* measured standing, so the reader can weight them
//! knowing which have earned it.
//!
//! Disagreement is not noise to be averaged away. Two estimators splitting on
//! the same instrument is a fact about the instrument, and it is reported as
//! one.

use serde::Serialize;

use crate::forecast_candidates::{self, CROWD_PROVIDER, EMPIRICAL_PROVIDER, TAPE_PROVIDER};

/// One estimator's current open claim on a target and horizon.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EstimatorView {
    /// Display name: "Regime", "Tape", "Crowd".
    pub(crate) name: String,
    pub(crate) provider_id: String,
    pub(crate) cohort_model: String,
    pub(crate) direction: String,
    /// Probability of the stated direction.
    pub(crate) probability_ppm: u32,
    /// Base rate for that same direction.
    pub(crate) climatology_ppm: u32,
    /// Signed edge over the base rate.
    pub(crate) edge_ppm: i64,
    /// Measured Brier skill over climatology, or `None` when unresolved.
    pub(crate) skill_ppm: Option<i64>,
    pub(crate) resolved_count: usize,
    /// Plain-language standing, so an unproven estimator is not read as a
    /// proven one that happens to be quiet.
    pub(crate) standing: String,
    pub(crate) summary: String,
    pub(crate) generated_at: String,
}

/// The agreement picture for one target and horizon.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConsensusView {
    pub(crate) target: String,
    pub(crate) horizon_days: usize,
    pub(crate) estimators: Vec<EstimatorView>,
    /// Estimators that filed a claim, of three.
    pub(crate) reporting: usize,
    /// Distinct directions among them.
    pub(crate) distinct_directions: usize,
    /// True only when every reporting estimator names the same direction.
    pub(crate) unanimous: bool,
    /// One line naming the state, including what is missing.
    pub(crate) verdict: String,
    /// Estimators with a positive measured skill record, of those reporting.
    pub(crate) proven_reporting: usize,
}

fn direction_label(direction: &str) -> &str {
    match direction {
        "up" => "up",
        "down" => "down",
        _ => "flat",
    }
}

fn standing_for(skill_ppm: Option<i64>, resolved: usize) -> String {
    match (skill_ppm, resolved) {
        (_, 0) => "unproven — no resolved forecasts yet".to_owned(),
        (Some(skill), n) if skill > 0 => {
            format!(
                "{:+.1} points of skill over {n} resolved",
                skill as f64 / 10_000.0
            )
        },
        (Some(skill), n) => format!(
            "{:+.1} points — worse than the base rate over {n} resolved",
            skill as f64 / 10_000.0
        ),
        (None, n) => format!("{n} resolved, skill not yet computable"),
    }
}

/// Build the agreement picture for one target and horizon.
///
/// Only open, unresolved claims count. A resolved candidate is history, and
/// showing it beside live ones would misreport what the desk currently
/// believes.
#[tauri::command]
pub(crate) fn consensus_for(target: String, horizon_days: usize) -> Result<ConsensusView, String> {
    let horizon_minutes = (horizon_days as u32).saturating_mul(24 * 60);
    let mut estimators = Vec::new();
    for (provider_id, name) in [
        (EMPIRICAL_PROVIDER, "Regime"),
        (TAPE_PROVIDER, "Tape"),
        (CROWD_PROVIDER, "Crowd"),
    ] {
        let Some(claim) =
            forecast_candidates::latest_open_claim(provider_id, &target, horizon_minutes)
        else {
            continue;
        };

        let climatology_ppm = claim.climatology_ppm;
        let skill = forecast_candidates::estimator_skill(provider_id, &claim.model, horizon_days);
        // `CohortSkill::skill_ppm` is already optional — a cohort can have
        // resolutions but too few to compute skill from — so flatten rather
        // than wrapping it a second time.
        let (skill_ppm, resolved_count) = skill
            .map(|s| (s.skill_ppm, s.sample_count))
            .unwrap_or((None, 0));

        estimators.push(EstimatorView {
            name: name.to_owned(),
            provider_id: provider_id.to_owned(),
            cohort_model: claim.model.clone(),
            direction: direction_label(&claim.direction).to_owned(),
            probability_ppm: claim.probability_ppm,
            climatology_ppm,
            edge_ppm: i64::from(claim.probability_ppm) - i64::from(climatology_ppm),
            skill_ppm,
            resolved_count,
            standing: standing_for(skill_ppm, resolved_count),
            summary: claim.summary.clone(),
            generated_at: claim.generated_at.clone(),
        });
    }

    let reporting = estimators.len();
    let mut directions: Vec<&str> = estimators.iter().map(|e| e.direction.as_str()).collect();
    directions.sort_unstable();
    directions.dedup();
    let distinct_directions = directions.len();
    let unanimous = reporting > 1 && distinct_directions == 1;
    let proven_reporting = estimators
        .iter()
        .filter(|e| e.skill_ppm.is_some_and(|s| s > 0) && e.resolved_count > 0)
        .count();

    // The verdict always names how many estimators are silent. "Two agree"
    // reads very differently from "two agree and the third has nothing to
    // say", and the second is what is actually true.
    let silent = 3 - reporting;
    let verdict = match (reporting, distinct_directions) {
        (0, _) => format!("No estimator has an open {horizon_days}d claim on {target}."),
        (1, _) => format!(
            "Only {} is reporting; {silent} of three are silent, so there is no agreement to read.",
            estimators[0].name
        ),
        (_, 1) => format!(
            "All {reporting} reporting estimators say {}{}.",
            estimators[0].direction,
            if silent > 0 {
                format!(", with {silent} silent")
            } else {
                String::new()
            }
        ),
        (_, n) => format!(
            "{reporting} estimators split across {n} directions{}. The disagreement is the finding.",
            if silent > 0 {
                format!(", with {silent} silent")
            } else {
                String::new()
            }
        ),
    };

    Ok(ConsensusView {
        target,
        horizon_days,
        estimators,
        reporting,
        distinct_directions,
        unanimous,
        verdict,
        proven_reporting,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standing_never_reads_as_proven_before_anything_resolves() {
        // The dangerous case: a large positive skill number attached to zero
        // resolutions. Resolution count wins, whatever the skill claims.
        assert_eq!(
            standing_for(Some(400_000), 0),
            "unproven — no resolved forecasts yet"
        );
        assert!(standing_for(None, 0).contains("unproven"));
    }

    #[test]
    fn negative_skill_is_stated_plainly() {
        let text = standing_for(Some(-120_000), 44);
        assert!(text.contains("worse than the base rate"), "{text}");
        assert!(text.contains("44"), "{text}");
    }

    #[test]
    fn positive_skill_reports_its_sample() {
        let text = standing_for(Some(85_000), 60);
        assert!(text.contains("+8.5"), "{text}");
        assert!(text.contains("60"), "{text}");
    }

    #[test]
    fn unrecognised_directions_fall_back_to_flat() {
        // A direction string that is neither up nor down must not be silently
        // rendered as one of them.
        assert_eq!(direction_label("up"), "up");
        assert_eq!(direction_label("down"), "down");
        assert_eq!(direction_label("sideways"), "flat");
        assert_eq!(direction_label(""), "flat");
    }
}
