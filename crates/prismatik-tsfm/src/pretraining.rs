//! Pretraining hygiene and contamination gate (`P5-QM-06` floor).

use crate::registry::ModelId;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Typed pretraining disclosure for foundation models (v0.3 §6.1 / v1.0 §17.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PretrainingRecord {
    /// Human-readable corpus description.
    pub corpus_description: String,
    /// Last real-world observation date included in pretraining.
    ///
    /// Backtest runtime hard-denies any model whose cutoff is at or after the
    /// backtest window start.
    #[serde(with = "time::serde::rfc3339")]
    pub pretraining_cutoff: OffsetDateTime,
    /// Optional synthetic-data disclosure.
    #[serde(default)]
    pub synthetic_data_description: Option<String>,
    /// Channel / modality names included in the corpus.
    #[serde(default)]
    pub channels_included: Vec<String>,
}

impl PretrainingRecord {
    /// Convenience constructor.
    #[must_use]
    pub fn new(corpus_description: impl Into<String>, pretraining_cutoff: OffsetDateTime) -> Self {
        Self {
            corpus_description: corpus_description.into(),
            pretraining_cutoff,
            synthetic_data_description: None,
            channels_included: Vec::new(),
        }
    }
}

/// Result of the contamination gate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContaminationVerdict {
    /// Cutoff is strictly before the backtest window start.
    Clean {
        /// Model under evaluation.
        model_id: ModelId,
        /// Cutoff that passed.
        #[serde(with = "time::serde::rfc3339")]
        pretraining_cutoff: OffsetDateTime,
        /// Backtest window start compared against.
        #[serde(with = "time::serde::rfc3339")]
        backtest_window_start: OffsetDateTime,
    },
    /// Cutoff is at or after the backtest window start — hard deny.
    Contaminated {
        /// Model under evaluation.
        model_id: ModelId,
        /// Offending cutoff.
        #[serde(with = "time::serde::rfc3339")]
        pretraining_cutoff: OffsetDateTime,
        /// Backtest window start compared against.
        #[serde(with = "time::serde::rfc3339")]
        backtest_window_start: OffsetDateTime,
    },
}

impl ContaminationVerdict {
    /// Returns `true` when the gate denied the model.
    #[must_use]
    pub const fn is_denied(&self) -> bool {
        matches!(self, Self::Contaminated { .. })
    }
}

/// Typed security events emitted by the TSFM plane on hard denials.
///
/// Logged as security events (not warnings) per v1.0 §12.5.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TsfmSecurityEvent {
    /// Pretraining cutoff overlaps or follows the backtest start (lookahead).
    LookaheadDenied {
        /// Denied model.
        model_id: ModelId,
        /// Model pretraining cutoff.
        #[serde(with = "time::serde::rfc3339")]
        pretraining_cutoff: OffsetDateTime,
        /// Backtest window start that was violated.
        #[serde(with = "time::serde::rfc3339")]
        backtest_start: OffsetDateTime,
    },
}

/// Evaluate whether a pretraining cutoff contaminates a backtest window.
///
/// Deny when `pretraining_cutoff >= backtest_window_start`.
#[must_use]
pub fn evaluate_contamination(
    model_id: ModelId,
    pretraining_cutoff: OffsetDateTime,
    backtest_window_start: OffsetDateTime,
) -> ContaminationVerdict {
    if pretraining_cutoff >= backtest_window_start {
        ContaminationVerdict::Contaminated {
            model_id,
            pretraining_cutoff,
            backtest_window_start,
        }
    } else {
        ContaminationVerdict::Clean {
            model_id,
            pretraining_cutoff,
            backtest_window_start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn denies_when_cutoff_equals_window_start() {
        let start = datetime!(2024-01-01 00:00:00 UTC);
        let v = evaluate_contamination(ModelId::new("m"), start, start);
        assert!(v.is_denied());
    }

    #[test]
    fn denies_when_cutoff_after_window_start() {
        let start = datetime!(2024-01-01 00:00:00 UTC);
        let cutoff = datetime!(2024-06-01 00:00:00 UTC);
        let v = evaluate_contamination(ModelId::new("m"), cutoff, start);
        assert!(matches!(v, ContaminationVerdict::Contaminated { .. }));
    }

    #[test]
    fn allows_when_cutoff_before_window_start() {
        let start = datetime!(2024-01-01 00:00:00 UTC);
        let cutoff = datetime!(2023-12-31 23:59:59 UTC);
        let v = evaluate_contamination(ModelId::new("m"), cutoff, start);
        assert!(!v.is_denied());
    }
}
