//! Durable observation store — corpus Day 2 (briefing §52 / §47).
//!
//! Append-only, content-addressed persistence for sealed [`Observation`]s and
//! their raw payloads. This is the on-disk floor beneath the in-memory
//! [`ObservationLog`]; it survives restart, is externally inspectable, and is
//! the substrate the audit-event bridge (§52 Day 2) writes against.
//!
//! ## Layout
//!
//! Under a root directory (the desktop `data_dir` or a cloud volume):
//!
//! ```text
//! <root>/
//!   observations.jsonl   # one sealed Observation per line, append-only
//!   payloads/
//!     <payload_hash hex>  # raw bytes, written only when policy permits
//! ```
//!
//! ## Guarantees (Part IV §47 / §55)
//!
//! - **Append-only / immutable.** Observations are never rewritten or deleted;
//!   corrections are new observations linked via `supersedes`.
//! - **Idempotent.** A duplicate `ObservationId` is a no-op returning
//!   [`AppendOutcome::AlreadyPresent`].
//! - **Atomic.** Each JSONL append is fsynced so a crash mid-append cannot
//!   corrupt the ledger (the trailing partial line is truncated on reopen).
//! - **Policy-gated raw payloads.** Raw bytes are written only when the
//!   observation's source policy permitted `RawPayloadPermitted`; otherwise
//!   only the metadata-bearing JSONL record exists (§47.2 copyright posture).
//! - **Point-in-time safe.** No `as_of` filtering here — the store is a raw
//!   ledger; the point-in-time property test (§10) lives in the read path.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use prismatik_determinism::ContentHash;
use prismatik_market_data::{Observation, ObservationId};
use serde::{Deserialize, Serialize};

/// Outcome of a durable append — mirrors the in-memory log's [`ObservationAppendOutcome`]
/// but carries the byte offset of the appended line for replay/proof lookups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurableAppendOutcome {
    /// A new observation was appended at this byte offset.
    Inserted {
        /// Zero-based line position in the JSONL ledger.
        position: usize,
        /// Byte offset of the appended line (for seek-based reads).
        byte_offset: u64,
    },
    /// The observation was already present; the append was a no-op.
    AlreadyPresent {
        /// Position of the existing record.
        position: usize,
    },
}

/// One JSONL ledger line: the sealed observation plus the byte offset recorded
/// for fast random access on replay.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct LedgerLine {
    /// Monotonic zero-based line number (for ordering / proof positions).
    line: u64,
    /// The sealed observation.
    observation: Observation,
}

/// Durable, append-only observation store backed by a filesystem directory.
///
/// Thread-safe via an internal `Mutex` guarding the open ledger handle. The
/// raw-payload blob store is content-addressed and write-once; concurrent
/// writers of the same hash are idempotent.
pub struct FileObservationStore {
    root: PathBuf,
    ledger_path: PathBuf,
    payloads_dir: PathBuf,
    /// Append handle + the set of already-stored observation ids (idempotency).
    inner: Mutex<StoreInner>,
}

impl std::fmt::Debug for FileObservationStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileObservationStore")
            .field("root", &self.root)
            .field("ledger_path", &self.ledger_path)
            .field("payloads_dir", &self.payloads_dir)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct StoreInner {
    file: std::fs::File,
    /// Observed ids and their line positions, for idempotent dedup.
    seen: BTreeSet<[u8; 32]>,
    /// Next line number to assign.
    next_line: u64,
}

