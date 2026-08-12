//! Deterministic convergence of heterogeneous market signals.
//!
//! Scores and confidence use fixed-point integers. Every accepted signal must
//! carry immutable evidence, and stale observations are reported rather than
//! silently influencing a conclusion.

use prismatik_market_data::{BlindSpot, EvidenceRef};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;
use time::{Duration, OffsetDateTime};

/// Direction asserted by a signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Negative expected effect.
    Bearish,
    /// No directional expected effect.
    Neutral,
    /// Positive expected effect.
    Bullish,
}

impl Direction {
    fn sign(self) -> i64 {
        match self {
            Self::Bearish => -1,
            Self::Neutral => 0,
            Self::Bullish => 1,
        }
    }
}

/// One normalized, evidence-backed analytical observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalObservation {
    /// Stable signal identifier.
    pub id: String,
    /// Instrument, event, or portfolio key the signal describes.
    pub subject_key: String,
    /// Signal family, such as price, filings, macro, or prediction market.
    pub family: String,
    /// Expected direction.
    pub direction: Direction,
    /// Absolute strength from zero through one million.
    pub strength_ppm: u32,
    /// Calibrated confidence from zero through one million.
    pub confidence_ppm: u32,
    /// Caller-defined importance from one through one million.
    pub weight_ppm: u32,
    /// Time represented by the observation.
    pub event_time: OffsetDateTime,
    /// Immutable evidence supporting the observation.
    pub evidence: Vec<EvidenceRef>,
}

/// A deterministic synthesis of accepted observations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvergenceResult {
    /// Shared subject key.
    pub subject_key: String,
    /// Signed score from minus one million through one million.
    pub score_ppm: i32,
    /// Direction implied by the score.
    pub direction: Direction,
    /// Weighted confidence of accepted observations.
    pub confidence_ppm: u32,
    /// Number of accepted observations.
    pub accepted_signals: usize,
    /// Stable, de-duplicated evidence references.
    pub evidence: Vec<EvidenceRef>,
    /// Explicit gaps encountered during convergence.
    pub blind_spots: Vec<BlindSpot>,
    /// Caller-supplied evaluation time.
    pub evaluated_at: OffsetDateTime,
}

/// Signal validation or convergence failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ConvergenceError {
    /// No observations were supplied.
    #[error("no signals supplied")]
    NoSignals,
    /// Signals described different subjects.
    #[error("signals describe different subjects")]
    MixedSubjects,
    /// A fixed-point field exceeded one million or a weight was zero.
    #[error("invalid fixed-point value in signal {0}")]
    InvalidScale(String),
    /// A required identifier was empty.
    #[error("signal {0} has an empty required identifier")]
    EmptyIdentifier(String),
    /// Maximum age was negative.
    #[error("maximum age must not be negative")]
    NegativeMaximumAge,
}

