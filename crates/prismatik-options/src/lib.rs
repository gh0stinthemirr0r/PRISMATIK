//! # prismatik-prismatik-options
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — options domain contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use chain::{ChainExpiry, ChainStrike, OptionsChain};
pub use flow::{FlowClassification, FlowConfidence, FlowPrint};
pub use types::{Greeks, Moneyness, OccSymbol, OptionAdjustmentFlag, OptionContract, OptionType};

/// Options type contracts.
pub mod types {
    /// Options contract type.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum OptionType {
        /// Call option.
        Call,
        /// Put option.
        Put,
    }

    /// Contract adjustment flag.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum OptionAdjustmentFlag {
        /// Standard contract.
        Standard,
        /// Adjusted contract.
        Adjusted,
    }

    /// OCC symbology wrapper.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct OccSymbol(pub String);

    /// Moneyness bucket.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Moneyness {
        /// In-the-money.
        Itm,
        /// At-the-money.
        Atm,
        /// Out-of-the-money.
        Otm,
    }

    /// Basic greek snapshot.
    #[derive(Clone, Debug, PartialEq)]
    pub struct Greeks {
        /// Delta.
        pub delta: f64,
        /// Gamma.
        pub gamma: f64,
        /// Vega.
        pub vega: f64,
        /// Theta.
        pub theta: f64,
    }

    /// Normalized option contract.
    #[derive(Clone, Debug, PartialEq)]
    pub struct OptionContract {
        /// OCC symbol.
        pub symbol: OccSymbol,
        /// Underlying asset id.
        pub underlying_asset_id: String,
        /// Expiry date as ISO string.
        pub expiry: String,
        /// Strike as decimal string.
        pub strike: String,
        /// Option side.
        pub option_type: OptionType,
        /// Adjustment state.
        pub adjustment: OptionAdjustmentFlag,
    }
}

/// Chain contracts.
pub mod chain {
    use crate::types::OptionContract;

    /// Strike bucket in an expiry.
    #[derive(Clone, Debug, PartialEq)]
    pub struct ChainStrike {
        /// Strike value as decimal string.
        pub strike: String,
        /// Contracts at this strike.
        pub contracts: Vec<OptionContract>,
    }

    /// Expiry bucket in chain.
    #[derive(Clone, Debug, PartialEq)]
    pub struct ChainExpiry {
        /// Expiry date.
        pub expiry: String,
        /// Strike rows.
        pub strikes: Vec<ChainStrike>,
    }

    /// Options chain surface.
    #[derive(Clone, Debug, PartialEq)]
    pub struct OptionsChain {
        /// Underlying asset id.
        pub underlying_asset_id: String,
        /// Expiry buckets.
        pub expiries: Vec<ChainExpiry>,
    }
}

/// Flow contracts.
pub mod flow {
    /// Confidence level for flow classification.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum FlowConfidence {
        /// Low confidence.
        Low,
        /// Medium confidence.
        Medium,
        /// High confidence.
        High,
    }

    /// Flow side classification.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum FlowClassification {
        /// Buyer initiated.
        BuyerInitiated,
        /// Seller initiated.
        SellerInitiated,
        /// Unclassified by design.
        Unclassified,
    }

    /// Normalized flow print.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct FlowPrint {
        /// Contract symbol.
        pub occ_symbol: String,
        /// Classification.
        pub classification: FlowClassification,
        /// Confidence.
        pub confidence: FlowConfidence,
    }
}
