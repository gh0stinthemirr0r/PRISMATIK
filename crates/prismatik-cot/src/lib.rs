//! # prismatik-prismatik-cot
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — commitments of traders contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

/// COT domain types.
/// COT type contracts.
pub mod types {
    /// COT report classification.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum CotReportType {
        /// Legacy report.
        Legacy,
        /// Disaggregated report.
        Disaggregated,
        /// Traders in financial futures.
        FinancialFutures,
    }

    /// Normalized COT row.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CotRecord {
        /// Market code.
        pub market_code: String,
        /// Report week date.
        pub week: String,
        /// Report type.
        pub report_type: CotReportType,
        /// Commercial long contracts.
        pub commercial_long: i64,
        /// Commercial short contracts.
        pub commercial_short: i64,
    }
}

/// COT parser contracts.
/// Parser contracts.
pub mod parser {
    use crate::types::CotRecord;

    /// Parse error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ParseError {
        /// Error message.
        pub message: String,
    }

    /// COT parser contract.
    pub trait CotParser: Send + Sync {
        /// Parse normalized row.
        fn parse(&self, raw: &str) -> Result<CotRecord, ParseError>;
    }
}

/// Provider extension point.
/// Provider contracts.
pub mod provider {
    /// Provider profile.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ProviderProfile {
        /// Provider id.
        pub provider_id: String,
        /// Supported report types.
        pub report_types: Vec<String>,
    }
}
