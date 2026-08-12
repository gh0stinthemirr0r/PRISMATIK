//! # prismatik-identity
//!
//! Layer 0 — Foundational (no workspace deps beyond determinism).
//!
//! Canonical, content-addressed, bitemporal asset identity (invariant I5).
//! Every venue-specific ticker, FIGI, ISIN, or on-chain address resolves to a
//! single [`AssetId`] *as of* a point in time; transitions (renames, splits,
//! mergers, spin-offs) are an append-only chain that never rewrites history.
//!
//! Spec: `DOCS/spec/CRATE_ARCHITECTURE.md` §1.2; Wave 2 DoD #1.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub mod asset_id;
pub mod corporate_action;
/// Hand-checked identity continuity corpus (Wave 2 DoD #1).
pub mod corpus;
pub mod external_id;
/// Entropy-backed synthetic `AssetId` factory.
pub mod factory;
/// Bitemporal OpenFIGI mapping surface.
pub mod openfigi;
/// Symbology resolver contract + in-memory implementation.
pub mod resolver;

pub use asset_id::{AssetId, MicCode, VenueId};
pub use corporate_action::{
    CorporateAction, CorporateActionId, CorporateActionKind, DelistingReason, MergerTerms,
    OccMemoRef, Ratio,
};
pub use external_id::ExternalIdentifier;
pub use openfigi::{FigiMapping, OpenFigiMapper};
pub use resolver::{
    CanonicalIdentityRecord, IdentityTransition, InMemoryResolver, SymbologyError,
    SymbologyResolver, ValidityInterval,
};

// Re-export the corpus surface at the crate root so the Wave 2 DoD #1 test
// (`tests/identity_corpus.rs`) and downstream callers can import
// `IdentityCorpus`, `IdentityEventType`, and `assert_event_as_of` directly.
pub use corpus::{assert_event_as_of, IdentityCorpus, IdentityEvent, IdentityEventType};
