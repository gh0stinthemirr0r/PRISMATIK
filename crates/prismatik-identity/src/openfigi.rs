//! OpenFIGI mapping surface.
//!
//! Spec: `DOCS/spec/CRATE_ARCHITECTURE.md` §1.2.
//!
//! Wave 0 ships the type and an in-memory snapshot mapper. Live OpenFIGI
//! HTTP enrichment is a Wave 2 provider-adapter concern — this crate stays
//! free of `reqwest`.

use crate::resolver::{
    IdentityTransition, InMemoryResolver, SymbologyError, SymbologyResolver, ValidityInterval,
};
use crate::{AssetId, ExternalIdentifier, MicCode};
use prismatik_determinism::{
    ArtifactId, ArtifactKind, ArtifactRef, ContentHash, DetMap, DualSignature, SemanticVersion,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// A FIGI → [`AssetId`] mapping row loaded from a pinned symbology snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FigiMapping {
    /// OpenFIGI identifier.
    pub figi: String,
    /// Internal canonical id.
    pub asset_id: AssetId,
    /// Optional composite FIGI / share class FIGI when present.
    #[serde(default)]
    pub composite_figi: Option<String>,
}

/// Resolves OpenFIGI identifiers against a pinned in-memory table.
///
/// Cold-start loads mappings from a verified symbology artifact (Wave 0/2).
/// Hot-reload is an explicit reviewed event — this struct is not mutated
/// behind the caller's back.
#[derive(Debug)]
pub struct OpenFigiMapper {
    by_figi: DetMap<String, AssetId>,
    resolver: InMemoryResolver,
}

impl OpenFigiMapper {
    /// Empty mapper.
    pub fn new() -> Self {
        Self {
            by_figi: DetMap::default(),
            resolver: InMemoryResolver::new(artifact_for(b"[]")),
        }
    }

    /// Build from an explicit mapping table (tests and artifact loaders).
    pub fn from_mappings(rows: impl IntoIterator<Item = FigiMapping>) -> Self {
        let rows: Vec<_> = rows.into_iter().collect();
        let bytes = serde_json::to_vec(&rows).expect("FigiMapping serializes");
        let mut by_figi = DetMap::default();
        let mut resolver = InMemoryResolver::new(artifact_for(&bytes));
        for row in rows {
            resolver.insert(ExternalIdentifier::Figi(row.figi.clone()), row.asset_id);
            by_figi.insert(row.figi, row.asset_id);
        }
        Self { by_figi, resolver }
    }

    /// Load a pinned OpenFIGI cassette with bitemporal ticker aliases.
    pub fn from_json_cassette(json: &str) -> Result<Self, SymbologyError> {
        let cassette: Cassette = serde_json::from_str(json)
            .map_err(|error| SymbologyError::Storage(error.to_string()))?;
        let mut by_figi = DetMap::default();
        let mut resolver = InMemoryResolver::new(artifact_for(json.as_bytes()));

        for row in cassette.mappings {
            let asset = AssetId::from_canonical_bytes(row.canonical_key.as_bytes());
            by_figi.insert(row.figi.clone(), asset);
            resolver.insert(ExternalIdentifier::Figi(row.figi), asset);
            if let Some(composite) = row.composite_figi {
                resolver.insert(ExternalIdentifier::Figi(composite), asset);
            }
            for ticker in row.identifiers {
                if ticker.mic.len() != 4 {
                    return Err(SymbologyError::Storage(format!(
                        "MIC must have four characters: {}",
                        ticker.mic
                    )));
                }
                let identifier = ExternalIdentifier::TickerAtVenue {
                    ticker: ticker.ticker,
                    mic: MicCode::from_str_unchecked(&ticker.mic),
                };
                resolver.insert_valid(
                    identifier,
                    asset,
                    ValidityInterval::new(ticker.valid_from, ticker.valid_to)?,
                );
            }
        }

        for step in cassette.transitions {
            let asset = AssetId::from_canonical_bytes(step.canonical_key.as_bytes());
            resolver.insert_transition(IdentityTransition {
                asset_id: asset,
                effective_at: step.effective_at,
                new_identifiers: tickers_to_ids(&step.new_tickers)?,
                dropped_identifiers: tickers_to_ids(&step.dropped_tickers)?,
                memo: step.memo,
            });
        }

        Ok(Self { by_figi, resolver })
    }

    /// Load the embedded deterministic demo cassette.
    pub fn demo() -> Result<Self, SymbologyError> {
        Self::from_json_cassette(include_str!("../cassettes/openfigi/demo_mappings.json"))
    }

    /// Number of FIGI entries loaded.
    pub fn len(&self) -> usize {
        self.by_figi.len()
    }

    /// True when no mappings are loaded.
    pub fn is_empty(&self) -> bool {
        self.by_figi.is_empty()
    }

    /// Resolve a raw FIGI string.
    pub fn resolve_figi(&self, figi: &str) -> Option<AssetId> {
        self.by_figi.get(figi).copied()
    }

    /// Resolve an [`ExternalIdentifier::Figi`] variant; other kinds return `None`.
    pub fn resolve(&self, id: &ExternalIdentifier) -> Option<AssetId> {
        match id {
            ExternalIdentifier::Figi(figi) => self.resolve_figi(figi),
            _ => None,
        }
    }
}

impl Default for OpenFigiMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbologyResolver for OpenFigiMapper {
    fn resolve_as_of(
        &self,
        identifier: &ExternalIdentifier,
        as_of: OffsetDateTime,
    ) -> Result<Option<AssetId>, SymbologyError> {
        self.resolver.resolve_as_of(identifier, as_of)
    }

