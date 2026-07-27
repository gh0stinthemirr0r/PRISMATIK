//! Journal entry capture, thesis linkage, outcome tags, review workflow (`P6-EX-02`).

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

/// Stable journal entry identifier (opaque string floor).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryId(pub String);

impl EntryId {
    /// Construct from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the inner id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Thesis text captured at entry time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thesis {
    /// Free-form thesis narrative.
    pub text: String,
}

impl Thesis {
    /// Construct a thesis.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

/// Outcome tagging for closed (or abandoned) journal entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeTag {
    /// Thesis validated; trade profitable vs expectation.
    Win,
    /// Thesis invalidated; trade unprofitable vs expectation.
    Loss,
    /// Near flat / no meaningful edge realized.
    Scratch,
    /// Entry closed without a graded outcome.
    Incomplete,
    /// Explicitly invalidated before full lifecycle.
    Invalidated,
}

/// Review workflow states for the journal entry lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewWorkflowState {
    /// Authoring in progress.
    Draft,
    /// Submitted for review.
    Submitted,
    /// Under active review.
    InReview,
    /// Review accepted; eligible for memory-loop promotion.
    Accepted,
    /// Review rejected; needs revision.
    Rejected,
}

/// Journal entry with thesis, linkage ids, optional outcome, and review state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    /// Stable id.
    pub id: EntryId,
    /// Thesis text.
    pub thesis: Thesis,
    /// Linkage ids (evidence refs, trade ids, strategy ids, etc.).
    pub linkage_ids: Vec<String>,
    /// Graded outcome when known.
    pub outcome: Option<OutcomeTag>,
    /// Review workflow position.
    pub review_state: ReviewWorkflowState,
    /// Creation timestamp (UTC).
    pub created_at: OffsetDateTime,
    /// Last update timestamp (UTC).
    pub updated_at: OffsetDateTime,
}

impl JournalEntry {
    /// Create a draft entry with thesis and linkage ids.
    pub fn draft(
        id: EntryId,
        thesis: Thesis,
        linkage_ids: Vec<String>,
        now: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            thesis,
            linkage_ids,
            outcome: None,
            review_state: ReviewWorkflowState::Draft,
            created_at: now,
            updated_at: now,
        }
    }

    /// Advance review workflow with light validation.
    pub fn transition(
        &mut self,
        next: ReviewWorkflowState,
        now: OffsetDateTime,
    ) -> Result<(), JournalError> {
        use ReviewWorkflowState::*;
        let ok = matches!(
            (self.review_state, next),
            (Draft, Submitted)
                | (Submitted, InReview)
                | (InReview, Accepted)
                | (InReview, Rejected)
                | (Rejected, Draft)
                | (Rejected, Submitted)
        );
        if !ok {
            return Err(JournalError::InvalidTransition {
                from: self.review_state,
                to: next,
            });
        }
        self.review_state = next;
        self.updated_at = now;
        Ok(())
    }

    /// Tag an outcome (typically after Accepted or on close).
    pub fn set_outcome(&mut self, tag: OutcomeTag, now: OffsetDateTime) {
        self.outcome = Some(tag);
        self.updated_at = now;
    }

    /// Attach an additional linkage id if not already present.
    pub fn link(&mut self, linkage_id: impl Into<String>) {
        let id = linkage_id.into();
        if !self.linkage_ids.contains(&id) {
            self.linkage_ids.push(id);
        }
    }

    /// Link an evidence reference by its hex record id.
    pub fn link_evidence(&mut self, evidence: &prismatik_market_data::EvidenceRef) {
        self.link(format!("evidence:{}", hex_record_id(&evidence.record_id)));
    }
}

fn hex_record_id(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Journal domain errors.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum JournalError {
    /// Illegal review workflow transition.
    #[error("invalid review transition {from:?} → {to:?}")]
    InvalidTransition {
        /// Current state.
        from: ReviewWorkflowState,
        /// Requested state.
        to: ReviewWorkflowState,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    #[test]
    fn entry_capture_thesis_linkage_outcome_and_review() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let mut entry = JournalEntry::draft(
            EntryId::new("je-1"),
            Thesis::new("Breakout above prior week high with rising volume"),
            vec!["evidence:abc".into(), "trade:t-9".into()],
            now,
        );
        assert_eq!(entry.review_state, ReviewWorkflowState::Draft);
        assert!(entry.outcome.is_none());
        assert_eq!(entry.linkage_ids.len(), 2);

        entry
            .transition(ReviewWorkflowState::Submitted, now)
            .unwrap();
        entry
            .transition(ReviewWorkflowState::InReview, now)
            .unwrap();
        entry
            .transition(ReviewWorkflowState::Accepted, now)
            .unwrap();
        entry.set_outcome(OutcomeTag::Win, now);
        assert_eq!(entry.outcome, Some(OutcomeTag::Win));
        assert_eq!(entry.review_state, ReviewWorkflowState::Accepted);
    }

    #[test]
    fn rejects_illegal_review_skip() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let mut entry = JournalEntry::draft(EntryId::new("je-2"), Thesis::new("x"), vec![], now);
        assert!(entry
            .transition(ReviewWorkflowState::Accepted, now)
            .is_err());
    }
}
