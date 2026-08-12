//! Bounded, deterministic alert evaluation for converged signals.

use crate::signal_convergence::ConvergenceResult;
use prismatik_market_data::EvidenceRef;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

/// User-facing alert severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    /// Context worth recording without interrupting the user.
    Informational,
    /// Material change worth reviewing.
    Warning,
    /// High-confidence threshold breach requiring prompt attention.
    Critical,
}

/// Deterministic alert policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertPolicy {
    /// Stable policy identifier.
    pub id: String,
    /// Minimum absolute convergence score required to emit.
    pub minimum_score_ppm: u32,
    /// Minimum calibrated confidence required to emit.
    pub minimum_confidence_ppm: u32,
    /// Suppression interval for the same policy and subject.
    pub cooldown_seconds: i64,
    /// Maximum emissions per rolling 24-hour window.
    pub maximum_per_day: u32,
    /// Emitted severity.
    pub severity: AlertSeverity,
    /// Whether the policy participates in evaluation.
    pub enabled: bool,
}

/// Minimal immutable record used to enforce alert budgets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorAlert {
    /// Policy that emitted the alert.
    pub policy_id: String,
    /// Subject that emitted the alert.
    pub subject_key: String,
    /// Caller-supplied emission time.
    pub emitted_at: OffsetDateTime,
}

/// Result of evaluating one alert policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum AlertDecision {
    /// Policy emitted an evidence-linked alert.
    Emit {
        /// Stable idempotency key for downstream delivery.
        dedup_key: String,
        /// Alert severity.
        severity: AlertSeverity,
        /// Evidence inherited from the converged conclusion.
        evidence: Vec<EvidenceRef>,
    },
    /// Policy was disabled.
    Disabled,
    /// Score or confidence did not reach the policy threshold.
    BelowThreshold,
    /// A matching alert occurred inside the cooldown interval.
    Cooldown,
    /// The rolling daily notification budget was exhausted.
    DailyBudgetExhausted,
    /// Policy configuration was invalid.
    InvalidPolicy,
}

/// Evaluate an alert without ambient clocks or hidden process state.
pub fn evaluate_alert(
    policy: &AlertPolicy,
    convergence: &ConvergenceResult,
    history: &[PriorAlert],
) -> AlertDecision {
    if !policy.enabled {
        return AlertDecision::Disabled;
    }
    if policy.id.trim().is_empty()
        || policy.minimum_score_ppm > 1_000_000
        || policy.minimum_confidence_ppm > 1_000_000
        || policy.cooldown_seconds < 0
        || policy.maximum_per_day == 0
    {
        return AlertDecision::InvalidPolicy;
    }
    if convergence.score_ppm.unsigned_abs() < policy.minimum_score_ppm
        || convergence.confidence_ppm < policy.minimum_confidence_ppm
        || convergence.evidence.is_empty()
    {
        return AlertDecision::BelowThreshold;
    }

    let matching = history.iter().filter(|alert| {
        alert.policy_id == policy.id
            && alert.subject_key == convergence.subject_key
            && alert.emitted_at <= convergence.evaluated_at
    });
    let day_start = convergence.evaluated_at - Duration::hours(24);
    let mut daily_count = 0_u32;
    let mut latest = None;
    for alert in matching {
        if alert.emitted_at >= day_start {
            daily_count = daily_count.saturating_add(1);
        }
        latest = Some(latest.map_or(alert.emitted_at, |current: OffsetDateTime| {
            current.max(alert.emitted_at)
        }));
    }
    if latest.is_some_and(|last| {
        convergence.evaluated_at - last < Duration::seconds(policy.cooldown_seconds)
    }) {
        return AlertDecision::Cooldown;
    }
    if daily_count >= policy.maximum_per_day {
        return AlertDecision::DailyBudgetExhausted;
    }

    AlertDecision::Emit {
        dedup_key: format!(
            "{}:{}:{}",
            policy.id,
            convergence.subject_key,
            convergence.evaluated_at.unix_timestamp()
        ),
        severity: policy.severity,
        evidence: convergence.evidence.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal_convergence::Direction;
    use prismatik_domain::ProviderId;
    use prismatik_market_data::BlindSpot;

    fn convergence(now: OffsetDateTime) -> ConvergenceResult {
        ConvergenceResult {
            subject_key: "BTC-USD".into(),
            score_ppm: 700_000,
            direction: Direction::Bullish,
            confidence_ppm: 800_000,
            accepted_signals: 2,
            evidence: vec![EvidenceRef {
                id: "observation:1".into(),
                provider: ProviderId::COINGECKO,
                retrieved_at: now,
            }],
            blind_spots: Vec::<BlindSpot>::new(),
            evaluated_at: now,
        }
    }

    fn policy() -> AlertPolicy {
        AlertPolicy {
            id: "large-convergence".into(),
            minimum_score_ppm: 600_000,
            minimum_confidence_ppm: 700_000,
            cooldown_seconds: 3_600,
            maximum_per_day: 2,
            severity: AlertSeverity::Warning,
            enabled: true,
        }
    }

    #[test]
    fn emits_with_evidence_then_respects_cooldown() {
        let now = OffsetDateTime::UNIX_EPOCH + Duration::hours(4);
        assert!(matches!(
            evaluate_alert(&policy(), &convergence(now), &[]),
            AlertDecision::Emit { .. }
        ));
        let history = [PriorAlert {
            policy_id: policy().id,
            subject_key: "BTC-USD".into(),
            emitted_at: now - Duration::minutes(5),
        }];
        assert_eq!(
            evaluate_alert(&policy(), &convergence(now), &history),
            AlertDecision::Cooldown
        );
    }

    #[test]
    fn enforces_rolling_daily_budget() {
        let now = OffsetDateTime::UNIX_EPOCH + Duration::hours(30);
        let history = [
            PriorAlert {
                policy_id: policy().id.clone(),
                subject_key: "BTC-USD".into(),
                emitted_at: now - Duration::hours(20),
            },
            PriorAlert {
                policy_id: policy().id.clone(),
                subject_key: "BTC-USD".into(),
                emitted_at: now - Duration::hours(2),
            },
        ];
        assert_eq!(
            evaluate_alert(&policy(), &convergence(now), &history),
            AlertDecision::DailyBudgetExhausted
        );
    }
}
