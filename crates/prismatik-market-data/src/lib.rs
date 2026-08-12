//! # prismatik-market-data
//!
//! Layer 2 — Domain.
//!
//! Spec: `DOCS/spec/CRATE_ARCHITECTURE.md`.
//! Status: market-data contracts + live provider adapters.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use chain::{AgreementPolicy, DivergenceAction, FailoverTrigger, ProviderChain};
pub use chain_exec::{FetchMarkets, FetchMarketsError, ProviderChainExecutor, ProviderChainResult};
pub use corpus::{
    AcquisitionMethod, AppendOutcome, CorpusError, IngestManifestId, Observation, ObservationDraft,
    ObservationId, ObservationLog, PayloadRetention, PermittedMetadata, PermittedMetadataField,
    RedistributionPolicy, RetrievalAttemptId, SourceId, SourcePolicy, SourcePolicyRegistry,
    SourcePolicyVersion, TemporalAnomaly,
};
pub use evidence::{BlindSpot, Concludes, EvidenceError, EvidenceRef};
pub use feed_registry::{FeedRegistration, FeedRegistry, FeedRegistryError};
/// Budget-side entitlement key (rate-budget permit), kept distinct from the
/// provider-account [`Entitlement`](crate::provider::Entitlement).
pub use governor::Entitlement as BudgetEntitlement;
pub use governor::{
    AdmissionDecision, BudgetGovernor, BudgetState, GcraBudgetGovernor, GovernorQuota, Permit,
    PriorityClass,
};
pub use lineage::{Lineage, TransformId};
pub use normalize::{
    InMemoryRawStore, NormalizeError as IngestError, NormalizePipeline, NormalizedBatch,
    RawObservation, RawStore, RawStoreError,
};
pub use orderbook::{
    reconstruct_book, BookAnomaly, BookEvent, BookEventKind, BookLevel, BookSide, BookSnapshot,
    DepthExecution,
};
pub use prediction_venues::{parse_kalshi_orderbook, parse_polymarket_orderbook, VenueBookError};
pub use provider::{
    Capability, Entitlement, EntitlementSet, HealthStatus, Provider, ProviderCapabilities,
    ProviderHealth,
};
pub use request::{CostUnits, EndpointId, ProviderRequest, ProviderResponse};
pub use types::*;

pub mod chain;
pub mod corpus;
pub mod evidence;
/// Lawful RSS/Atom source registry and fair conditional-fetch scheduler.
pub mod feed_registry;
pub mod governor;
pub mod ingest;
pub mod lineage;
/// Raw-first normalized observation pipeline.
pub mod normalize;
/// Deterministic L2 reconstruction and executable-depth analytics.
pub mod orderbook;
/// Read-only prediction-venue response normalization.
pub mod prediction_venues;
pub mod provider;
pub mod request;

pub mod adapters;
pub mod cassette;
pub mod chain_exec;
pub mod http;
pub mod types;

pub use cassette::{CassetteExchange, CassetteTransport};