    fn identifiers_as_of(
        &self,
        asset: &AssetId,
        as_of: OffsetDateTime,
    ) -> Result<Vec<ExternalIdentifier>, SymbologyError> {
        self.resolver.identifiers_as_of(asset, as_of)
    }

    fn identity_chain(&self, asset: &AssetId) -> Result<Vec<IdentityTransition>, SymbologyError> {
        self.resolver.identity_chain(asset)
    }

    fn snapshot_artifact(&self) -> &ArtifactRef {
        self.resolver.snapshot_artifact()
    }
}

#[derive(Debug, Deserialize)]
struct Cassette {
    mappings: Vec<CassetteMapping>,
    #[serde(default)]
    transitions: Vec<CassetteTransition>,
}

#[derive(Debug, Deserialize)]
struct CassetteMapping {
    canonical_key: String,
    figi: String,
    composite_figi: Option<String>,
    identifiers: Vec<CassetteTicker>,
}

#[derive(Debug, Deserialize)]
struct CassetteTicker {
    ticker: String,
    mic: String,
    #[serde(with = "time::serde::rfc3339")]
    valid_from: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    valid_to: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
struct CassetteTransition {
    canonical_key: String,
    #[serde(with = "time::serde::rfc3339")]
    effective_at: OffsetDateTime,
    #[serde(default)]
    new_tickers: Vec<CassetteTickerRef>,
    #[serde(default)]
    dropped_tickers: Vec<CassetteTickerRef>,
    memo: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CassetteTickerRef {
    ticker: String,
    mic: String,
}

fn tickers_to_ids(rows: &[CassetteTickerRef]) -> Result<Vec<ExternalIdentifier>, SymbologyError> {
    rows.iter()
        .map(|row| {
            if row.mic.len() != 4 {
                return Err(SymbologyError::Storage(format!(
                    "MIC must have four characters: {}",
                    row.mic
                )));
            }
            Ok(ExternalIdentifier::TickerAtVenue {
                ticker: row.ticker.clone(),
                mic: MicCode::from_str_unchecked(&row.mic),
            })
        })
        .collect()
}

fn artifact_for(bytes: &[u8]) -> ArtifactRef {
    ArtifactRef {
        artifact_id: ArtifactId::new("openfigi-demo-mappings"),
        kind: ArtifactKind::SymbologySnapshot,
        version: SemanticVersion::new(1, 0, 0),
        content_hash: ContentHash::from_bytes(bytes),
        signature: DualSignature::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MicCode, SymbologyResolver};
    use time::macros::datetime;

    #[test]
    fn resolves_loaded_figi() {
        let aapl = AssetId::from_canonical_bytes(b"AAPL");
        let mapper = OpenFigiMapper::from_mappings([FigiMapping {
            figi: "BBG000B9XRY4".into(),
            asset_id: aapl,
            composite_figi: None,
        }]);
        assert_eq!(mapper.resolve_figi("BBG000B9XRY4"), Some(aapl));
        assert_eq!(
            mapper.resolve(&ExternalIdentifier::Figi("BBG000B9XRY4".into())),
            Some(aapl)
        );
        assert!(mapper
            .resolve(&ExternalIdentifier::CoinGeckoId("bitcoin".into()))
            .is_none());
    }

    #[test]
    fn demo_cassette_loads_required_equities() {
        let mapper = OpenFigiMapper::from_json_cassette(include_str!(
            "../cassettes/openfigi/demo_mappings.json"
        ))
        .unwrap();
        assert_eq!(mapper.len(), 4);
        for figi in [
            "BBG000B9XRY4",
            "BBG000MM2P62",
            "BBG009S39JX6",
            "BBG000BLNNH6",
        ] {
            assert!(mapper.resolve_figi(figi).is_some(), "missing {figi}");
        }
    }

    #[test]
    fn fb_rename_preserves_meta_identity_and_figi() {
        let mapper = OpenFigiMapper::demo().unwrap();
        let fb = ExternalIdentifier::TickerAtVenue {
            ticker: "FB".into(),
            mic: MicCode::new(*b"XNAS"),
        };
        let meta = ExternalIdentifier::TickerAtVenue {
            ticker: "META".into(),
            mic: MicCode::new(*b"XNAS"),
        };
        let figi = ExternalIdentifier::Figi("BBG000MM2P62".into());

        let old = mapper
            .resolve_as_of(&fb, datetime!(2020-01-02 0:00 UTC))
            .unwrap();
        let current = mapper
            .resolve_as_of(&meta, datetime!(2026-01-02 0:00 UTC))
            .unwrap();
        let by_figi = mapper
            .resolve_as_of(&figi, datetime!(2026-01-02 0:00 UTC))
            .unwrap();

        assert_eq!(old, current);
        assert_eq!(current, by_figi);
        assert_eq!(
            mapper
                .resolve_as_of(&fb, datetime!(2026-01-02 0:00 UTC))
                .unwrap(),
            None
        );
        let chain = mapper.identity_chain(&current.unwrap()).unwrap();
        assert!(chain
            .iter()
            .any(|step| step.memo.as_deref() == Some("renamed FB → META")));
    }

    #[test]
    fn split_does_not_change_asset_identity() {
        let mapper = OpenFigiMapper::demo().unwrap();
        let aapl = ExternalIdentifier::TickerAtVenue {
            ticker: "AAPL".into(),
            mic: MicCode::new(*b"XNAS"),
        };
        let before = mapper
            .resolve_as_of(&aapl, datetime!(2020-08-28 0:00 UTC))
            .unwrap();
        let after = mapper
            .resolve_as_of(&aapl, datetime!(2020-08-31 0:00 UTC))
            .unwrap();
        assert_eq!(before, after);
    }
}
