//! Evidence-bound research packet construction.
//!
//! This module accepts structured analyst claims but never invents evidence.
//! Every included claim must cite a registered immutable observation, and
//! point-in-time checks reject evidence retrieved after packet generation.

use prismatik_market_data::{BlindSpot, EvidenceRef};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use time::OffsetDateTime;

/// Role responsible for a research claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchRole {
    /// Price, liquidity, and technical structure.
    MarketStructure,
    /// Financial statements, filings, and valuation.
    Fundamental,
    /// Macro releases, policy, and cross-asset context.
    Macro,
    /// News attention, diffusion, and sentiment.
    Attention,
    /// Prediction-market probabilities and resolution rules.
    PredictionMarket,
    /// Explicitly searches for disconfirming evidence.
    Skeptic,
}

/// Direction a claim takes on its normalized proposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStance {
    /// Supports the proposition.
    Supports,
    /// Opposes the proposition.
    Opposes,
    /// Adds context without taking a direction.
    Context,
}

/// Registered evidence metadata available to a packet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceDocument {
    /// Immutable observation reference.
    pub reference: EvidenceRef,
    /// Canonical source URI retained for user inspection.
    pub source_uri: String,
    /// Publisher or dataset title when retention policy permits it.
    pub title: Option<String>,
}

/// One analyst claim with mandatory citations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitedClaim {
    /// Stable claim identifier.
    pub id: String,
    /// Normalized proposition key used to reveal disagreement.
    pub proposition_key: String,
    /// Human-readable claim.
    pub text: String,
    /// Analyst role that produced the claim.
    pub role: ResearchRole,
    /// Direction on the proposition.
    pub stance: ClaimStance,
    /// Calibrated confidence in parts per million.
    pub confidence_ppm: u32,
    /// Evidence ids cited by this claim.
    pub citation_ids: Vec<String>,
}

/// Explicit disagreement retained in the packet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchContradiction {
    /// Shared proposition on which claims disagree.
    pub proposition_key: String,
    /// Supporting claim ids.
    pub supporting_claim_ids: Vec<String>,
    /// Opposing claim ids.
    pub opposing_claim_ids: Vec<String>,
}

/// Point-in-time-safe research packet suitable for UI rendering or export.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchPacket {
    /// Subject under investigation.
    pub subject_key: String,
    /// Caller-supplied packet generation time.
    pub generated_at: OffsetDateTime,
    /// Validated cited claims in stable id order.
    pub claims: Vec<CitedClaim>,
    /// Evidence actually cited by included claims.
    pub evidence: Vec<EvidenceDocument>,
    /// Automatically exposed proposition conflicts.
    pub contradictions: Vec<ResearchContradiction>,
    /// Declared coverage gaps and caveats.
    pub blind_spots: Vec<BlindSpot>,
}

/// Research packet validation failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ResearchError {
    /// Packet subject was empty.
    #[error("research packet subject must not be empty")]
    EmptySubject,
    /// Claim or proposition identity was empty.
    #[error("claim has an empty required field: {0}")]
    EmptyClaimField(String),
    /// Claim confidence exceeded one million.
    #[error("claim confidence exceeds one million: {0}")]
    InvalidConfidence(String),
    /// Claim supplied no citations.
    #[error("claim is not evidence-bound: {0}")]
    OrphanClaim(String),
    /// Claim cited an unregistered evidence id.
    #[error("claim {claim_id} cites unknown evidence {evidence_id}")]
    UnknownEvidence {
        /// Claim containing the citation.
        claim_id: String,
        /// Missing evidence identifier.
        evidence_id: String,
    },
    /// Evidence was retrieved after packet generation.
    #[error("evidence {0} was retrieved after packet generation")]
    FutureEvidence(String),
    /// Duplicate claim or evidence id was supplied.
    #[error("duplicate research identifier: {0}")]
    DuplicateIdentifier(String),
}

