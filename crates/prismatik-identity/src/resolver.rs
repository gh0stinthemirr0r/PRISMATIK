//! Bitemporal symbology resolution.
//!
//! Spec: `DOCS/spec/CRATE_ARCHITECTURE.md` §1.2; Wave 2 DoD #1.
//!
//! Two contracts live here:
//! - the [`SymbologyResolver`] trait (the port every venue/mapper implements),
//! - the [`InMemoryResolver`] reference implementation, a bitemporal map from
//!   [`ExternalIdentifier`] to [`AssetId`] that resolves *as of* a point in
//!   time and replays an append-only transition chain.
//!
//! The resolver is the single place that enforces point-in-time identity
//! correctness (invariant I5): an identifier that did not yet exist, or that
//! has since been dropped, must not resolve at the wrong timestamp.

use crate::{AssetId, ExternalIdentifier, VenueId};
use prismatik_determinism::{ArtifactRef, DetMap};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Canonical bitemporal identity record used to derive `AssetId`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalIdentityRecord {
    /// Primary provider-facing identifier.
    pub primary_identifier: ExternalIdentifier,
    /// Optional venue context.
    pub venue: Option<VenueId>,
    /// Inclusive validity lower bound.
    pub valid_from: Option<OffsetDateTime>,
    /// Exclusive validity upper bound.
    pub valid_to: Option<OffsetDateTime>,
}

/// Inclusive-lower / exclusive-upper validity window for an identifier binding.
///
/// `None` bounds are open. An identifier is live at `as_of` when
/// `valid_from <= as_of` and `valid_to` is either open or strictly greater
/// than `as_of`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidityInterval {
    /// Inclusive lower bound (`None` = since the beginning).
    pub valid_from: Option<OffsetDateTime>,
    /// Exclusive upper bound (`None` = open-ended).
    pub valid_to: Option<OffsetDateTime>,
}

impl ValidityInterval {
    /// Construct a validity interval. `valid_to` must be strictly after
    /// `valid_from` when both are present.
    pub fn new(
        valid_from: OffsetDateTime,
        valid_to: Option<OffsetDateTime>,
    ) -> Result<Self, SymbologyError> {
        if let Some(to) = valid_to {
            if to <= valid_from {
                return Err(SymbologyError::Storage(format!(
                    "valid_to ({to}) must be after valid_from ({valid_from})"
                )));
            }
        }
        Ok(Self {
            valid_from: Some(valid_from),
            valid_to,
        })
    }

    /// Always-open interval (live from the beginning of time).
    pub fn always() -> Self {
        Self {
            valid_from: None,
            valid_to: None,
        }
    }

    /// True when `as_of` falls inside `[valid_from, valid_to)`.
    pub fn contains(&self, as_of: OffsetDateTime) -> bool {
        let after_lower = self.valid_from.is_none_or(|from| as_of >= from);
        let before_upper = self.valid_to.is_none_or(|to| as_of < to);
        after_lower && before_upper
    }
}

/// One append-only identity transition in an asset's history chain.
///
/// Transitions describe how an asset's identifiers change over time: a ticker
/// rename adds new tickers and drops old ones on the effective date, while
/// preserving the underlying canonical [`AssetId`]. Transitions never mutate
/// past bindings; they append to the chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityTransition {
    /// The asset this transition concerns.
    pub asset_id: AssetId,
    /// When the transition takes effect.
    pub effective_at: OffsetDateTime,
    /// Identifiers that begin identifying `asset_id` at `effective_at`.
    #[serde(default)]
    pub new_identifiers: Vec<ExternalIdentifier>,
    /// Identifiers that stop identifying `asset_id` at `effective_at`.
    #[serde(default)]
    pub dropped_identifiers: Vec<ExternalIdentifier>,
    /// Optional human-readable reason / memo.
    #[serde(default)]
    pub memo: Option<String>,
}

/// Bitemporal symbology resolver contract.
pub trait SymbologyResolver: Send + Sync {
    /// Resolve an external identifier as of a specific point in time.
    ///
    /// Returns `Ok(None)` when the identifier legitimately did not map to any
    /// asset at `as_of` (e.g. a ticker before its listing or after its delist).
    fn resolve_as_of(
        &self,
        identifier: &ExternalIdentifier,
        as_of: OffsetDateTime,
    ) -> Result<Option<AssetId>, SymbologyError>;

    /// Return known identifiers for an asset at `as_of`.
    fn identifiers_as_of(
        &self,
        asset: &AssetId,
        as_of: OffsetDateTime,
    ) -> Result<Vec<ExternalIdentifier>, SymbologyError>;

    /// Return full identity transition chain for an asset, in effective order.
    fn identity_chain(&self, asset: &AssetId) -> Result<Vec<IdentityTransition>, SymbologyError>;