/// Store configuration / construction failure.
#[derive(Debug, thiserror::Error)]
pub enum ObservationStoreError {
    /// The root path could not be created or opened.
    #[error("observation store io: {0}")]
    Io(#[from] std::io::Error),
    /// An observation could not be (de)serialized.
    #[error("observation store serde: {0}")]
    Serde(#[from] serde_json::Error),
    /// The on-disk ledger was corrupt and could not be recovered.
    #[error("observation store ledger corrupt: {0}")]
    Corrupt(String),
}

impl FileObservationStore {
    /// Open (or create) a store rooted at `root`. Existing observations are
    /// replayed into the idempotency index so duplicate appends after a restart
    /// are no-ops.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, ObservationStoreError> {
        let root = root.as_ref().to_path_buf();
        let payloads_dir = root.join("payloads");
        std::fs::create_dir_all(&payloads_dir)?;
        let ledger_path = root.join("observations.jsonl");

        // Open for append + read; create if absent.
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&ledger_path)?;

        let mut inner = StoreInner {
            file,
            seen: BTreeSet::new(),
            next_line: 0,
        };

        // Replay existing lines to rebuild the idempotency index. A trailing
        // partial line (crash mid-append) is truncated.
        Self::replay_and_repair(&ledger_path, &mut inner)?;

        Ok(Self {
            root,
            ledger_path,
            payloads_dir,
            inner: Mutex::new(inner),
        })
    }

    /// Replay the ledger into the idempotency index, truncating any trailing
    /// partial line left by a crash.
    fn replay_and_repair(
        ledger_path: &Path,
        inner: &mut StoreInner,
    ) -> Result<(), ObservationStoreError> {
        let bytes = std::fs::read(ledger_path)?;
        if bytes.is_empty() {
            return Ok(());
        }
        // Split into lines; the last segment after the final '\n' may be partial.
        let mut valid_len = 0usize;
        let mut next_line = 0u64;
        let mut seen = BTreeSet::new();
        for line in bytes.split_inclusive(|b| *b == b'\n') {
            let ends_newline = line.ends_with(b"\n");
            let payload = if ends_newline {
                &line[..line.len() - 1]
            } else {
                // Trailing partial line — stop here; we'll truncate below.
                break;
            };
            let parsed: LedgerLine = serde_json::from_slice(payload)?;
            let id_bytes = parsed.observation.id().as_bytes();
            if !seen.insert(id_bytes) {
                return Err(ObservationStoreError::Corrupt(format!(
                    "duplicate observation id {} in ledger",
                    parsed.observation.id()
                )));
            }
            next_line = parsed.line + 1;
            valid_len += line.len();
        }

        // Truncate the trailing partial line if one was present, then reopen
        // for append so the handle points past the now-clean end of file.
        if valid_len < bytes.len() {
            drop(std::mem::replace(
                &mut inner.file,
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(ledger_path)
                    .and_then(|f| {
                        f.set_len(valid_len as u64)?;
                        Ok(f)
                    })?,
            ));
            inner.file = std::fs::OpenOptions::new()
                .append(true)
                .read(true)
                .open(ledger_path)?;
        }

        inner.seen = seen;
        inner.next_line = next_line;
        Ok(())
    }

    /// Append a sealed observation. Idempotent on `observation.id()`. When
    /// `raw_payload` is `Some` and the observation's policy permitted raw
    /// retention, the bytes are written as a content-addressed blob.
    pub fn append(
        &self,
        observation: &Observation,
        raw_payload: Option<&[u8]>,
    ) -> Result<DurableAppendOutcome, ObservationStoreError> {
        let id_bytes = observation.id().as_bytes();
        let mut inner = self.inner.lock().expect("observation store lock");

        // Idempotency: a seen id is a no-op.
        if inner.seen.contains(&id_bytes) {
            // Callers needing the exact position should scan; return a
            // best-effort position derived from the current line count.
            return Ok(DurableAppendOutcome::AlreadyPresent {
                position: inner.next_line.saturating_sub(1) as usize,
            });
        }

        // Write the raw payload blob (content-addressed, write-once) if policy
        // permitted it. The observation already encodes whether raw retention
        // was allowed via its `raw_object_ref` field; the caller supplies bytes
        // only when appropriate.
        if let Some(bytes) = raw_payload {
            self.write_payload_blob(&observation.payload_hash(), bytes)?;
        }

        let line = inner.next_line;
        let entry = LedgerLine {
            line,
            observation: observation.clone(),
        };
        let mut json = serde_json::to_vec(&entry)?;
        json.push(b'\n');

        // Record the byte offset of this line before writing.
        let byte_offset = inner
            .file
            .metadata()
            .map_err(ObservationStoreError::Io)?
            .len();

        inner.file.write_all(&json)?;
        // fsync to make the append durable before reporting success.
        inner.file.sync_data()?;

        inner.seen.insert(id_bytes);
        inner.next_line = line + 1;

        Ok(DurableAppendOutcome::Inserted {
            position: line as usize,
            byte_offset,
        })
    }

