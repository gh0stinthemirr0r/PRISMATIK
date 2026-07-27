//! Hand-labelled options flow corpus floor (Wave 2 author-ops residual → Wave 4).
//!
//! Typed labels for classifier goldens. The turbo gate waives collecting 20
//! hand-labelled prints on author hardware; this module stores the schema, an
//! expanded fixture corpus with paired [`OptionsFlowPrint`]s, and a summary
//! surface so CI / desktop can score against committed labels when present.

use crate::{FlowClassification, FlowSide, OptionContract, OptionRight, OptionsFlowPrint};
use prismatik_identity::AssetId;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::macros::{date, datetime};
use time::OffsetDateTime;

/// Author (or fixture) label for a single flow print.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandLabel {
    /// Stable print id (opaque key into a cassette / store).
    pub print_id: String,
    /// Author-assigned classification.
    pub expected: FlowClassification,
    /// Optional free-text rationale.
    #[serde(default)]
    pub notes: String,
    /// Labeler identity (opaque).
    pub labeled_by: String,
}

/// Corpus of hand labels for regression scoring.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandLabelCorpus {
    /// Corpus version / schema id.
    pub version: String,
    /// Labels.
    pub labels: Vec<HandLabel>,
}

/// Per-class label counts in a corpus.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandLabelClassCounts {
    /// Directional labels.
    pub directional: usize,
    /// Hedging labels.
    pub hedging: usize,
    /// Closing labels.
    pub closing: usize,
    /// Spread-leg labels.
    pub spread_leg: usize,
    /// Unclassified labels.
    pub unclassified: usize,
}

/// Compact corpus + score summary for gates and desktop chips.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandLabelCorpusSummary {
    /// Corpus version / schema id.
    pub version: String,
    /// Number of labels present.
    pub label_count: usize,
    /// Author review target (20).
    pub author_target: usize,
    /// Whether `label_count >= author_target`.
    pub meets_author_target: bool,
    /// Classifier matches against paired fixture prints (when scored).
    pub matched: usize,
    /// Labels compared against paired fixture prints.
    pub compared: usize,
    /// Label histogram by expected class.
    pub class_counts: HandLabelClassCounts,
}

/// Errors loading or scoring a corpus.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum HandLabelError {
    /// JSON parse failure.
    #[error("hand-label corpus parse: {0}")]
    Parse(String),
    /// Corpus empty when a non-empty set was required.
    #[error("hand-label corpus empty")]
    Empty,
}

impl HandLabelCorpus {
    /// Parse corpus JSON.
    pub fn from_json(json: &str) -> Result<Self, HandLabelError> {
        serde_json::from_str(json).map_err(|e| HandLabelError::Parse(e.to_string()))
    }

    /// Minimum turbo-fixture size (author target is 20).
    pub const AUTHOR_TARGET: usize = 20;

    /// Whether the corpus meets the author target count.
    #[must_use]
    pub fn meets_author_target(&self) -> bool {
        self.labels.len() >= Self::AUTHOR_TARGET
    }

    /// Histogram of expected classifications.
    #[must_use]
    pub fn class_counts(&self) -> HandLabelClassCounts {
        let mut counts = HandLabelClassCounts::default();
        for label in &self.labels {
            match label.expected {
                FlowClassification::Directional => counts.directional += 1,
                FlowClassification::Hedging => counts.hedging += 1,
                FlowClassification::Closing => counts.closing += 1,
                FlowClassification::SpreadLeg => counts.spread_leg += 1,
                FlowClassification::Unclassified => counts.unclassified += 1,
            }
        }
        counts
    }

    /// Build a summary, optionally including a classifier score.
    #[must_use]
    pub fn summary(&self, score: Option<(usize, usize)>) -> HandLabelCorpusSummary {
        let (matched, compared) = score.unwrap_or((0, 0));
        HandLabelCorpusSummary {
            version: self.version.clone(),
            label_count: self.labels.len(),
            author_target: Self::AUTHOR_TARGET,
            meets_author_target: self.meets_author_target(),
            matched,
            compared,
            class_counts: self.class_counts(),
        }
    }

