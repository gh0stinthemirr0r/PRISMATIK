//! Public manifest verification service (`P9-QM-01` floor).
//!
//! Offline third-party verification: no HTTP server; callers supply a manifest
//! path and optional published [`PublicTreeHead`].

use crate::CliError;
use prismatik_determinism::{Clock, ContentHash, SystemClock};
use prismatik_manifest::{ManifestV1, StandaloneVerifier, VerificationReport};
use serde::{Deserialize, Serialize};
use std::fs;

/// Publicly published audit tree head (offline verification input).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicTreeHead {
    /// Merkle root of the audit log at publication.
    pub root_hash: ContentHash,
    /// Number of entries in the tree (`tree_size`).
    pub entry_count: u64,
    /// Publication instant as Unix epoch microseconds.
    pub published_at_micros: u64,
}

/// Combined manifest + optional tree-head verification report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyReport {
    /// Standalone manifest verification details.
    #[serde(flatten)]
    pub manifest: VerificationReport,
    /// `Some(true|false)` when a public tree head was supplied; `None` otherwise.
    pub tree_head_ok: Option<bool>,
    /// Overall pass (manifest checks and optional tree head).
    pub overall: bool,
}

/// Offline public manifest verification service.
#[derive(Debug, Default)]
pub struct PublicVerifyService;

impl PublicVerifyService {
    /// Verify a manifest JSON file against optional published tree head.
    pub fn verify_manifest(
        path: &str,
        tree_head: Option<&PublicTreeHead>,
    ) -> Result<VerifyReport, CliError> {
        let json = fs::read_to_string(path)?;
        Self::verify_manifest_json(&json, tree_head)
    }

    /// Verify manifest JSON (used by tests and file-based entry point).
    pub fn verify_manifest_json(
        json: &str,
        tree_head: Option<&PublicTreeHead>,
    ) -> Result<VerifyReport, CliError> {
        let manifest: ManifestV1 =
            serde_json::from_str(json).map_err(|e| CliError::Verify(format!("parse: {e}")))?;

        let tree_head_ok = tree_head.map(|head| tree_head_matches(&manifest, head));

        let verifier = StandaloneVerifier {
            transitional: false,
        };
        // CLI is the outermost shell; real wall-clock time is the correct
        // source for "when did the operator run this verification".
        let verified_at = SystemClock::new().now();
        let manifest_report = verifier
            .verify_json(json, None, None, verified_at)
            .map_err(|e| CliError::Verify(e.to_string()))?;

        let overall = manifest_report.overall && tree_head_ok.unwrap_or(true);

        Ok(VerifyReport {
            manifest: manifest_report,
            tree_head_ok,
            overall,
        })
    }
}

fn tree_head_matches(manifest: &ManifestV1, head: &PublicTreeHead) -> bool {
    manifest.audit.tree_root == head.root_hash && manifest.audit.tree_size == head.entry_count
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_manifest::GOLDEN_DATA_INGEST;
    use std::io::Write;
    use time::macros::datetime;

    fn write_temp_manifest(contents: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        write!(file, "{contents}").expect("write manifest");
        file
    }

    fn golden_tree_head() -> PublicTreeHead {
        PublicTreeHead {
            root_hash: ContentHash::from([0u8; 32]),
            entry_count: 0,
            published_at_micros: 1_784_000_000_000_000,
        }
    }

    #[test]
    fn valid_manifest_with_matching_tree_head_passes() {
        let file = write_temp_manifest(GOLDEN_DATA_INGEST);
        let report = PublicVerifyService::verify_manifest(
            file.path().to_str().unwrap(),
            Some(&golden_tree_head()),
        )
        .expect("verify");
        assert!(report.manifest.overall, "{report:?}");
        assert_eq!(report.tree_head_ok, Some(true));
        assert!(report.overall);
    }

    #[test]
    fn valid_manifest_without_tree_head_passes() {
        let file = write_temp_manifest(GOLDEN_DATA_INGEST);
        let report = PublicVerifyService::verify_manifest(file.path().to_str().unwrap(), None)
            .expect("verify");
        assert!(report.overall);
        assert!(report.tree_head_ok.is_none());
    }

    #[test]
    fn tampered_manifest_fails() {
        let mut tampered = GOLDEN_DATA_INGEST.to_string();
        tampered = tampered.replace("golden-data-ingest-001", "golden-data-ingest-TAMPERED");
        let file = write_temp_manifest(&tampered);
        let report = PublicVerifyService::verify_manifest(
            file.path().to_str().unwrap(),
            Some(&golden_tree_head()),
        )
        .expect("verify returns report even on failure");
        assert!(!report.manifest.overall, "signature must fail");
        assert!(!report.overall);
    }

    #[test]
    fn tree_head_mismatch_fails() {
        let file = write_temp_manifest(GOLDEN_DATA_INGEST);
        let bad_head = PublicTreeHead {
            root_hash: ContentHash::from([1u8; 32]),
            entry_count: 99,
            published_at_micros: 1,
        };
        let report =
            PublicVerifyService::verify_manifest(file.path().to_str().unwrap(), Some(&bad_head))
                .expect("verify");
        assert!(report.manifest.overall, "manifest itself is valid");
        assert_eq!(report.tree_head_ok, Some(false));
        assert!(!report.overall);
    }

    #[test]
    fn tree_head_entry_count_mismatch_fails() {
        let file = write_temp_manifest(GOLDEN_DATA_INGEST);
        let mut head = golden_tree_head();
        head.entry_count = 42;
        let report =
            PublicVerifyService::verify_manifest(file.path().to_str().unwrap(), Some(&head))
                .expect("verify");
        assert_eq!(report.tree_head_ok, Some(false));
        assert!(!report.overall);
    }

    #[test]
    fn public_tree_head_round_trips_json() {
        let head = golden_tree_head();
        let json = serde_json::to_string(&head).unwrap();
        let back: PublicTreeHead = serde_json::from_str(&json).unwrap();
        assert_eq!(head, back);
    }

    #[test]
    fn verify_report_includes_verified_at() {
        let file = write_temp_manifest(GOLDEN_DATA_INGEST);
        let at = datetime!(2026-07-26 14:32:00 UTC);
        let json = fs::read_to_string(file.path()).unwrap();
        let verifier = StandaloneVerifier::transitional();
        let manifest = verifier.verify_json(&json, None, None, at).unwrap();
        assert_eq!(manifest.verified_at, at);
        let report =
            PublicVerifyService::verify_manifest(file.path().to_str().unwrap(), None).unwrap();
        assert!(report.manifest.verified_at.unix_timestamp() > 0);
    }
}