/// Validate evidence and construct a deterministic multi-role research packet.
pub fn build_research_packet(
    subject_key: impl Into<String>,
    generated_at: OffsetDateTime,
    evidence: Vec<EvidenceDocument>,
    claims: Vec<CitedClaim>,
    blind_spots: Vec<BlindSpot>,
) -> Result<ResearchPacket, ResearchError> {
    let subject_key = subject_key.into();
    if subject_key.trim().is_empty() {
        return Err(ResearchError::EmptySubject);
    }
    let mut evidence_by_id = BTreeMap::new();
    for document in evidence {
        if document.reference.id.trim().is_empty() || document.source_uri.trim().is_empty() {
            return Err(ResearchError::DuplicateIdentifier(document.reference.id));
        }
        if document.reference.retrieved_at > generated_at {
            return Err(ResearchError::FutureEvidence(document.reference.id));
        }
        let evidence_id = document.reference.id.clone();
        if evidence_by_id
            .insert(evidence_id.clone(), document)
            .is_some()
        {
            return Err(ResearchError::DuplicateIdentifier(evidence_id));
        }
    }

    let mut claim_ids = BTreeSet::new();
    let mut validated_claims = Vec::with_capacity(claims.len());
    let mut cited_ids = BTreeSet::new();
    for mut claim in claims {
        if claim.id.trim().is_empty()
            || claim.proposition_key.trim().is_empty()
            || claim.text.trim().is_empty()
        {
            return Err(ResearchError::EmptyClaimField(claim.id));
        }
        if !claim_ids.insert(claim.id.clone()) {
            return Err(ResearchError::DuplicateIdentifier(claim.id));
        }
        if claim.confidence_ppm > 1_000_000 {
            return Err(ResearchError::InvalidConfidence(claim.id));
        }
        claim.citation_ids.sort();
        claim.citation_ids.dedup();
        if claim.citation_ids.is_empty() {
            return Err(ResearchError::OrphanClaim(claim.id));
        }
        for evidence_id in &claim.citation_ids {
            if !evidence_by_id.contains_key(evidence_id) {
                return Err(ResearchError::UnknownEvidence {
                    claim_id: claim.id.clone(),
                    evidence_id: evidence_id.clone(),
                });
            }
            cited_ids.insert(evidence_id.clone());
        }
        validated_claims.push(claim);
    }
    validated_claims.sort_by(|left, right| left.id.cmp(&right.id));

    let mut propositions = BTreeMap::<String, (Vec<String>, Vec<String>)>::new();
    for claim in &validated_claims {
        let sides = propositions
            .entry(claim.proposition_key.clone())
            .or_default();
        match claim.stance {
            ClaimStance::Supports => sides.0.push(claim.id.clone()),
            ClaimStance::Opposes => sides.1.push(claim.id.clone()),
            ClaimStance::Context => {},
        }
    }
    let contradictions = propositions
        .into_iter()
        .filter_map(
            |(proposition_key, (supporting_claim_ids, opposing_claim_ids))| {
                (!supporting_claim_ids.is_empty() && !opposing_claim_ids.is_empty()).then_some(
                    ResearchContradiction {
                        proposition_key,
                        supporting_claim_ids,
                        opposing_claim_ids,
                    },
                )
            },
        )
        .collect();
    let evidence = cited_ids
        .into_iter()
        .filter_map(|id| evidence_by_id.remove(&id))
        .collect();
    Ok(ResearchPacket {
        subject_key,
        generated_at,
        claims: validated_claims,
        evidence,
        contradictions,
        blind_spots,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_domain::ProviderId;

    fn evidence(id: &str) -> EvidenceDocument {
        EvidenceDocument {
            reference: EvidenceRef {
                id: id.into(),
                provider: ProviderId::SEC_EDGAR,
                retrieved_at: OffsetDateTime::UNIX_EPOCH,
            },
            source_uri: format!("https://example.test/{id}"),
            title: None,
        }
    }

    fn claim(id: &str, stance: ClaimStance) -> CitedClaim {
        CitedClaim {
            id: id.into(),
            proposition_key: "earnings_accelerating".into(),
            text: format!("claim {id}"),
            role: ResearchRole::Fundamental,
            stance,
            confidence_ppm: 700_000,
            citation_ids: vec!["filing:1".into()],
        }
    }

    #[test]
    fn builds_cited_packet_and_exposes_contradiction() {
        let packet = build_research_packet(
            "AAPL",
            OffsetDateTime::UNIX_EPOCH,
            vec![evidence("filing:1")],
            vec![
                claim("bull", ClaimStance::Supports),
                claim("bear", ClaimStance::Opposes),
            ],
            vec![],
        )
        .unwrap();
        assert_eq!(packet.evidence.len(), 1);
        assert_eq!(packet.contradictions.len(), 1);
    }

    #[test]
    fn rejects_orphan_claims_and_future_evidence() {
        let mut orphan = claim("orphan", ClaimStance::Context);
        orphan.citation_ids.clear();
        assert_eq!(
            build_research_packet(
                "AAPL",
                OffsetDateTime::UNIX_EPOCH,
                vec![],
                vec![orphan],
                vec![]
            ),
            Err(ResearchError::OrphanClaim("orphan".into()))
        );
        let mut future = evidence("future");
        future.reference.retrieved_at = OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1);
        assert_eq!(
            build_research_packet(
                "AAPL",
                OffsetDateTime::UNIX_EPOCH,
                vec![future],
                vec![],
                vec![]
            ),
            Err(ResearchError::FutureEvidence("future".into()))
        );
    }
}
