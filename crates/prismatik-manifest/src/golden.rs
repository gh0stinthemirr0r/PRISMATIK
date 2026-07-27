//! Golden manifest corpus harness (P0-QM-02).

use crate::types::{
    AuditBlock, DeterminismBlock, EntropyStreamSeed, ManifestBuilder, ManifestKind, ProducerInfo,
    ProducerProfile,
};
use crate::verify::sign_manifest_ed25519;
use prismatik_determinism::{
    signing_key_from_seed, ContentHash, PinnedArtifactSet, RunId, SigningIdentity,
    SigningIdentityKind,
};
use time::OffsetDateTime;

/// Embedded golden manifest JSON (data ingest).
pub const GOLDEN_DATA_INGEST: &str = include_str!("../golden/manifest_data_ingest_v1.json");

/// Embedded golden manifest JSON (DST replay).
pub const GOLDEN_DST_REPLAY: &str = include_str!("../golden/manifest_dst_replay_v1.json");

/// Build the two golden manifests used by the corpus (deterministic keys).
pub fn build_golden_pair(produced_at: OffsetDateTime) -> (String, String) {
    let key = signing_key_from_seed([42u8; 32]);
    let identity = SigningIdentity {
        kind: SigningIdentityKind::Ci,
        id: "prismatik-golden-corpus".into(),
        signed_at: Some(produced_at.to_string()),
    };

    let mut ingest = ManifestBuilder::new(
        "golden-data-ingest-001",
        RunId::from_bytes([1u8; 16]),
        ManifestKind::DataIngest,
        produced_at,
        ProducerInfo {
            prismatik_version: "0.1.0".into(),
            build_hash: "golden".into(),
            build_attestation: None,
            profile: ProducerProfile::Desktop,
        },
        DeterminismBlock {
            root_seed: 8675309,
            clock_kind: "frozen".into(),
            clock_start: Some(produced_at),
            clock_end: None,
            clock_tick_step_micros: None,
            entropy_streams: vec![EntropyStreamSeed {
                label: "ingest.pipeline".into(),
                seed: 12345,
            }],
            thread_count: 1,
            rayon_parallel: false,
            libc_overrides: vec![],
            trace_digest: ContentHash::from_bytes(b"ingest-trace"),
        },
    )
    .pinned(PinnedArtifactSet::default())
    .lineage("data_ingest.trivial", "1.0.0")
    .audit(AuditBlock {
        tree_size: 0,
        tree_root: ContentHash::from([0u8; 32]),
        leaf_position: None,
        leaf_hash: None,
    })
    .build_unsigned();
    sign_manifest_ed25519(&mut ingest, &key, identity.clone()).unwrap();

    let mut dst = ManifestBuilder::new(
        "golden-dst-replay-001",
        RunId::from_bytes([2u8; 16]),
        ManifestKind::DstReplay,
        produced_at,
        ProducerInfo {
            prismatik_version: "0.1.0".into(),
            build_hash: "golden".into(),
            build_attestation: None,
            profile: ProducerProfile::Desktop,
        },
        DeterminismBlock {
            root_seed: 42,
            clock_kind: "simulated".into(),
            clock_start: Some(OffsetDateTime::UNIX_EPOCH),
            clock_end: Some(OffsetDateTime::UNIX_EPOCH),
            clock_tick_step_micros: Some(1_000_000),
            entropy_streams: vec![EntropyStreamSeed {
                label: "dst.pipeline".into(),
                seed: 42,
            }],
            thread_count: 1,
            rayon_parallel: false,
            libc_overrides: vec!["madsim::rand".into(), "madsim::time".into()],
            trace_digest: ContentHash::from_bytes(b"dst-trace"),
        },
    )
    .lineage("dst_replay_suite.trivial", "1.0.0")
    .build_unsigned();
    sign_manifest_ed25519(&mut dst, &key, identity).unwrap();

    (
        serde_json::to_string_pretty(&ingest).unwrap(),
        serde_json::to_string_pretty(&dst).unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::unsigned_canonical_digest;
    use crate::types::SCHEMA_VERSION;
    use crate::verify::StandaloneVerifier;
    use time::macros::datetime;

    #[test]
    fn committed_goldens_verify_and_match_schema() {
        let verifier = StandaloneVerifier::transitional();
        let at = datetime!(2026-07-26 14:32:00 UTC);
        for json in [GOLDEN_DATA_INGEST, GOLDEN_DST_REPLAY] {
            let report = verifier.verify_json(json, None, None, at).unwrap();
            assert!(report.overall, "{report:?}");
            assert!(report.schema_version_ok);
            let m: crate::types::ManifestV1 = serde_json::from_str(json).unwrap();
            assert_eq!(m.schema_version, SCHEMA_VERSION);
            let digest = unsigned_canonical_digest(&m).unwrap();
            assert_ne!(digest, ContentHash::from([0u8; 32]));
        }
    }

    #[test]
    fn regenerated_goldens_match_embedded_digests() {
        let at = datetime!(2026-07-26 14:32:00 UTC);
        let (ingest, dst) = build_golden_pair(at);
        let v = StandaloneVerifier::transitional();
        assert!(v.verify_json(&ingest, None, None, at).unwrap().overall);
        assert!(v.verify_json(&dst, None, None, at).unwrap().overall);
        assert!(
            v.verify_json(GOLDEN_DATA_INGEST, None, None, at)
                .unwrap()
                .overall
        );
        assert!(
            v.verify_json(GOLDEN_DST_REPLAY, None, None, at)
                .unwrap()
                .overall
        );
    }
}