    /// Score classifier output against labels for prints present in both.
    ///
    /// Returns `(matches, compared)`.
    pub fn score_against(&self, prints: &[(String, FlowClassification)]) -> (usize, usize) {
        let mut matches = 0usize;
        let mut compared = 0usize;
        for label in &self.labels {
            if let Some((_, got)) = prints.iter().find(|(id, _)| id == &label.print_id) {
                compared += 1;
                if *got == label.expected {
                    matches += 1;
                }
            }
        }
        (matches, compared)
    }
}

fn fixture_contract(root: &str, strike_millis: u64, right: OptionRight) -> OptionContract {
    OptionContract {
        underlying: AssetId::from_canonical_bytes(root.as_bytes()),
        root: root.into(),
        expiry: date!(2026 - 07 - 17),
        right,
        strike_millis,
    }
}

/// Build a labelled flow fixture. Ten fields map 1:1 to the print's columns;
/// bundling them into a struct would obscure the test data without behavioral
/// gain, so the argument count is allowed here deliberately.
#[allow(clippy::too_many_arguments)]
fn fixture_print(
    root: &str,
    strike_millis: u64,
    right: OptionRight,
    executed_at: OffsetDateTime,
    quantity: u64,
    premium_cents: u64,
    side: FlowSide,
    open_interest: Option<u64>,
    volume: Option<u64>,
    is_opening: Option<bool>,
) -> OptionsFlowPrint {
    OptionsFlowPrint {
        contract: fixture_contract(root, strike_millis, right),
        executed_at,
        quantity,
        premium_cents,
        side,
        open_interest,
        volume,
        is_opening,
    }
}

/// Built-in turbo fixture — expanded coverage, still below author target of 20.
#[must_use]
pub fn turbo_fixture_corpus() -> HandLabelCorpus {
    HandLabelCorpus {
        version: "flow-labels.v1".into(),
        labels: vec![
            HandLabel {
                print_id: "fix-open-ask-1".into(),
                expected: FlowClassification::Directional,
                notes: "opening ask sweep".into(),
                labeled_by: "turbo-fixture".into(),
            },
            HandLabel {
                print_id: "fix-open-ask-2".into(),
                expected: FlowClassification::Directional,
                notes: "opening ask put sweep".into(),
                labeled_by: "turbo-fixture".into(),
            },
            HandLabel {
                print_id: "fix-open-bid-aggressive-1".into(),
                expected: FlowClassification::Directional,
                notes: "opening bid qty > OI (aggressive)".into(),
                labeled_by: "turbo-fixture".into(),
            },
            HandLabel {
                print_id: "fix-close-1".into(),
                expected: FlowClassification::Closing,
                notes: "is_opening=false".into(),
                labeled_by: "turbo-fixture".into(),
            },
            HandLabel {
                print_id: "fix-close-2".into(),
                expected: FlowClassification::Closing,
                notes: "closing mid print".into(),
                labeled_by: "turbo-fixture".into(),
            },
            HandLabel {
                print_id: "fix-hedge-bid-1".into(),
                expected: FlowClassification::Hedging,
                notes: "opening bid with qty <= OI".into(),
                labeled_by: "turbo-fixture".into(),
            },
            HandLabel {
                print_id: "fix-hedge-bid-2".into(),
                expected: FlowClassification::Hedging,
                notes: "opening bid hedge equal OI".into(),
                labeled_by: "turbo-fixture".into(),
            },
            HandLabel {
                print_id: "fix-spread-mid-1".into(),
                expected: FlowClassification::SpreadLeg,
                notes: "opening mid".into(),
                labeled_by: "turbo-fixture".into(),
            },
            HandLabel {
                print_id: "fix-spread-mid-2".into(),
                expected: FlowClassification::SpreadLeg,
                notes: "opening mid put leg".into(),
                labeled_by: "turbo-fixture".into(),
            },
            HandLabel {
                print_id: "fix-unknown-1".into(),
                expected: FlowClassification::Unclassified,
                notes: "unknown side, missing opening flag".into(),
                labeled_by: "turbo-fixture".into(),
            },
        ],
    }
}

