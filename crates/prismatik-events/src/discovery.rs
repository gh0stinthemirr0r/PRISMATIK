//! Deterministic event lifecycle and discovery signals.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// One observed event state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventLifecycleObservation {
    /// Canonical event identifier.
    pub event_id: String,
    /// Provider venue.
    pub venue: String,
    /// Normalized category.
    pub category: String,
    /// Probability ppm when available.
    pub probability_ppm: Option<u32>,
    /// Liquidity micros when available.
    pub liquidity_micros: Option<u64>,
    /// Immutable rules digest.
    pub rules_hash: String,
    /// Venue lifecycle label.
    pub status: String,
    /// Observation timestamp.
    pub observed_at: OffsetDateTime,
    /// Evidence record.
    pub evidence_id: String,
}

/// Explainable lifecycle signal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSignal {
    /// First observation of an event.
    NewlyListed {
        /// Event identifier.
        event_id: String,
    },
    /// Venue rule text changed.
    RulesChanged {
        /// Event identifier.
        event_id: String,
        /// Prior digest.
        prior_hash: String,
        /// Current digest.
        current_hash: String,
    },
    /// Probability moved by at least threshold.
    ProbabilityMoved {
        /// Event identifier.
        event_id: String,
        /// Signed change ppm.
        delta_ppm: i64,
    },
    /// Liquidity grew by at least threshold.
    LiquidityAccelerated {
        /// Event identifier.
        event_id: String,
        /// Growth ppm.
        growth_ppm: u32,
    },
    /// Lifecycle status changed.
    StatusChanged {
        /// Event identifier.
        event_id: String,
        /// Prior state.
        prior: String,
        /// Current state.
        current: String,
    },
}

/// Compare consecutive observations without manufacturing missing fields.
pub fn discover_event_changes(
    prior: Option<&EventLifecycleObservation>,
    current: &EventLifecycleObservation,
    probability_threshold_ppm: u32,
    liquidity_growth_threshold_ppm: u32,
) -> Vec<EventSignal> {
    let Some(prior) = prior else {
        return vec![EventSignal::NewlyListed {
            event_id: current.event_id.clone(),
        }];
    };
    if prior.event_id != current.event_id || current.observed_at < prior.observed_at {
        return Vec::new();
    }
    let mut signals = Vec::new();
    if prior.rules_hash != current.rules_hash {
        signals.push(EventSignal::RulesChanged {
            event_id: current.event_id.clone(),
            prior_hash: prior.rules_hash.clone(),
            current_hash: current.rules_hash.clone(),
        });
    }
    if prior.status != current.status {
        signals.push(EventSignal::StatusChanged {
            event_id: current.event_id.clone(),
            prior: prior.status.clone(),
            current: current.status.clone(),
        });
    }
    if let (Some(before), Some(after)) = (prior.probability_ppm, current.probability_ppm) {
        let delta = i64::from(after) - i64::from(before);
        if delta.unsigned_abs() >= u64::from(probability_threshold_ppm) {
            signals.push(EventSignal::ProbabilityMoved {
                event_id: current.event_id.clone(),
                delta_ppm: delta,
            });
        }
    }
    if let (Some(before), Some(after)) = (prior.liquidity_micros, current.liquidity_micros) {
        if before > 0 && after > before {
            let growth = u32::try_from(u128::from(after - before) * 1_000_000 / u128::from(before))
                .unwrap_or(u32::MAX);
            if growth >= liquidity_growth_threshold_ppm {
                signals.push(EventSignal::LiquidityAccelerated {
                    event_id: current.event_id.clone(),
                    growth_ppm: growth,
                });
            }
        }
    }
    signals
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_material_changes() {
        let base = EventLifecycleObservation {
            event_id: "e".into(),
            venue: "v".into(),
            category: "macro".into(),
            probability_ppm: Some(500_000),
            liquidity_micros: Some(100),
            rules_hash: "a".into(),
            status: "open".into(),
            observed_at: OffsetDateTime::UNIX_EPOCH,
            evidence_id: "1".into(),
        };
        let mut next = base.clone();
        next.probability_ppm = Some(600_000);
        next.liquidity_micros = Some(200);
        next.rules_hash = "b".into();
        next.observed_at += time::Duration::seconds(1);
        assert_eq!(
            discover_event_changes(Some(&base), &next, 50_000, 500_000).len(),
            3
        );
    }
}