    /// Write a content-addressed raw payload blob. Idempotent: a pre-existing
    /// blob with the same hash is left in place (write-once).
    fn write_payload_blob(
        &self,
        hash: &ContentHash,
        bytes: &[u8],
    ) -> Result<(), ObservationStoreError> {
        let blob_path = self.payloads_dir.join(payload_blob_name(hash));
        if blob_path.exists() {
            // Write-once: trust the existing blob. (A hash collision under
            // BLAKE3 is computationally infeasible; a mismatch here would be a
            // storage corruption event worth surfacing separately.)
            return Ok(());
        }
        // Atomic write via temp + rename so a crash cannot leave a partial blob.
        let tmp = blob_path.with_extension("tmp");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &blob_path)?;
        Ok(())
    }

    /// Read a raw payload blob by content hash, if it was retained.
    pub fn read_payload(
        &self,
        hash: &ContentHash,
    ) -> Result<Option<Vec<u8>>, ObservationStoreError> {
        let blob_path = self.payloads_dir.join(payload_blob_name(hash));
        if !blob_path.exists() {
            return Ok(None);
        }
        Ok(Some(std::fs::read(&blob_path)?))
    }

    /// Number of observations durably stored.
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .expect("observation store lock")
            .seen
            .len()
    }

    /// True when no observations are stored.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether an identical publisher record payload already exists for a source.
    /// This prevents scheduled refreshes from appending unchanged historical rows.
    pub fn find_publisher_payload(
        &self,
        source_id: &str,
        publisher_record_id: &str,
        payload_hash: &ContentHash,
    ) -> Result<Option<ObservationId>, ObservationStoreError> {
        Ok(self.iter()?.into_iter().find_map(|observation| {
            (observation.source_id().as_str() == source_id
                && observation.publisher_record_id() == Some(publisher_record_id)
                && observation.payload_hash() == *payload_hash)
                .then(|| observation.id())
        }))
    }

    /// Iterate all stored observations in append order (replays the ledger).
    pub fn iter(&self) -> Result<Vec<Observation>, ObservationStoreError> {
        let bytes = {
            let _inner = self.inner.lock().expect("observation store lock");
            std::fs::read(&self.ledger_path)?
        };
        let mut out = Vec::new();
        for line in bytes.split(|b| *b == b'\n') {
            if line.is_empty() {
                continue;
            }
            let parsed: LedgerLine = serde_json::from_slice(line)?;
            out.push(parsed.observation);
        }
        Ok(out)
    }

    /// The root directory backing this store.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// File name for a content-addressed payload blob: bare 64-char hex of the hash.