/// Paired [`OptionsFlowPrint`]s for [`turbo_fixture_corpus`] (same `print_id`s).
#[must_use]
pub fn turbo_fixture_prints() -> Vec<(String, OptionsFlowPrint)> {
    vec![
        (
            "fix-open-ask-1".into(),
            fixture_print(
                "AAPL",
                200_000,
                OptionRight::Call,
                datetime!(2026-07-15 14:30:00 UTC),
                120,
                480_000,
                FlowSide::Ask,
                Some(80),
                Some(400),
                Some(true),
            ),
        ),
        (
            "fix-open-ask-2".into(),
            fixture_print(
                "MSFT",
                450_000,
                OptionRight::Put,
                datetime!(2026-07-15 14:31:00 UTC),
                90,
                270_000,
                FlowSide::Ask,
                Some(200),
                Some(300),
                Some(true),
            ),
        ),
        (
            "fix-open-bid-aggressive-1".into(),
            fixture_print(
                "NVDA",
                120_000,
                OptionRight::Call,
                datetime!(2026-07-15 14:32:00 UTC),
                500,
                1_250_000,
                FlowSide::Bid,
                Some(100),
                Some(800),
                Some(true),
            ),
        ),
        (
            "fix-close-1".into(),
            fixture_print(
                "AAPL",
                195_000,
                OptionRight::Call,
                datetime!(2026-07-15 14:33:00 UTC),
                40,
                60_000,
                FlowSide::Bid,
                Some(1_000),
                Some(200),
                Some(false),
            ),
        ),
        (
            "fix-close-2".into(),
            fixture_print(
                "SPY",
                550_000,
                OptionRight::Put,
                datetime!(2026-07-15 14:34:00 UTC),
                25,
                40_000,
                FlowSide::Mid,
                Some(5_000),
                Some(150),
                Some(false),
            ),
        ),
        (
            "fix-hedge-bid-1".into(),
            fixture_print(
                "TSLA",
                250_000,
                OptionRight::Put,
                datetime!(2026-07-15 14:35:00 UTC),
                50,
                125_000,
                FlowSide::Bid,
                Some(200),
                Some(180),
                Some(true),
            ),
        ),
        (
            "fix-hedge-bid-2".into(),
            fixture_print(
                "AMZN",
                180_000,
                OptionRight::Call,
                datetime!(2026-07-15 14:36:00 UTC),
                75,
                90_000,
                FlowSide::Bid,
                Some(75),
                Some(90),
                Some(true),
            ),
        ),
        (
            "fix-spread-mid-1".into(),
            fixture_print(
                "QQQ",
                480_000,
                OptionRight::Call,
                datetime!(2026-07-15 14:37:00 UTC),
                60,
                72_000,
                FlowSide::Mid,
                Some(400),
                Some(120),
                Some(true),
            ),
        ),
        (
            "fix-spread-mid-2".into(),
            fixture_print(
                "IWM",
                210_000,
                OptionRight::Put,
                datetime!(2026-07-15 14:38:00 UTC),
                30,
                36_000,
                FlowSide::Mid,
                Some(150),
                Some(70),
                Some(true),
            ),
        ),
        (
            "fix-unknown-1".into(),
            fixture_print(
                "META",
                520_000,
                OptionRight::Call,
                datetime!(2026-07-15 14:39:00 UTC),
                15,
                45_000,
                FlowSide::Unknown,
                Some(90),
                Some(40),
                None,
            ),
        ),
    ]
}

/// Score the turbo fixture prints against the turbo corpus via [`classify_flow`].
#[must_use]
pub fn turbo_fixture_score() -> (usize, usize) {
    score_classifier(&turbo_fixture_corpus(), &turbo_fixture_prints())
}

