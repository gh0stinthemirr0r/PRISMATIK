//! # prismatik-prismatik-domain
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — shared domain primitives.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use currency::CurrencyCode;
pub use dataset_version::{DatasetVersionId, DatasetVersionRef};
pub use layer::DataLayer;
pub use prediction::{
    best_cross_venue_market, ContractId, EventId, EventMarket, EventStatus, Outcome,
    PredictionContract, ProbabilityPpm, VenueMarketQuote,
};
pub use prediction_logic::{
    detect_probability_inconsistencies, LogicalConstraint, ProbabilityInconsistency,
};
pub use provider_id::ProviderId;
pub use quality::DataQualityScore;
pub use resolution::{
    assess_resolution_risk, compare_contract_semantics, ContractSemanticComparison,
    ResolutionPrecedent, ResolutionRiskAssessment, ResolutionRiskReason, ResolutionRuleSnapshot,
};

pub mod currency;
pub mod dataset_version;
pub mod layer;
pub mod prediction;
pub mod prediction_logic;
pub mod provider_id;
pub mod quality;
pub mod resolution;