/// Converge fresh signals for one subject using fixed-point weighted scoring.
pub fn converge_signals(
    signals: &[SignalObservation],
    evaluated_at: OffsetDateTime,
    maximum_age: Duration,
) -> Result<ConvergenceResult, ConvergenceError> {
    if maximum_age.is_negative() {
        return Err(ConvergenceError::NegativeMaximumAge);
    }
    let first = signals.first().ok_or(ConvergenceError::NoSignals)?;
    let mut numerator = 0_i128;
    let mut denominator = 0_i128;
    let mut confidence_numerator = 0_i128;
    let mut accepted_signals = 0_usize;
    let mut evidence = Vec::new();
    let mut evidence_ids = BTreeSet::new();
    let mut blind_spots = Vec::new();

    for signal in signals {
        validate(signal)?;
        if signal.subject_key != first.subject_key {
            return Err(ConvergenceError::MixedSubjects);
        }
        if signal.event_time > evaluated_at {
            blind_spots.push(BlindSpot {
                code: "future_signal".into(),
                message: format!("{} occurs after evaluation time", signal.id),
            });
            continue;
        }
        if evaluated_at - signal.event_time > maximum_age {
            blind_spots.push(BlindSpot {
                code: "stale_signal".into(),
                message: format!("{} exceeded the configured maximum age", signal.id),
            });
            continue;
        }
        if signal.evidence.is_empty() {
            blind_spots.push(BlindSpot {
                code: "missing_evidence".into(),
                message: format!("{} has no immutable evidence reference", signal.id),
            });
            continue;
        }

        let effective_weight = i128::from(signal.weight_ppm) * i128::from(signal.confidence_ppm);
        numerator +=
            signal.direction.sign() as i128 * i128::from(signal.strength_ppm) * effective_weight;
        denominator += 1_000_000_i128 * effective_weight;
        confidence_numerator += i128::from(signal.confidence_ppm) * i128::from(signal.weight_ppm);
        accepted_signals += 1;
        for reference in &signal.evidence {
            if evidence_ids.insert(reference.id.clone()) {
                evidence.push(reference.clone());
            }
        }
    }

    let score_ppm = if denominator == 0 {
        0
    } else {
        i32::try_from(numerator * 1_000_000 / denominator).expect("score is bounded")
    };
    let total_weight: i128 = signals
        .iter()
        .filter(|signal| {
            !signal.evidence.is_empty()
                && signal.event_time <= evaluated_at
                && evaluated_at - signal.event_time <= maximum_age
        })
        .map(|signal| i128::from(signal.weight_ppm))
        .sum();
    let confidence_ppm = if total_weight == 0 {
        0
    } else {
        u32::try_from(confidence_numerator / total_weight).expect("confidence is bounded")
    };
    let direction = match score_ppm.cmp(&0) {
        std::cmp::Ordering::Less => Direction::Bearish,
        std::cmp::Ordering::Equal => Direction::Neutral,
        std::cmp::Ordering::Greater => Direction::Bullish,
    };
    Ok(ConvergenceResult {
        subject_key: first.subject_key.clone(),
        score_ppm,
        direction,
        confidence_ppm,
        accepted_signals,
        evidence,
        blind_spots,
        evaluated_at,
    })
}

fn validate(signal: &SignalObservation) -> Result<(), ConvergenceError> {
    if signal.id.trim().is_empty()
        || signal.subject_key.trim().is_empty()
        || signal.family.trim().is_empty()
    {
        return Err(ConvergenceError::EmptyIdentifier(signal.id.clone()));
    }
    if signal.strength_ppm > 1_000_000
        || signal.confidence_ppm > 1_000_000
        || signal.weight_ppm == 0
        || signal.weight_ppm > 1_000_000
    {
        return Err(ConvergenceError::InvalidScale(signal.id.clone()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_domain::ProviderId;

    fn signal(id: &str, direction: Direction, event_time: OffsetDateTime) -> SignalObservation {
        SignalObservation {
            id: id.into(),
            subject_key: "event:rates".into(),
            family: "prediction_market".into(),
            direction,
            strength_ppm: 800_000,
            confidence_ppm: 750_000,
            weight_ppm: 500_000,
            event_time,
            evidence: vec![EvidenceRef {
                id: format!("evidence:{id}"),
                provider: ProviderId::KALSHI,
                retrieved_at: event_time,
            }],
        }
    }

    #[test]
    fn converges_fresh_evidence_and_reports_stale_inputs() {
        let now = OffsetDateTime::UNIX_EPOCH + Duration::hours(2);
        let result = converge_signals(
            &[
                signal("fresh", Direction::Bullish, now - Duration::minutes(5)),
                signal("stale", Direction::Bearish, OffsetDateTime::UNIX_EPOCH),
            ],
            now,
            Duration::hours(1),
        )
        .unwrap();
        assert_eq!(result.direction, Direction::Bullish);
        assert_eq!(result.score_ppm, 800_000);
        assert_eq!(result.accepted_signals, 1);
        assert_eq!(result.blind_spots[0].code, "stale_signal");
    }

    #[test]
    fn excludes_signals_without_evidence() {
        let mut unsupported = signal(
            "unsupported",
            Direction::Bullish,
            OffsetDateTime::UNIX_EPOCH,
        );
        unsupported.evidence.clear();
        let result = converge_signals(
            &[unsupported],
            OffsetDateTime::UNIX_EPOCH,
            Duration::hours(1),
        )
        .unwrap();
        assert_eq!(result.score_ppm, 0);
        assert_eq!(result.accepted_signals, 0);
        assert_eq!(result.blind_spots[0].code, "missing_evidence");
    }
}
