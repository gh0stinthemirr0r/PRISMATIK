//! Budget governor contracts.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Entitlement requirement key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Entitlement(pub String);

/// Scheduling priority class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorityClass {
    /// User interactive.
    Interactive,
    /// Background sync.
    Background,
}

/// Granted permit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permit {
    /// Permit token value.
    pub token: String,
}

/// Budget state snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetState {
    /// Priority class.
    pub class: PriorityClass,
    /// Remaining units.
    pub remaining: u32,
    /// Reset timestamp.
    pub resets_at: OffsetDateTime,
}

/// Request admission decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AdmissionDecision {
    /// Request may proceed.
    Admit {
        /// Granted permit token.
        permit: Permit,
    },
    /// Request deferred with exact retry time.
    Defer {
        /// Next eligible request time.
        retry_at: OffsetDateTime,
        /// Queue position.
        position: usize,
    },
    /// Budget exhausted.
    BudgetExhausted {
        /// Reset timestamp.
        resets_at: OffsetDateTime,
        /// Priority class.
        class: PriorityClass,
    },
    /// Missing entitlement.
    NotEntitled {
        /// Required entitlement.
        required: Entitlement,
    },
}

/// Governor interface.
pub trait BudgetGovernor: Send + Sync {
    /// Admit or defer a request for class.
    fn admit(&self, class: PriorityClass) -> AdmissionDecision;
    /// Return budget state for class.
    fn state(&self, class: PriorityClass) -> BudgetState;
}
