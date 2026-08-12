//! # prismatik-prismatik-journal
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — thesis journal contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use behavior::{
    analyze_behavior, BehaviorConfig, BehaviorError, BehaviorFinding, BehaviorKind, TradeReview,
};
pub use entry::{EntryId, JournalEntry, Outcome, OutcomeTag, Thesis};
pub use feedback::{FeedbackSignal, ScoreAdjustment};
pub use memory::{MemoryLayer, MemoryLoop, MemoryRecord, TriggerWinRate};

/// Deterministic behavioral review of completed trading activity.
pub mod behavior;

/// Entry contracts.
pub mod entry {
    /// Stable journal entry id.
    pub type EntryId = String;

    /// Strategy thesis block.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Thesis {
        /// Thesis statement.
        pub statement: String,
        /// Trigger description.
        pub trigger: String,
    }

    /// Outcome tag.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum OutcomeTag {
        /// Positive outcome.
        Win,
        /// Negative outcome.
        Loss,
        /// Neutral outcome.
        Flat,
    }

    /// Entry outcome.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Outcome {
        /// Outcome tag.
        pub tag: OutcomeTag,
        /// Outcome notes.
        pub notes: String,
    }

    /// Journal entry with linked evidence references.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct JournalEntry {
        /// Entry id.
        pub id: EntryId,
        /// Strategy thesis.
        pub thesis: Thesis,
        /// Linked evidence identifiers.
        pub evidence_refs: Vec<String>,
        /// Recorded outcome.
        pub outcome: Outcome,
    }
}

/// Memory-loop contracts.
pub mod memory {
    /// Memory layer tier.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum MemoryLayer {
        /// Detailed short-term memory.
        Layer1,
        /// Pattern synthesis memory.
        Layer2,
        /// Principle memory.
        Layer3,
    }

    /// Trigger win-rate summary.
    #[derive(Clone, Debug, PartialEq)]
    pub struct TriggerWinRate {
        /// Trigger key.
        pub trigger: String,
        /// Win-rate in [0, 1].
        pub win_rate: f64,
    }

    /// Single memory record.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct MemoryRecord {
        /// Layer.
        pub layer: MemoryLayer,
        /// Compressed statement.
        pub summary: String,
    }

    /// Memory loop contract.
    pub trait MemoryLoop: Send + Sync {
        /// Ingest a record.
        fn record(&mut self, record: MemoryRecord);
        /// Return all records.
        fn records(&self) -> &[MemoryRecord];
    }
}

/// Feedback contracts.
pub mod feedback {
    /// Feedback signal from realized performance.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct FeedbackSignal {
        /// Confidence delta in [-1, 1].
        pub confidence_delta: f64,
    }

    /// Scoring adjustment.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct ScoreAdjustment {
        /// Weight delta in [-1, 1].
        pub weight_delta: f64,
    }
}
