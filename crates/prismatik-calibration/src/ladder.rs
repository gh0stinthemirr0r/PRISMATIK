//! Baseline ladder promotion gate (`P5-QM-13` floor).
//!
//! A candidate may promote only when it beats **every** lower rung on both
//! discrimination and calibration scores (higher is better; `f64` placeholders).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Ladder rung 0..=5 (v1.0 §17.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum LadderRung {
    /// Naive persistence / random walk / unconditional mean.
    R0 = 0,
    /// Classical statistical (ARIMA, GARCH, EWMA).
    R1 = 1,
    /// Regularized linear / gradient boosted trees.
    R2 = 2,
    /// Task-specific neural network.
    R3 = 3,
    /// TSFM (zero-shot or fine-tuned).
    R4 = 4,
    /// Ensemble.
    R5 = 5,
}

impl LadderRung {
    /// Numeric rung index 0..=5.
    pub fn index(self) -> u8 {
        self as u8
    }

    /// Parse a rung index; rejects values outside 0..=5.
    pub fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::R0),
            1 => Some(Self::R1),
            2 => Some(Self::R2),
            3 => Some(Self::R3),
            4 => Some(Self::R4),
            5 => Some(Self::R5),
            _ => None,
        }
    }

    /// All rungs in ascending order.
    pub fn all() -> [Self; 6] {
        [Self::R0, Self::R1, Self::R2, Self::R3, Self::R4, Self::R5]
    }
}

/// Out-of-sample comparison scores (placeholders until proper metrics land).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LadderScores {
    /// Discrimination score — higher is better (e.g. inverted CRPS / AUC stub).
    pub discrimination: f64,
    /// Calibration score — higher is better (e.g. coverage honesty stub).
    pub calibration: f64,
}

/// A model evaluated at a ladder rung.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LadderEntry {
    /// Rung occupied by this entry.
    pub rung: LadderRung,
    /// Opaque model id.
    pub model_id: String,
    /// Out-of-sample scores.
    pub scores: LadderScores,
}

/// Why promotion was denied.
#[derive(Clone, Debug, PartialEq, Error)]
pub enum PromotionDenied {
    /// Candidate failed to beat a lower rung on discrimination.
    #[error(
        "candidate rung {candidate:?} loses discrimination to rung {beaten_by:?} ({candidate_score} <= {baseline_score})"
    )]
    Discrimination {
        /// Candidate rung.
        candidate: LadderRung,
        /// Lower rung that was not beaten.
        beaten_by: LadderRung,
        /// Candidate discrimination.
        candidate_score: f64,
        /// Baseline discrimination.
        baseline_score: f64,
    },
    /// Candidate failed to beat a lower rung on calibration.
    #[error(
        "candidate rung {candidate:?} loses calibration to rung {beaten_by:?} ({candidate_score} <= {baseline_score})"
    )]
    Calibration {
        /// Candidate rung.
        candidate: LadderRung,
        /// Lower rung that was not beaten.
        beaten_by: LadderRung,
        /// Candidate calibration.
        candidate_score: f64,
        /// Baseline calibration.
        baseline_score: f64,
    },
    /// Missing a required lower-rung comparison.
    #[error("missing lower-rung comparison for rung {missing:?}")]
    MissingLowerRung {
        /// Expected lower rung not present in the comparison set.
        missing: LadderRung,
    },
}

/// Promotion gate: candidate must strictly beat every lower rung on both axes.
pub fn can_promote(
    candidate: &LadderEntry,
    lower_rungs: &[LadderEntry],
) -> Result<(), PromotionDenied> {
    let required: Vec<LadderRung> = LadderRung::all()
        .into_iter()
        .filter(|r| r.index() < candidate.rung.index())
        .collect();

    for need in &required {
        if !lower_rungs.iter().any(|e| e.rung == *need) {
            return Err(PromotionDenied::MissingLowerRung { missing: *need });
        }
    }

    for baseline in lower_rungs
        .iter()
        .filter(|e| e.rung.index() < candidate.rung.index())
    {
        if candidate.scores.discrimination <= baseline.scores.discrimination {
            return Err(PromotionDenied::Discrimination {
                candidate: candidate.rung,
                beaten_by: baseline.rung,
                candidate_score: candidate.scores.discrimination,
                baseline_score: baseline.scores.discrimination,
            });
        }
        if candidate.scores.calibration <= baseline.scores.calibration {
            return Err(PromotionDenied::Calibration {
                candidate: candidate.rung,
                beaten_by: baseline.rung,
                candidate_score: candidate.scores.calibration,
                baseline_score: baseline.scores.calibration,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(rung: LadderRung, disc: f64, cal: f64) -> LadderEntry {
        LadderEntry {
            rung,
            model_id: format!("rung-{}", rung.index()),
            scores: LadderScores {
                discrimination: disc,
                calibration: cal,
            },
        }
    }

    #[test]
    fn rungs_are_zero_through_five() {
        assert_eq!(LadderRung::all().map(|r| r.index()), [0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn promotion_requires_beating_discrimination_and_calibration() {
        let lowers = [
            entry(LadderRung::R0, 0.10, 0.10),
            entry(LadderRung::R1, 0.20, 0.20),
            entry(LadderRung::R2, 0.30, 0.30),
            entry(LadderRung::R3, 0.40, 0.40),
        ];
        let ok = entry(LadderRung::R4, 0.50, 0.50);
        assert!(can_promote(&ok, &lowers).is_ok());
    }

    /// Negative: rung-4 loses calibration to rung-1 → cannot promote.
    #[test]
    fn rung4_losing_calibration_to_rung1_cannot_promote() {
        let lowers = [
            entry(LadderRung::R0, 0.10, 0.10),
            entry(LadderRung::R1, 0.20, 0.90), // strong calibration
            entry(LadderRung::R2, 0.30, 0.30),
            entry(LadderRung::R3, 0.40, 0.40),
        ];
        // Beats everyone on discrimination, but loses calibration to R1.
        let candidate = entry(LadderRung::R4, 0.99, 0.50);
        let err = can_promote(&candidate, &lowers).unwrap_err();
        match err {
            PromotionDenied::Calibration { beaten_by, .. } => {
                assert_eq!(beaten_by, LadderRung::R1);
            },
            other => panic!("expected calibration denial, got {other:?}"),
        }
    }
}
