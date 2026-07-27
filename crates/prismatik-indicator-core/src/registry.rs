//! Native indicator kind registry (P4-QM-13 floor).

use serde::{Deserialize, Serialize};

/// Closed-set of native indicator kinds shipped in this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeIndicatorKind {
    /// Simple moving average.
    Sma,
    /// Exponential moving average.
    Ema,
    /// Relative strength index (simple average gain/loss).
    Rsi,
    /// Rate of change.
    Roc,
    /// Bollinger middle band (SMA).
    BollingerMid,
    /// Weighted moving average.
    Wma,
    /// Population standard deviation of closes.
    StdDev,
    /// Average true range.
    Atr,
    /// MACD line (fast EMA − slow EMA).
    MacdLine,
    /// Stochastic %K.
    StochasticK,
    /// On-balance volume.
    Obv,
    /// Rolling volume-weighted average price.
    Vwap,
    /// Rolling minimum of closes.
    Min,
    /// Rolling maximum of closes.
    Max,
    /// Rolling range of closes.
    Range,
    /// Simple return (decimal).
    Returns,
    /// Bollinger upper band (SMA + k·σ).
    BollingerUpper,
    /// Bollinger lower band (SMA − k·σ).
    BollingerLower,
    /// MACD signal line (EMA of MACD line).
    MacdSignal,
    /// MACD histogram (MACD line − signal).
    MacdHist,
    /// Stochastic %D (SMA of %K).
    StochasticD,
    /// Williams %R.
    WilliamsR,
    /// Commodity Channel Index.
    Cci,
    /// Momentum (close − close[n]).
    Momentum,
    /// Rolling median of closes.
    Median,
    /// Rolling sum of closes.
    Sum,
    /// True range (single bar).
    TrueRange,
    /// Typical price (H+L+C)/3.
    TypicalPrice,
    /// Mid price (H+L)/2.
    MidPrice,
    /// Percent change (decimal return × 100).
    PercentChange,
}

impl NativeIndicatorKind {
    /// All native kinds in stable catalog order.
    pub const ALL: &'static [Self] = &[
        Self::Sma,
        Self::Ema,
        Self::Rsi,
        Self::Roc,
        Self::BollingerMid,
        Self::Wma,
        Self::StdDev,
        Self::Atr,
        Self::MacdLine,
        Self::StochasticK,
        Self::Obv,
        Self::Vwap,
        Self::Min,
        Self::Max,
        Self::Range,
        Self::Returns,
        Self::BollingerUpper,
        Self::BollingerLower,
        Self::MacdSignal,
        Self::MacdHist,
        Self::StochasticD,
        Self::WilliamsR,
        Self::Cci,
        Self::Momentum,
        Self::Median,
        Self::Sum,
        Self::TrueRange,
        Self::TypicalPrice,
        Self::MidPrice,
        Self::PercentChange,
    ];

    /// Descriptor `kind` string (matches [`crate::IndicatorDescriptor::kind`]).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sma => "sma",
            Self::Ema => "ema",
            Self::Rsi => "rsi",
            Self::Roc => "roc",
            Self::BollingerMid => "bb_mid",
            Self::Wma => "wma",
            Self::StdDev => "stddev",
            Self::Atr => "atr",
            Self::MacdLine => "macd_line",
            Self::StochasticK => "stoch_k",
            Self::Obv => "obv",
            Self::Vwap => "vwap",
            Self::Min => "min",
            Self::Max => "max",
            Self::Range => "range",
            Self::Returns => "returns",
            Self::BollingerUpper => "bb_upper",
            Self::BollingerLower => "bb_lower",
            Self::MacdSignal => "macd_signal",
            Self::MacdHist => "macd_hist",
            Self::StochasticD => "stoch_d",
            Self::WilliamsR => "williams_r",
            Self::Cci => "cci",
            Self::Momentum => "momentum",
            Self::Median => "median",
            Self::Sum => "sum",
            Self::TrueRange => "true_range",
            Self::TypicalPrice => "typical_price",
            Self::MidPrice => "mid_price",
            Self::PercentChange => "pct_change",
        }
    }
}

impl std::fmt::Display for NativeIndicatorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Registry listing every native indicator kind.
#[derive(Clone, Debug)]
pub struct IndicatorRegistry {
    kinds: &'static [NativeIndicatorKind],
}

impl IndicatorRegistry {
    /// Built-in native floor catalog.
    #[must_use]
    pub const fn native() -> Self {
        Self {
            kinds: NativeIndicatorKind::ALL,
        }
    }

    /// Number of registered kinds.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.kinds.len()
    }

    /// Whether the registry is empty (always false for the native catalog).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    /// Slice of all registered kinds.
    #[must_use]
    pub const fn kinds(&self) -> &'static [NativeIndicatorKind] {
        self.kinds
    }

    /// Whether `kind` is registered.
    #[must_use]
    pub fn contains(&self, kind: NativeIndicatorKind) -> bool {
        self.kinds.contains(&kind)
    }

    /// Whether a descriptor kind string is registered.
    #[must_use]
    pub fn contains_str(&self, kind: &str) -> bool {
        self.kinds.iter().any(|k| k.as_str() == kind)
    }

    /// Iterate registered kinds.
    pub fn iter(&self) -> impl Iterator<Item = NativeIndicatorKind> + '_ {
        self.kinds.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_registry_lists_all_kinds() {
        let reg = IndicatorRegistry::native();
        assert_eq!(reg.len(), 30);
        assert!(!reg.is_empty());
        let labels: Vec<&str> = reg.iter().map(NativeIndicatorKind::as_str).collect();
        assert_eq!(
            labels,
            vec![
                "sma",
                "ema",
                "rsi",
                "roc",
                "bb_mid",
                "wma",
                "stddev",
                "atr",
                "macd_line",
                "stoch_k",
                "obv",
                "vwap",
                "min",
                "max",
                "range",
                "returns",
                "bb_upper",
                "bb_lower",
                "macd_signal",
                "macd_hist",
                "stoch_d",
                "williams_r",
                "cci",
                "momentum",
                "median",
                "sum",
                "true_range",
                "typical_price",
                "mid_price",
                "pct_change"
            ]
        );
        assert!(reg.contains(NativeIndicatorKind::Wma));
        assert!(reg.contains(NativeIndicatorKind::MacdLine));
        assert!(reg.contains_str("returns"));
        assert!(!reg.contains_str("yata_rsi"));
    }
}
