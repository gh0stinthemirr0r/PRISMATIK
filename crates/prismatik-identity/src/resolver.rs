//! Symbology resolution interfaces.

use crate::{AssetId, ExternalIdentifier, VenueId};
use prismatik_determinism::ArtifactRef;
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

/// A single identity transition in the asset history chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityTransition {
    /// Source asset id.
    pub from: AssetId,
    /// Target asset id.
    pub to: AssetId,
    /// Transition event time.
    pub occurred_at: OffsetDateTime,
    /// Human-readable reason.
    pub reason: String,
}

/// Bitemporal symbology resolver contract.
pub trait SymbologyResolver: Send + Sync {
    /// Resolve an external identifier as of a specific point in time.
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

    /// Return full identity transition chain for an asset.
    fn identity_chain(&self, asset: &AssetId) -> Result<Vec<IdentityTransition>, SymbologyError>;

    /// Return the pinned snapshot artifact this resolver was loaded from.
    fn snapshot_artifact(&self) -> &ArtifactRef;
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

    #[derive(Debug)]
    struct DummyResolver {
        artifact: ArtifactRef,
    }

    impl SymbologyResolver for DummyResolver {
        fn resolve_as_of(
            &self,
            _identifier: &ExternalIdentifier,
            _as_of: OffsetDateTime,
        ) -> Result<Option<AssetId>, SymbologyError> {
            Ok(None)
        }

        fn identifiers_as_of(
            &self,
            _asset: &AssetId,
            _as_of: OffsetDateTime,
        ) -> Result<Vec<ExternalIdentifier>, SymbologyError> {
            Ok(vec![ExternalIdentifier::TickerAtVenue {
                ticker: "AAPL".into(),
                mic: MicCode::parse("XNAS").expect("valid mic"),
            }])
        }

        fn identity_chain(
            &self,
            _asset: &AssetId,
        ) -> Result<Vec<IdentityTransition>, SymbologyError> {
            Ok(Vec::new())
        }

        fn snapshot_artifact(&self) -> &ArtifactRef {
            &self.artifact
        }
    }

    #[test]
    fn resolver_contract_is_callable() {
        let resolver = DummyResolver {
            artifact: ArtifactRef {
                artifact_id: ArtifactId::new("sym-1"),
                kind: ArtifactKind::SymbologySnapshot,
                version: SemanticVersion::new(1, 0, 0),
                content_hash: ContentHash::from_bytes(b"sym"),
                signature: DualSignature::default(),
            },
        };

        let ids = resolver
            .identifiers_as_of(&AssetId([0; 32]), OffsetDateTime::UNIX_EPOCH)
            .expect("lookup should not fail");
        assert_eq!(ids.len(), 1);
        assert_eq!(
            resolver.snapshot_artifact().artifact_id,
            ArtifactId::new("sym-1")
        );
    }
}
