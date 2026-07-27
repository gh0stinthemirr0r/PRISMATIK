//! Corporate action model.

use crate::AssetId;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Stable corporate-action identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorporateActionId(pub [u8; 32]);

/// Corporate action affecting one or more assets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorporateAction {
    /// Event identifier.
    pub id: CorporateActionId,
    /// Primary asset affected by this action.
    pub asset: AssetId,
    /// Effective event time.
    pub occurred_at: OffsetDateTime,
    /// Action payload.
    pub kind: CorporateActionKind,
}

/// Ratio helper type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ratio {
    /// Numerator component.
    pub numerator: u32,
    /// Denominator component.
    pub denominator: u32,
}

impl Ratio {
    /// Construct a positive ratio.
    pub fn new(numerator: u32, denominator: u32) -> Result<Self, RatioError> {
        if numerator == 0 || denominator == 0 {
            return Err(RatioError::ZeroComponent);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }
}

/// Merger terms.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergerTerms {
    /// Freeform terms in canonical text.
    pub description: String,
}

/// OCC memo reference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OccMemoRef {
    /// OCC memo identifier.
    pub memo_id: String,
}

/// Delisting reason taxonomy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelistingReason {
    /// Voluntary delisting.
    Voluntary,
    /// Involuntary delisting by exchange/regulator.
    Involuntary,
    /// Unknown reason.
    Unknown,
}

/// Supported corporate-action kinds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CorporateActionKind {
    /// Forward stock split.
    Split {
        /// Split numerator.
        numerator: u32,
        /// Split denominator.
        denominator: u32,
    },
    /// Reverse stock split.
    ReverseSplit {
        /// Reverse-split numerator.
        numerator: u32,
        /// Reverse-split denominator.
        denominator: u32,
    },
    /// Cash dividend in canonical decimal string format.
    CashDividend {
        /// Decimal amount in display currency.
        amount: String,
        /// ISO currency code.
        currency: String,
    },
    /// Stock dividend ratio.
    StockDividend {
        /// Distribution ratio.
        ratio: Ratio,
    },
    /// Spin-off into a child asset.
    SpinOff {
        /// Child asset created by the spin-off.
        child: AssetId,
        /// Distribution ratio.
        ratio: Ratio,
    },
    /// Merger event.
    Merger {
        /// Surviving asset.
        surviving: AssetId,
        /// Merger terms.
        terms: MergerTerms,
    },
    /// Ticker symbol change.
    TickerChange {
        /// Previous ticker.
        from: String,
        /// New ticker.
        to: String,
    },
    /// Delisting event.
    Delisting {
        /// Delisting reason.
        reason: DelistingReason,
    },
    /// OCC option adjustment.
    OptionAdjustment {
        /// Referenced OCC memo.
        memo: OccMemoRef,
    },
}

/// Ratio validation errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RatioError {
    /// Numerator or denominator was zero.
    #[error("ratio numerator and denominator must be > 0")]
    ZeroComponent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_rejects_zero() {
        assert!(Ratio::new(0, 1).is_err());
        assert!(Ratio::new(1, 0).is_err());
        assert!(Ratio::new(3, 2).is_ok());
    }
}
