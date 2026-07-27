//! # prismatik-prismatik-market-data
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — market-data contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use chain::{AgreementPolicy, DivergenceAction, FailoverTrigger, ProviderChain};
pub use evidence::{BlindSpot, Concludes, EvidenceError, EvidenceRef};
pub use governor::{
    AdmissionDecision, BudgetGovernor, BudgetState, Entitlement, Permit, PriorityClass,
};
pub use lineage::{Lineage, TransformId};
pub use provider::{EntitlementSet, Provider, ProviderCapabilities, ProviderHealth};
pub use request::{CostUnits, EndpointId, ProviderRequest, ProviderResponse};

pub mod chain;
pub mod evidence;
pub mod governor;
pub mod ingest;
pub mod lineage;
pub mod provider;
pub mod request;
