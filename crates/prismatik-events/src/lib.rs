//! # prismatik-prismatik-events
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — normalized event contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

/// Event domain types.
/// Event type contracts.
pub mod types {
    /// Event category.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum EventCategory {
        /// Macro-economic event.
        Macro,
        /// Earnings event.
        Earnings,
        /// Corporate-action event.
        CorporateAction,
        /// Unknown category.
        Unknown,
    }

    /// Normalized event.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct EventRecord {
        /// Event id.
        pub id: String,
        /// Event category.
        pub category: EventCategory,
        /// Event timestamp.
        pub event_time: String,
        /// Event description.
        pub description: String,
    }
}

/// Event parser contracts.
/// Parser contracts.
pub mod parser {
    use crate::types::EventRecord;

    /// Parse error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ParseError {
        /// Error message.
        pub message: String,
    }

    /// Event parser contract.
    pub trait EventParser: Send + Sync {
        /// Parse normalized event.
        fn parse(&self, raw: &str) -> Result<EventRecord, ParseError>;
    }
}

/// Event provider extension.
/// Provider contracts.
pub mod provider {
    /// Provider capabilities.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ProviderCapability {
        /// Provider id.
        pub provider_id: String,
        /// Supports realtime feed.
        pub supports_realtime: bool,
    }
}