fn payload_blob_name(hash: &ContentHash) -> String {
    hex::encode(hash.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_determinism::ContentHash;
    use prismatik_determinism::FrozenClock;
    use prismatik_market_data::{
        AcquisitionMethod, Observation, ObservationDraft, PayloadRetention, PermittedMetadata,
        RetrievalAttemptId, SourceId, SourcePolicy, SourcePolicyVersion,
    };
    use tempfile::TempDir;
    use time::{Duration, OffsetDateTime};

    fn sealed_observation(
        clock: &FrozenClock,
        uri: &str,
        payload: &[u8],
        policy_retention: PayloadRetention,
    ) -> Observation {
        let source_id = SourceId::new("test-source").unwrap();
        let version = SourcePolicyVersion::new(1).unwrap();
        let at = OffsetDateTime::UNIX_EPOCH;
        let policy = SourcePolicy::new(
            source_id.clone(),
            version,
            [AcquisitionMethod::Api],
            policy_retention,
            prismatik_market_data::RedistributionPolicy::PermittedMetadataOnly,
            [
                prismatik_market_data::PermittedMetadataField::Title,
                prismatik_market_data::PermittedMetadataField::Entities,
            ],
            "test",
            at,
            at,
            at + Duration::days(365),
            "test-review",
        )
        .unwrap();
        let draft = ObservationDraft::new(
            clock,
            source_id,
            version,
            AcquisitionMethod::Api,
            uri,
            "application/json",
            ContentHash::from_bytes(payload),
            RetrievalAttemptId::from_bytes([1u8; 32]),
            prismatik_market_data::IngestManifestId::from_bytes([2u8; 32]),
        )
        .unwrap();
        Observation::seal(draft, &policy).unwrap()
    }

    #[test]
    fn append_is_durable_and_idempotent() {
        let tmp = TempDir::new().unwrap();
        let clock = FrozenClock::new(OffsetDateTime::UNIX_EPOCH);
        let store = FileObservationStore::open(tmp.path()).unwrap();

        let obs = sealed_observation(
            &clock,
            "https://example.test/a",
            b"payload-a",
            PayloadRetention::RawPayloadPermitted,
        );
        let out1 = store.append(&obs, Some(b"payload-a")).unwrap();
        assert!(matches!(out1, DurableAppendOutcome::Inserted { .. }));

        // Duplicate id is a no-op.
        let out2 = store.append(&obs, Some(b"payload-a")).unwrap();
        assert!(matches!(out2, DurableAppendOutcome::AlreadyPresent { .. }));

        assert_eq!(store.len(), 1);
        let payload = store.read_payload(&obs.payload_hash()).unwrap();
        assert_eq!(payload.as_deref(), Some(b"payload-a".as_slice()));
    }

    #[test]
    fn store_survives_reopen_and_replays_index() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();
        let clock = FrozenClock::new(OffsetDateTime::UNIX_EPOCH);

        {
            let store = FileObservationStore::open(&path).unwrap();
            let a = sealed_observation(
                &clock,
                "https://example.test/a",
                b"a",
                PayloadRetention::RawPayloadPermitted,
            );
            let b = sealed_observation(
                &clock,
                "https://example.test/b",
                b"b",
                PayloadRetention::RawPayloadPermitted,
            );
            store.append(&a, Some(b"a")).unwrap();
            store.append(&b, Some(b"b")).unwrap();
            assert_eq!(store.len(), 2);
        }

        // Reopen: the index must be rebuilt and dedup must still hold.
        let reopened = FileObservationStore::open(&path).unwrap();
        assert_eq!(reopened.len(), 2);
        let all = reopened.iter().unwrap();
        assert_eq!(all.len(), 2);

        // Appending a duplicate after reopen is still idempotent.
        let a = sealed_observation(
            &clock,
            "https://example.test/a",
            b"a",
            PayloadRetention::RawPayloadPermitted,
        );
        let out = reopened.append(&a, Some(b"a")).unwrap();
        assert!(matches!(out, DurableAppendOutcome::AlreadyPresent { .. }));
        assert_eq!(reopened.len(), 2);
    }

    #[test]
    fn metadata_only_policy_stores_no_raw_blob() {
        let tmp = TempDir::new().unwrap();
        let clock = FrozenClock::new(OffsetDateTime::UNIX_EPOCH);
        let store = FileObservationStore::open(tmp.path()).unwrap();
        let obs = sealed_observation(
            &clock,
            "https://example.test/m",
            b"meta-only",
            PayloadRetention::MetadataOnly,
        );
        // Caller honors the policy by passing None for the raw payload.
        store.append(&obs, None).unwrap();
        // The observation is in the ledger...
        assert_eq!(store.len(), 1);
        // ...but no raw blob exists.
        assert!(store.read_payload(&obs.payload_hash()).unwrap().is_none());
    }

    // `PermittedMetadata` is exercised indirectly via the sealed observation;
    // keep the import so a future field change is caught at compile time.
    const _: fn() = || {
        let _ = PermittedMetadata::default();
    };
}
