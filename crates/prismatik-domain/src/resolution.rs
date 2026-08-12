//! Prediction-contract resolution semantics and deterministic risk assessment.

use crate::prediction::{ContractId, PredictionContract};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use time::OffsetDateTime;

/// Immutable observation of venue resolution language.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionRuleSnapshot {
    /// Contract whose rules were observed.
    pub contract_id: ContractId,
    /// Monotonic venue revision identifier when supplied.
    pub revision: Option<String>,
    /// Exact normalized rule text.
    pub rules: String,
    /// Named authoritative source.
    pub resolution_source: String,
    /// Observation time at the provider boundary.
    pub observed_at: OffsetDateTime,
    /// Content digest supplied by the ingestion layer.
    pub content_hash: String,
}

/// Historical resolution precedent attached as evidence, never as an automatic legal interpretation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionPrecedent {
    /// Stable precedent identifier.
    pub id: String,
    /// Venue where the precedent occurred.
    pub venue: String,
    /// Short normalized description.
    pub summary: String,
    /// Similarity in parts per million, computed upstream.
    pub similarity_ppm: u32,
    /// Evidence record supporting the precedent.
    pub evidence_id: String,
}

/// Explainable resolution-risk reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionRiskReason {
    /// No authoritative source is named.
    MissingSource,
    /// No deadline is expressed in supplied context.
    MissingDeadline,
    /// Rules contain discretion-bearing language.
    SubjectiveLanguage(String),
    /// Rule text changed after first observation.
    RuleChanged,
    /// A dispute/challenge mechanism is mentioned.
    DisputeWindow,
    /// Similar-looking contracts have materially different semantics.
    SemanticMismatch,
}

/// Explainable fixed-point resolution risk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionRiskAssessment {
    /// Risk score from zero through one million.
    pub score_ppm: u32,
    /// Deterministically ordered contributing reasons.
    pub reasons: Vec<ResolutionRiskReason>,
    /// Evidence precedents supplied to the assessment.
    pub precedents: Vec<ResolutionPrecedent>,
}

/// Deterministic semantic comparison between two venue contracts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractSemanticComparison {
    /// Token-set similarity in parts per million.
    pub rules_similarity_ppm: u32,
    /// Whether normalized resolution sources match.
    pub same_resolution_source: bool,
    /// Whether conservative equivalence checks pass.
    pub equivalent: bool,
    /// Deterministic mismatch explanations.
    pub differences: Vec<String>,
}

/// Compare exact contract semantics using a conservative token-set measure.
pub fn compare_contract_semantics(
    left: &PredictionContract,
    right: &PredictionContract,
) -> ContractSemanticComparison {
    let left_tokens = tokens(&left.resolution_rules);
    let right_tokens = tokens(&right.resolution_rules);
    let union = left_tokens.union(&right_tokens).count();
    let intersection = left_tokens.intersection(&right_tokens).count();
    let similarity = if union == 0 {
        0
    } else {
        u32::try_from(intersection.saturating_mul(1_000_000) / union).unwrap_or(1_000_000)
    };
    let same_source = normalize(&left.resolution_source) == normalize(&right.resolution_source);
    let mut differences = Vec::new();
    if !same_source {
        differences.push("resolution sources differ".into());
    }
    if similarity < 900_000 {
        differences.push("resolution language differs materially".into());
    }
    if left.outcome != right.outcome {
        differences.push("contract outcomes differ".into());
    }
    ContractSemanticComparison {
        rules_similarity_ppm: similarity,
        same_resolution_source: same_source,
        equivalent: differences.is_empty(),
        differences,
    }
}

/// Assess resolution risk from immutable rule snapshots and optional context.
pub fn assess_resolution_risk(
    contract: &PredictionContract,
    snapshots: &[ResolutionRuleSnapshot],
    has_deadline: bool,
    semantic_mismatch: bool,
    precedents: Vec<ResolutionPrecedent>,
) -> ResolutionRiskAssessment {
    let mut reasons = Vec::new();
    if contract.resolution_source.trim().is_empty() {
        reasons.push(ResolutionRiskReason::MissingSource);
    }
    if !has_deadline {
        reasons.push(ResolutionRiskReason::MissingDeadline);
    }
    let normalized = normalize(&contract.resolution_rules);
    for marker in [
        "sole discretion",
        "may determine",
        "subjective",
        "reasonable judgment",
    ] {
        if normalized.contains(marker) {
            reasons.push(ResolutionRiskReason::SubjectiveLanguage(marker.into()));
        }
    }
    if snapshots
        .iter()
        .map(|snapshot| snapshot.content_hash.as_str())
        .collect::<BTreeSet<_>>()
        .len()
        > 1
    {
        reasons.push(ResolutionRiskReason::RuleChanged);
    }
    if normalized.contains("dispute") || normalized.contains("challenge period") {
        reasons.push(ResolutionRiskReason::DisputeWindow);
    }
    if semantic_mismatch {
        reasons.push(ResolutionRiskReason::SemanticMismatch);
    }
    let score_ppm = reasons
        .iter()
        .map(|reason| match reason {
            ResolutionRiskReason::MissingSource => 300_000_u32,
            ResolutionRiskReason::MissingDeadline => 100_000,
            ResolutionRiskReason::SubjectiveLanguage(_) => 250_000,
            ResolutionRiskReason::RuleChanged => 250_000,
            ResolutionRiskReason::DisputeWindow => 100_000,
            ResolutionRiskReason::SemanticMismatch => 400_000,
        })
        .fold(0_u32, u32::saturating_add)
        .min(1_000_000);
    ResolutionRiskAssessment {
        score_ppm,
        reasons,
        precedents,
    }
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn tokens(value: &str) -> BTreeSet<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| token.len() > 2)
        .map(str::to_ascii_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prediction::{EventId, Outcome};

    fn contract(id: &str, source: &str, rules: &str) -> PredictionContract {
        PredictionContract {
            id: ContractId::new(id).unwrap(),
            event_id: EventId::new("event").unwrap(),
            outcome: Outcome::Yes,
            resolution_source: source.into(),
            resolution_rules: rules.into(),
        }
    }

    #[test]
    fn semantic_comparison_rejects_different_sources() {
        let left = contract(
            "a",
            "Official count",
            "Resolves yes when official count exceeds five",
        );
        let right = contract(
            "b",
            "Press reports",
            "Resolves yes when official count exceeds five",
        );
        let comparison = compare_contract_semantics(&left, &right);
        assert!(!comparison.equivalent);
        assert_eq!(comparison.rules_similarity_ppm, 1_000_000);
    }

    #[test]
    fn risk_is_explainable_and_clamped() {
        let candidate = contract(
            "a",
            "",
            "Venue may determine in its sole discretion after dispute",
        );
        let assessment = assess_resolution_risk(&candidate, &[], false, true, vec![]);
        assert_eq!(assessment.score_ppm, 1_000_000);
        assert!(assessment.reasons.len() >= 4);
    }
}
