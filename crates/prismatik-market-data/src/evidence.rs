//! Evidence-plane contracts.

use prismatik_domain::ProviderId;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Reference to immutable evidence record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Evidence id.
    pub id: String,
    /// Source provider.
    pub provider: ProviderId,
    /// Retrieval timestamp.
    pub retrieved_at: OffsetDateTime,
}

/// Blind spot marker for incomplete evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlindSpot {
    /// Blind spot code.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
}

/// Evidence errors.
#[derive(Debug, thiserror::Error)]
pub enum EvidenceError {
    /// Missing evidence ref.
    #[error("missing evidence reference")]
    MissingReference,
}

/// Trait for values that can provide evidence links.
pub trait Concludes {
    /// Resolve evidence references attached to this conclusion.
    fn evidence_refs(&self) -> &[EvidenceRef];
}
