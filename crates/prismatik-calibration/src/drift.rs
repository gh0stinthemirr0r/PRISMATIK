//! Drift detector suite and actions (`P5-QM-14` / `P5-QM-15` floor).
//!
//! Doctrine: out-of-domain conditions MUST widen uncertainty or suppress
//! output. Embedding drift above threshold maps to [`DriftAction::Widen`].

use serde::{Deserialize, Serialize};

/// Drift detector suite (v1.0 §17.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftDetector {
    /// Input distribution shift (PSI / KL).
    Feature,
    /// Realized coverage departing from nominal.
    Calibration,
    /// TSFM hidden-state / embedding distribution shift.
    Embedding,
    /// Token-usage distribution vs pretraining prior.
    TokenUsage,
    /// Realized error growth (confirmatory).
    Performance,
}

impl DriftDetector {
    /// All detectors in the suite.
    pub fn suite() -> [Self; 5] {
        [
            Self::Feature,
            Self::Calibration,
            Self::Embedding,
            Self::TokenUsage,
            Self::Performance,
        ]
    }
}

/// Configurable magnitude thresholds for drift actions.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriftThresholds {
    /// Above this: annotate (or widen for embedding).
    pub annotate: f64,
    /// Above this: widen intervals (embedding default response).
    pub widen: f64,
    /// Above this: suppress serving.
    pub suppress: f64,
    /// Above this: propose demotion.
    pub demote: f64,
}

impl Default for DriftThresholds {
    fn default() -> Self {
        Self {
            annotate: 0.10,
            widen: 0.25,
            suppress: 0.50,
            demote: 0.75,
        }
    }
}

/// Recommended response to a drift signal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftAction {
    /// Within tolerance.
    None,
    /// Annotate output with a drift notice.
    Annotate {
        /// Human-readable notice (floor string).
        notice: String,
    },
    /// Multiply interval width (default for embedding drift).
    Widen {
        /// Width multiplier (> 1.0).
        factor: f64,
    },
    /// Stop serving; fall back to another model id.
    Suppress {
        /// Fallback model id (opaque).
        fallback: String,
    },
    /// Propose demotion from the registry (human confirmation).
    ProposeDemotion {
        /// Evidence magnitude that triggered the action.
        magnitude: f64,
    },
}

/// Assessment produced by evaluating a detector against a magnitude.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriftAssessment {
    /// Which detector fired.
    pub detector: DriftDetector,
    /// Observed drift magnitude (placeholder metric).
    pub magnitude: f64,
    /// Recommended action.
    pub action: DriftAction,
}

/// Map detector + magnitude → action.
///
/// **Normative:** when [`DriftDetector::Embedding`] magnitude exceeds the widen
/// threshold, the action **must** be [`DriftAction::Widen`] (before suppress /
/// demote tiers apply at higher thresholds).
pub fn recommend_action(
    detector: DriftDetector,
    magnitude: f64,
    thresholds: &DriftThresholds,
) -> DriftAction {
    if magnitude <= thresholds.annotate {
        return DriftAction::None;
    }

    // Embedding drift: widen is mandatory once past the widen threshold,
    // until suppress/demote tiers take over.
    if detector == DriftDetector::Embedding {
        if magnitude > thresholds.demote {
            return DriftAction::ProposeDemotion { magnitude };
        }
        if magnitude > thresholds.suppress {
            return DriftAction::Suppress {
                fallback: "ladder-safe".into(),
            };
        }
        if magnitude > thresholds.widen {
            return DriftAction::Widen {
                factor: 1.0 + magnitude,
            };
        }
        return DriftAction::Annotate {
            notice: "embedding drift elevated".into(),
        };
    }

    if magnitude > thresholds.demote {
        return DriftAction::ProposeDemotion { magnitude };
    }
    if magnitude > thresholds.suppress {
        return DriftAction::Suppress {
            fallback: "ladder-safe".into(),
        };
    }
    if magnitude > thresholds.widen {
        // Non-embedding detectors may also widen, but embedding is the
        // normative must-widen case tested below.
        return DriftAction::Widen {
            factor: 1.0 + magnitude,
        };
    }
    DriftAction::Annotate {
        notice: format!("{detector:?} drift elevated"),
    }
}

/// Evaluate a detector against an observed magnitude.
pub fn evaluate(
    detector: DriftDetector,
    magnitude: f64,
    thresholds: &DriftThresholds,
) -> DriftAssessment {
    DriftAssessment {
        detector,
        magnitude,
        action: recommend_action(detector, magnitude, thresholds),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_lists_five_detectors() {
        assert_eq!(
            DriftDetector::suite(),
            [
                DriftDetector::Feature,
                DriftDetector::Calibration,
                DriftDetector::Embedding,
                DriftDetector::TokenUsage,
                DriftDetector::Performance,
            ]
        );
    }

    #[test]
    fn embedding_drift_above_threshold_must_widen() {
        let thresholds = DriftThresholds::default();
        // Strictly above widen, below suppress.
        let magnitude = thresholds.widen + 0.01;
        assert!(magnitude > thresholds.widen);
        assert!(magnitude <= thresholds.suppress);

        let action = recommend_action(DriftDetector::Embedding, magnitude, &thresholds);
        match action {
            DriftAction::Widen { factor } => {
                assert!(factor > 1.0);
            },
            other => panic!("embedding drift must Widen, got {other:?}"),
        }

        let assessment = evaluate(DriftDetector::Embedding, magnitude, &thresholds);
        assert!(matches!(assessment.action, DriftAction::Widen { .. }));
    }

    #[test]
    fn actions_cover_annotate_widen_suppress_demote() {
        let t = DriftThresholds::default();
        assert!(matches!(
            recommend_action(DriftDetector::Feature, t.annotate + 0.01, &t),
            DriftAction::Annotate { .. }
        ));
        assert!(matches!(
            recommend_action(DriftDetector::Feature, t.widen + 0.01, &t),
            DriftAction::Widen { .. }
        ));
        assert!(matches!(
            recommend_action(DriftDetector::Feature, t.suppress + 0.01, &t),
            DriftAction::Suppress { .. }
        ));
        assert!(matches!(
            recommend_action(DriftDetector::Feature, t.demote + 0.01, &t),
            DriftAction::ProposeDemotion { .. }
        ));
    }
}
