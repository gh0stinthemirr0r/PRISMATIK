//! Data quality scoring.

use serde::{Deserialize, Serialize};

/// Normalized quality score in `[0.0, 1.0]`.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DataQualityScore(f32);

impl DataQualityScore {
    /// Create a clamped quality score.
    pub fn new(score: f32) -> Self {
        Self(score.clamp(0.0, 1.0))
    }

    /// Read score value.
    pub fn get(self) -> f32 {
        self.0
    }

    /// Perfect-quality constant (1.0). Used by providers that return
    /// provider-native identifiers (e.g. SEC EDGAR, FRED) where the value is
    /// authoritative rather than observed and reconciled.
    pub const PERFECT: DataQualityScore = DataQualityScore(1.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_is_clamped() {
        assert_eq!(DataQualityScore::new(-1.0).get(), 0.0);
        assert_eq!(DataQualityScore::new(2.0).get(), 1.0);
    }
}
