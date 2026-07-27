//! Observation-delay primitives.

use serde::{Deserialize, Serialize};

/// Delay between event time and observation availability, in milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationDelay {
    millis: i64,
}

impl ObservationDelay {
    /// Construct from milliseconds.
    pub fn from_millis(millis: i64) -> Self {
        Self {
            millis: millis.max(0),
        }
    }

    /// Delay in milliseconds.
    pub fn millis(self) -> i64 {
        self.millis
    }
}
