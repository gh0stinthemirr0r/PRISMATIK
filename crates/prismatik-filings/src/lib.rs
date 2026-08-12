//! # prismatik-prismatik-filings
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — filings domain contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use ownership::{
    holdings_observable_as_of, parse_13f, parse_form4, FilingParseError, Form13fHolding,
    Form4Transaction,
};

/// Point-in-time-safe institutional ownership and insider-transaction parsing.
pub mod ownership;

/// Filing types.
/// Filing type contracts.
pub mod types {
    /// Filing kind.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum FilingKind {
        /// US SEC 8-K.
        Form8K,
        /// US SEC 10-Q.
        Form10Q,
        /// US SEC 10-K.
        Form10K,
        /// Unknown kind.
        Unknown,
    }

    /// Normalized filing record.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct FilingRecord {
        /// Issuer asset id.
        pub issuer_asset_id: String,
        /// Filing identifier.
        pub filing_id: String,
        /// Filing kind.
        pub kind: FilingKind,
        /// Filed-at timestamp.
        pub filed_at: String,
        /// Source URL.
        pub source_url: String,
    }
}

/// Filing parser contracts.
/// Parser contracts.
pub mod parser {
    use crate::types::FilingRecord;

    /// Filing parse error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ParseError {
        /// Error message.
        pub message: String,
    }

    /// Filing parser contract.
    pub trait FilingParser: Send + Sync {
        /// Parse normalized filing.
        fn parse(&self, raw: &str) -> Result<FilingRecord, ParseError>;
    }
}

/// Filing provider extension.
/// Provider contracts.
pub mod provider {
    /// Provider capability.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ProviderCapability {
        /// Provider id.
        pub provider_id: String,
        /// Supports historical backfill.
        pub supports_historical: bool,
    }
}
