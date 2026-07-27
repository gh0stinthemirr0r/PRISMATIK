//! External indicator adapter floor (`P4-QM-11`).
//!
//! Typed façade documenting how an external indicator crate would map into
//! [`crate::Indicator`]. The workspace does **not** link `yata` yet (supply-chain
//! + golden-vector ownership stay native). This module proves the adapter
//!   surface and returns typed errors when construction is requested.
//!
//! The optional [`ExternalIndicatorSpec::backend_id`] field records the future
//! crate id (e.g. `"yata"`) as an integration label — not as a product type name.

use crate::{Indicator, IndicatorDescriptor, WarmupBehavior};
use thiserror::Error;

/// Errors from the deferred external indicator adapter.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExternalIndicatorError {
    /// Workspace has not linked the external indicator crate.
    #[error("external indicator crate not linked at P4-QM-11 floor: {0}")]
    NotLinked(String),
    /// Unknown external indicator id.
    #[error("unknown external indicator id: {0}")]
    UnknownId(String),
}

/// Catalog entry describing an externally-backed indicator that *would* be adapted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalIndicatorSpec {
    /// Stable registry id within the external backend (e.g. `"sma"`, `"rsi"`).
    pub indicator_id: String,
    /// Native kind string used in [`IndicatorDescriptor::kind`].
    pub native_kind: String,
    /// Warmup bars required.
    pub warmup_len: usize,
    /// Future crate / backend id (integration label), e.g. `"yata"`.
    pub backend_id: String,
}

impl ExternalIndicatorSpec {
    /// Construct a catalog entry with the default deferred backend id `"yata"`.
    #[must_use]
    pub fn new(
        indicator_id: impl Into<String>,
        native_kind: impl Into<String>,
        warmup_len: usize,
    ) -> Self {
        Self {
            indicator_id: indicator_id.into(),
            native_kind: native_kind.into(),
            warmup_len,
            backend_id: "yata".into(),
        }
    }

    /// Build an [`IndicatorDescriptor`] for provenance tagging.
    #[must_use]
    pub fn descriptor(&self) -> IndicatorDescriptor {
        IndicatorDescriptor {
            kind: self.native_kind.clone(),
            name: format!("external({})", self.indicator_id),
            warmup_len: self.warmup_len,
            warmup_behavior: WarmupBehavior::Nan,
            provenance: "external-indicator-deferred".into(),
        }
    }
}

/// Floor catalog of external indicator ids we intend to adapt once a dep lands.
#[must_use]
pub fn external_indicator_floor_catalog() -> Vec<ExternalIndicatorSpec> {
    vec![
        ExternalIndicatorSpec::new("sma", "sma", 0),
        ExternalIndicatorSpec::new("ema", "ema", 0),
        ExternalIndicatorSpec::new("rsi", "rsi", 1),
        ExternalIndicatorSpec::new("macd", "macd_line", 0),
        ExternalIndicatorSpec::new("stoch", "stoch_k", 0),
        ExternalIndicatorSpec::new("atr", "atr", 1),
        ExternalIndicatorSpec::new("cci", "cci", 0),
        ExternalIndicatorSpec::new("willr", "willr", 0),
    ]
}

/// Attempt to construct an externally-backed [`Indicator`]. Always fails closed
/// until the backend crate is a workspace dependency with cargo-vet coverage.
pub fn try_adapt_external_indicator(
    spec: &ExternalIndicatorSpec,
) -> Result<Box<dyn Indicator>, ExternalIndicatorError> {
    let _ = spec.descriptor();
    Err(ExternalIndicatorError::NotLinked(format!(
        "requested backend_id={} indicator_id={} — use native IndicatorRegistry until linked",
        spec.backend_id, spec.indicator_id
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_non_empty_and_descriptors_tagged() {
        let cat = external_indicator_floor_catalog();
        assert!(!cat.is_empty());
        for spec in &cat {
            let d = spec.descriptor();
            assert_eq!(d.provenance, "external-indicator-deferred");
            assert!(d.name.starts_with("external("));
            assert_eq!(spec.backend_id, "yata");
        }
    }

    #[test]
    fn try_adapt_fails_closed() {
        let spec = ExternalIndicatorSpec::new("sma", "sma", 0);
        let err = try_adapt_external_indicator(&spec)
            .err()
            .expect("must fail");
        assert!(matches!(err, ExternalIndicatorError::NotLinked(_)));
    }
}
