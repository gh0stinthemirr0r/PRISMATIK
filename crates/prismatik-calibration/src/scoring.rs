//! Fixed-point binary forecast scoring and sample-size-aware participant skill.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const SCALE: u64 = 1_000_000;

/// One resolved binary forecast.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForecastObservation {
    /// Stable forecaster or public-address identifier.
    pub participant_id: String,
    /// Event category.
    pub category: String,
    /// Horizon bucket such as `1h`, `1d`, or `30d`.
    pub horizon: String,
    /// Forecast probability in parts per million.
    pub probability_ppm: u32,
    /// Realized binary outcome.
    pub outcome: bool,
}

/// One reliability-diagram bucket.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationBin {
    /// Inclusive lower probability boundary.
    pub lower_ppm: u32,
    /// Exclusive upper boundary, except certainty in the last bin.
    pub upper_ppm: u32,
    /// Number of forecasts in the bucket.
    pub count: u64,
    /// Mean predicted probability.
    pub mean_prediction_ppm: u32,
    /// Realized positive rate.
    pub realized_rate_ppm: u32,
}

/// Aggregate fixed-point calibration report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationReport {
    /// Mean Brier score scaled to one million.
    pub brier_ppm: u32,
    /// Mean absolute calibration error weighted by bin count.
    pub expected_calibration_error_ppm: u32,
    /// Reliability bins.
    pub bins: Vec<CalibrationBin>,
}

/// Sample-size-aware participant skill estimate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantSkill {
    /// Stable participant identifier.
    pub participant_id: String,
    /// Resolved sample count.
    pub sample_count: u64,
    /// Raw skill (`1 - Brier`) in parts per million.
    pub raw_skill_ppm: u32,
    /// Skill shrunk toward the supplied population prior.
    pub shrunk_skill_ppm: u32,
    /// Conservative lower bound based on effective sample weight.
    pub lower_bound_ppm: u32,
    /// Reports broken down by category and horizon.
    pub cohorts: BTreeMap<String, CalibrationReport>,
}

/// Produce a deterministic reliability report.
pub fn calibration_report(
    observations: &[ForecastObservation],
    bin_count: u32,
) -> Option<CalibrationReport> {
    if observations.is_empty() || bin_count == 0 {
        return None;
    }
    let mut sum_squared = 0_u128;
    let mut buckets = vec![(0_u64, 0_u128, 0_u64); usize::try_from(bin_count).ok()?];
    for observation in observations {
        if observation.probability_ppm > SCALE as u32 {
            return None;
        }
        let target = if observation.outcome { SCALE as i64 } else { 0 };
        let error = i64::from(observation.probability_ppm) - target;
        sum_squared = sum_squared.saturating_add((error * error) as u128);
        let index = ((u64::from(observation.probability_ppm) * u64::from(bin_count)) / (SCALE + 1))
            .min(u64::from(bin_count - 1));
        let bucket = &mut buckets[usize::try_from(index).ok()?];
        bucket.0 += 1;
        bucket.1 += u128::from(observation.probability_ppm);
        bucket.2 += u64::from(observation.outcome);
    }
    let total = observations.len() as u128;
    let brier_ppm = u32::try_from(sum_squared / total / u128::from(SCALE)).ok()?;
    let width = SCALE / u64::from(bin_count);
    let mut weighted_error = 0_u128;
    let bins = buckets
        .into_iter()
        .enumerate()
        .map(|(index, (count, prediction_sum, positives))| {
            let mean = if count == 0 {
                0
            } else {
                (prediction_sum / u128::from(count)) as u32
            };
            let realized = if count == 0 {
                0
            } else {
                (positives * SCALE / count) as u32
            };
            weighted_error += u128::from(mean.abs_diff(realized)) * u128::from(count);
            CalibrationBin {
                lower_ppm: u32::try_from(index as u64 * width).unwrap_or(1_000_000),
                upper_ppm: if index + 1 == bin_count as usize {
                    1_000_000
                } else {
                    u32::try_from((index as u64 + 1) * width).unwrap_or(1_000_000)
                },
                count,
                mean_prediction_ppm: mean,
                realized_rate_ppm: realized,
            }
        })
        .collect();
    Some(CalibrationReport {
        brier_ppm,
        expected_calibration_error_ppm: u32::try_from(weighted_error / total).ok()?,
        bins,
    })
}

/// Estimate participant skill with deterministic empirical-Bayes shrinkage.
pub fn participant_skill(
    participant_id: &str,
    observations: &[ForecastObservation],
    prior_skill_ppm: u32,
    prior_weight: u64,
) -> Option<ParticipantSkill> {
    let owned = observations
        .iter()
        .filter(|observation| observation.participant_id == participant_id)
        .cloned()
        .collect::<Vec<_>>();
    let report = calibration_report(&owned, 10)?;
    let raw_skill = 1_000_000_u32.saturating_sub(report.brier_ppm);
    let count = owned.len() as u64;
    let denominator = count.saturating_add(prior_weight).max(1);
    let shrunk = (u128::from(raw_skill) * u128::from(count)
        + u128::from(prior_skill_ppm.min(1_000_000)) * u128::from(prior_weight))
        / u128::from(denominator);
    let uncertainty = 500_000_u64 / integer_sqrt(count.max(1));
    let mut grouped = BTreeMap::<String, Vec<ForecastObservation>>::new();
    for observation in owned {
        grouped
            .entry(format!("{}::{}", observation.category, observation.horizon))
            .or_default()
            .push(observation);
    }
    let cohorts = grouped
        .into_iter()
        .filter_map(|(key, values)| calibration_report(&values, 10).map(|report| (key, report)))
        .collect();
    let shrunk_skill_ppm = u32::try_from(shrunk).ok()?;
    Some(ParticipantSkill {
        participant_id: participant_id.into(),
        sample_count: count,
        raw_skill_ppm: raw_skill,
        shrunk_skill_ppm,
        lower_bound_ppm: shrunk_skill_ppm.saturating_sub(u32::try_from(uncertainty).ok()?),
        cohorts,
    })
}

fn integer_sqrt(value: u64) -> u64 {
    let mut result = 0_u64;
    while (result + 1).saturating_mul(result + 1) <= value {
        result += 1;
    }
    result.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(probability_ppm: u32, outcome: bool) -> ForecastObservation {
        ForecastObservation {
            participant_id: "wallet".into(),
            category: "macro".into(),
            horizon: "30d".into(),
            probability_ppm,
            outcome,
        }
    }

    #[test]
    fn perfect_forecasts_have_zero_brier() {
        let report =
            calibration_report(&[observation(1_000_000, true), observation(0, false)], 10).unwrap();
        assert_eq!(report.brier_ppm, 0);
    }

    #[test]
    fn small_samples_are_shrunk_and_expose_lower_bound() {
        let skill =
            participant_skill("wallet", &[observation(1_000_000, true)], 500_000, 20).unwrap();
        assert!(skill.shrunk_skill_ppm < skill.raw_skill_ppm);
        assert!(skill.lower_bound_ppm <= skill.shrunk_skill_ppm);
    }
}