    /// Return the pinned snapshot artifact this resolver was loaded from.
    fn snapshot_artifact(&self) -> &ArtifactRef;
}

/// In-memory bitemporal resolver backed by a pinned symbology snapshot.
///
/// Bindings are stored per identifier as a vector of `(AssetId, ValidityInterval)`
/// rows, and transitions are stored per asset in effective order. All iteration
/// over bindings uses a deterministic [`DetMap`] so resolve results are stable
/// across runs and machines (invariant I3).
#[derive(Debug)]
pub struct InMemoryResolver {
    artifact: ArtifactRef,
    /// identifier -> ordered bindings (validity windows for each asset).
    bindings: DetMap<ExternalIdentifier, Vec<(AssetId, ValidityInterval)>>,
    /// asset -> ordered transition chain.
    chains: DetMap<AssetId, Vec<IdentityTransition>>,
}

impl InMemoryResolver {
    /// Construct an empty resolver anchored to a pinned snapshot artifact.
    pub fn new(artifact: ArtifactRef) -> Self {
        Self {
            artifact,
            bindings: DetMap::default(),
            chains: DetMap::default(),
        }
    }

    /// Insert an always-open binding (live from the beginning of time).
    pub fn insert(&mut self, identifier: ExternalIdentifier, asset: AssetId) {
        self.insert_valid(identifier, asset, ValidityInterval::always());
    }

    /// Insert a time-bounded binding.
    pub fn insert_valid(
        &mut self,
        identifier: ExternalIdentifier,
        asset: AssetId,
        interval: ValidityInterval,
    ) {
        self.bindings
            .entry(identifier)
            .or_default()
            .push((asset, interval));
    }

    /// Append a transition to the asset's chain. The transition's
    /// `new_identifiers` / `dropped_identifiers` are *also* applied to the
    /// binding table so that resolves pick them up immediately.
    pub fn insert_transition(&mut self, transition: IdentityTransition) {
        let asset = transition.asset_id;
        let effective = transition.effective_at;
        // Drop the old identifiers starting at the effective instant: give them
        // an exclusive upper bound of `effective`.
        for dropped in &transition.dropped_identifiers {
            if let Some(rows) = self.bindings.get_mut(dropped) {
                for (a, interval) in rows.iter_mut() {
                    if *a == asset && interval.valid_to.is_none() {
                        interval.valid_to = Some(effective);
                    }
                }
            }
        }
        // Add the new identifiers as live from the effective instant.
        for added in &transition.new_identifiers {
            self.insert_valid(
                added.clone(),
                asset,
                ValidityInterval {
                    valid_from: Some(effective),
                    valid_to: None,
                },
            );
        }
        self.chains.entry(asset).or_default().push(transition);
    }

    /// Number of distinct identifiers currently bound.
    pub fn identifier_count(&self) -> usize {
        self.bindings.len()
    }
}

impl SymbologyResolver for InMemoryResolver {
    fn resolve_as_of(
        &self,
        identifier: &ExternalIdentifier,
        as_of: OffsetDateTime,
    ) -> Result<Option<AssetId>, SymbologyError> {
        let Some(rows) = self.bindings.get(identifier) else {
            return Ok(None);
        };
        // Bindings are append-only; the first live row in insertion order is
        // the canonical answer (cassettes are authored not to overlap).
        for (asset, interval) in rows {
            if interval.contains(as_of) {
                return Ok(Some(*asset));
            }
        }
        Ok(None)
    }

    fn identifiers_as_of(
        &self,
        asset: &AssetId,
        as_of: OffsetDateTime,
    ) -> Result<Vec<ExternalIdentifier>, SymbologyError> {
        let mut out = Vec::new();
        for (identifier, rows) in &self.bindings {
            for (a, interval) in rows {
                if a == asset && interval.contains(as_of) {
                    out.push(identifier.clone());
                    break;
                }
            }
        }
        // Deterministic ordering for stable serialization.
        out.sort_by(sort_identifiers);
        Ok(out)
    }

    fn identity_chain(&self, asset: &AssetId) -> Result<Vec<IdentityTransition>, SymbologyError> {
        Ok(self.chains.get(asset).cloned().unwrap_or_default())
    }

    fn snapshot_artifact(&self) -> &ArtifactRef {
        &self.artifact
    }
}

/// Stable ordering for [`ExternalIdentifier`] variants (deterministic output).
fn sort_identifiers(a: &ExternalIdentifier, b: &ExternalIdentifier) -> std::cmp::Ordering {
    // Order by serialized form — stable and total.
    let sa = serde_json::to_string(a).unwrap_or_default();
    let sb = serde_json::to_string(b).unwrap_or_default();
    sa.cmp(&sb)
}

