//! # prismatik-prismatik-crypto
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — crypto market contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

/// Normalized crypto instruments and bars.
/// Crypto type contracts.
pub mod types {
    /// Crypto instrument.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CryptoInstrument {
        /// Canonical asset id.
        pub asset_id: String,
        /// Exchange venue id.
        pub venue_id: String,
        /// Exchange symbol.
        pub symbol: String,
        /// Base currency.
        pub base_currency: String,
        /// Quote currency.
        pub quote_currency: String,
    }

    /// OHLCV bar.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CryptoBar {
        /// Event time.
        pub event_time: String,
        /// Open price.
        pub open: String,
        /// High price.
        pub high: String,
        /// Low price.
        pub low: String,
        /// Close price.
        pub close: String,
        /// Volume.
        pub volume: String,
    }
}

/// Provider parser contracts.
/// Parser contracts.
pub mod parser {
    use crate::types::{CryptoBar, CryptoInstrument};

    /// Parser error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ParseError {
        /// Error message.
        pub message: String,
    }

    /// Parser contract for provider payloads.
    pub trait CryptoParser: Send + Sync {
        /// Parse instrument metadata.
        fn parse_instrument(&self, raw: &str) -> Result<CryptoInstrument, ParseError>;
        /// Parse market bar.
        fn parse_bar(&self, raw: &str) -> Result<CryptoBar, ParseError>;
    }
}

/// Provider extension point.
/// Provider contracts.
pub mod provider {
    /// Provider capability declaration.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ProviderCapability {
        /// Provider id.
        pub provider_id: String,
        /// Whether websocket streaming is supported.
        pub supports_streaming: bool,
    }
}
