//! # prismatik-prismatik-observability
//!
//! Layer 3 — Extension and AI
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — observability setup contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

/// Observability configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservabilityConfig {
    /// Service name.
    pub service_name: String,
    /// Environment name.
    pub environment: String,
    /// Exporter endpoint string.
    pub otlp_endpoint: String,
}

/// Determinism telemetry attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeterminismAttributes {
    /// Context digest.
    pub context_digest: String,
    /// Seed root hash.
    pub seed_hash: String,
}

/// Build span name from domain and action.
pub fn span_name(domain: &str, action: &str) -> String {
    format!("{domain}.{action}")
}