/// Resolver failures.
#[derive(Debug, thiserror::Error)]
pub enum SymbologyError {
    /// No mapping existed at requested time.
    #[error("no mapping for {identifier:?} as of {as_of}")]
    Unmapped {
        /// Missing identifier.
        identifier: ExternalIdentifier,
        /// Time requested.
        as_of: OffsetDateTime,
    },
    /// Snapshot is older than requested timestamp.
    #[error("artifact stale: snapshot {snapshot} is before requested as_of {as_of}")]
    ArtifactStale {
        /// Snapshot time.
        snapshot: OffsetDateTime,
        /// Requested time.
        as_of: OffsetDateTime,
    },
    /// Backing storage or index failure.
    #[error("storage error: {0}")]
    Storage(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MicCode;
    use prismatik_determinism::{
        ArtifactId, ArtifactKind, ContentHash, DualSignature, SemanticVersion,
    };
    use time::macros::datetime;

    fn sample_artifact() -> ArtifactRef {
        ArtifactRef {
            artifact_id: ArtifactId::new("sym-1"),
            kind: ArtifactKind::SymbologySnapshot,
            version: SemanticVersion::new(1, 0, 0),
            content_hash: ContentHash::from_bytes(b"sym"),
            signature: DualSignature::default(),
        }
    }

    #[test]
    fn resolver_round_trips_bitemporal_binding() {
        let mut resolver = InMemoryResolver::new(sample_artifact());
        let aapl = AssetId::from_canonical_bytes(b"AAPL");
        let id = ExternalIdentifier::TickerAtVenue {
            ticker: "AAPL".into(),
            mic: MicCode::new(*b"XNAS"),
        };
        resolver.insert_valid(
            id.clone(),
            aapl,
            ValidityInterval::new(datetime!(2020-01-02 0:00 UTC), None).unwrap(),
        );

        // Before the binding is live -> None.
        assert_eq!(
            resolver
                .resolve_as_of(&id, datetime!(2019-01-01 0:00 UTC))
                .unwrap(),
            None
        );
        // On/after the lower bound -> resolves.
        assert_eq!(
            resolver
                .resolve_as_of(&id, datetime!(2020-06-01 0:00 UTC))
                .unwrap(),
            Some(aapl)
        );
        assert_eq!(
            resolver
                .identifiers_as_of(&aapl, datetime!(2020-06-01 0:00 UTC))
                .unwrap(),
            vec![id.clone()]
        );
        assert!(resolver.identity_chain(&aapl).unwrap().is_empty());
        assert_eq!(
            resolver.snapshot_artifact().artifact_id,
            ArtifactId::new("sym-1")
        );
    }

    #[test]
    fn transition_renames_identifier_at_effective_instant() {
        let mut resolver = InMemoryResolver::new(sample_artifact());
        let meta = AssetId::from_canonical_bytes(b"META");
        let fb = ExternalIdentifier::TickerAtVenue {
            ticker: "FB".into(),
            mic: MicCode::new(*b"XNAS"),
        };
        let new = ExternalIdentifier::TickerAtVenue {
            ticker: "META".into(),
            mic: MicCode::new(*b"XNAS"),
        };
        // FB live from 2020-01-02 until the rename.
        resolver.insert_valid(
            fb.clone(),
            meta,
            ValidityInterval::new(datetime!(2020-01-02 0:00 UTC), None).unwrap(),
        );
        resolver.insert_transition(IdentityTransition {
            asset_id: meta,
            effective_at: datetime!(2022-06-09 0:00 UTC),
            new_identifiers: vec![new.clone()],
            dropped_identifiers: vec![fb.clone()],
            memo: Some("renamed FB → META".into()),
        });

        // Before rename: FB resolves, META does not.
        assert_eq!(
            resolver
                .resolve_as_of(&fb, datetime!(2021-01-01 0:00 UTC))
                .unwrap(),
            Some(meta)
        );
        assert_eq!(
            resolver
                .resolve_as_of(&new, datetime!(2021-01-01 0:00 UTC))
                .unwrap(),
            None
        );
        // On/after rename: META resolves, FB does not.
        assert_eq!(
            resolver
                .resolve_as_of(&new, datetime!(2022-07-01 0:00 UTC))
                .unwrap(),
            Some(meta)
        );
        assert_eq!(
            resolver
                .resolve_as_of(&fb, datetime!(2022-07-01 0:00 UTC))
                .unwrap(),
            None
        );
        let chain = resolver.identity_chain(&meta).unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].memo.as_deref(), Some("renamed FB → META"));
    }

    #[test]
    fn validity_interval_rejects_inverted_bounds() {
        let err = ValidityInterval::new(
            datetime!(2022-01-02 0:00 UTC),
            Some(datetime!(2022-01-01 0:00 UTC)),
        )
        .expect_err("inverted interval must fail");
        assert!(matches!(err, SymbologyError::Storage(_)));
    }
}