/// Summary of the turbo fixture corpus including live classifier score.
#[must_use]
pub fn turbo_fixture_summary() -> HandLabelCorpusSummary {
    let corpus = turbo_fixture_corpus();
    let score = score_classifier(&corpus, &turbo_fixture_prints());
    corpus.summary(Some(score))
}

/// Classify prints and score against a corpus by synthetic ids.
pub fn score_classifier(
    corpus: &HandLabelCorpus,
    prints: &[(String, OptionsFlowPrint)],
) -> (usize, usize) {
    let classified: Vec<(String, FlowClassification)> = prints
        .iter()
        .map(|(id, p)| (id.clone(), crate::classify_flow(p).classification))
        .collect();
    corpus.score_against(&classified)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turbo_fixture_below_author_target() {
        let c = turbo_fixture_corpus();
        assert!(!c.meets_author_target());
        assert_eq!(c.labels.len(), 10);
        assert_eq!(HandLabelCorpus::AUTHOR_TARGET, 20);
    }

    #[test]
    fn turbo_fixture_covers_all_classes() {
        let counts = turbo_fixture_corpus().class_counts();
        assert_eq!(counts.directional, 3);
        assert_eq!(counts.hedging, 2);
        assert_eq!(counts.closing, 2);
        assert_eq!(counts.spread_leg, 2);
        assert_eq!(counts.unclassified, 1);
    }

    #[test]
    fn score_perfect_match_on_labels_alone() {
        let c = turbo_fixture_corpus();
        let prints: Vec<(String, FlowClassification)> = c
            .labels
            .iter()
            .map(|l| (l.print_id.clone(), l.expected))
            .collect();
        let (m, n) = c.score_against(&prints);
        assert_eq!((m, n), (10, 10));
    }

    #[test]
    fn score_classifier_perfect_on_paired_prints() {
        let (m, n) = turbo_fixture_score();
        assert_eq!(n, 10, "all fixture labels must have paired prints");
        assert_eq!(m, 10, "classifier must match every turbo fixture label");
    }

    #[test]
    fn turbo_fixture_print_ids_align() {
        let corpus = turbo_fixture_corpus();
        let prints = turbo_fixture_prints();
        assert_eq!(corpus.labels.len(), prints.len());
        for (label, (id, _)) in corpus.labels.iter().zip(prints.iter()) {
            assert_eq!(&label.print_id, id);
        }
    }

    #[test]
    fn summary_reports_score_and_gap_to_author_target() {
        let summary = turbo_fixture_summary();
        assert_eq!(summary.version, "flow-labels.v1");
        assert_eq!(summary.label_count, 10);
        assert_eq!(summary.author_target, 20);
        assert!(!summary.meets_author_target);
        assert_eq!((summary.matched, summary.compared), (10, 10));
        assert_eq!(summary.class_counts.directional, 3);
    }

    #[test]
    fn score_partial_when_print_missing() {
        let c = turbo_fixture_corpus();
        let prints = vec![
            ("fix-open-ask-1".into(), FlowClassification::Directional),
            ("fix-missing".into(), FlowClassification::Closing),
        ];
        let (m, n) = c.score_against(&prints);
        assert_eq!((m, n), (1, 1));
    }

    #[test]
    fn score_mismatch_counts_compared() {
        let c = turbo_fixture_corpus();
        let prints = vec![("fix-open-ask-1".into(), FlowClassification::Hedging)];
        let (m, n) = c.score_against(&prints);
        assert_eq!((m, n), (0, 1));
    }

    #[test]
    fn json_round_trip() {
        let c = turbo_fixture_corpus();
        let json = serde_json::to_string(&c).unwrap();
        let back = HandLabelCorpus::from_json(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn summary_json_round_trip() {
        let summary = turbo_fixture_summary();
        let json = serde_json::to_string(&summary).unwrap();
        let back: HandLabelCorpusSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, summary);
    }
}
