//! Post-trade learning: realized vs expected (`P6-QM-06`).

use crate::entry::{EntryId, OutcomeTag};
use serde::{Deserialize, Serialize};

/// Realized-versus-expected learning record for a closed journal entry / trade.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostTradeLearningRecord {
    /// Journal entry this learning attaches to.
    pub entry_id: EntryId,
    /// Optional trade / fill linkage id.
    pub trade_id: Option<String>,
    /// Realized P&L in integer micros (currency minor units × 1e6 scale).
    pub realized_pnl_micros: i64,
    /// Expected P&L in integer micros at thesis time.
    pub expected_pnl_micros: i64,
    /// Derived outcome suggestion from the PnL delta (caller may override).
    pub suggested_outcome: OutcomeTag,
}

impl PostTradeLearningRecord {
    /// Build a record and suggest an outcome from realized vs expected.
    pub fn new(
        entry_id: EntryId,
        trade_id: Option<String>,
        realized_pnl_micros: i64,
        expected_pnl_micros: i64,
    ) -> Self {
        let suggested_outcome = Self::suggest(realized_pnl_micros, expected_pnl_micros);
        Self {
            entry_id,
            trade_id,
            realized_pnl_micros,
            expected_pnl_micros,
            suggested_outcome,
        }
    }

    /// `realized - expected` in micros.
    pub fn delta_micros(&self) -> i64 {
        self.realized_pnl_micros - self.expected_pnl_micros
    }

    /// Thesis validation: realized met or beat expectation (within scratch band).
    pub fn thesis_validated(&self, scratch_band_micros: i64) -> bool {
        self.delta_micros() >= -scratch_band_micros.abs()
            && self.realized_pnl_micros >= -scratch_band_micros.abs()
    }

    fn suggest(realized: i64, expected: i64) -> OutcomeTag {
        let delta = realized - expected;
        const SCRATCH: i64 = 1_000; // 0.001 currency unit in micros
        if realized.abs() <= SCRATCH && expected.abs() <= SCRATCH {
            OutcomeTag::Scratch
        } else if delta >= -SCRATCH && realized > SCRATCH {
            OutcomeTag::Win
        } else if realized < -SCRATCH || delta < -SCRATCH {
            OutcomeTag::Loss
        } else {
            OutcomeTag::Incomplete
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realized_vs_expected_micros() {
        let rec = PostTradeLearningRecord::new(
            EntryId::new("je-1"),
            Some("trade-1".into()),
            2_500_000,
            2_000_000,
        );
        assert_eq!(rec.delta_micros(), 500_000);
        assert_eq!(rec.suggested_outcome, OutcomeTag::Win);
        assert!(rec.thesis_validated(100));

        let loss = PostTradeLearningRecord::new(EntryId::new("je-2"), None, -1_000_000, 500_000);
        assert_eq!(loss.suggested_outcome, OutcomeTag::Loss);
        assert!(!loss.thesis_validated(100));
    }
}
