//! Generic append-only, hash-chained state transition journal.

use prismatik_determinism::ContentHash;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StatePayload<T> {
    journal_kind: String,
    sequence: u64,
    #[serde(with = "time::serde::rfc3339")]
    occurred_at: OffsetDateTime,
    event: String,
    state: T,
    previous_hash: ContentHash,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StateRecord<T> {
    payload: StatePayload<T>,
    record_hash: ContentHash,
}

/// Durable state journal failure.
#[derive(Debug, thiserror::Error)]
pub enum StateJournalError {
    /// File operation failed.
    #[error("state journal I/O at {path}: {source}")]
    Io {
        /// Affected path.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
    /// JSON encoding or decoding failed.
    #[error("invalid state journal JSON at {path}, line {line}: {source}")]
    Json {
        /// Affected path.
        path: PathBuf,
        /// One-based line number.
        line: usize,
        /// JSON error.
        source: serde_json::Error,
    },
    /// Chain metadata or digest failed verification.
    #[error("state journal integrity failure at {path}, line {line}: {reason}")]
    Integrity {
        /// Affected path.
        path: PathBuf,
        /// One-based line number.
        line: usize,
        /// Verification detail.
        reason: String,
    },
}

/// Fsynced JSONL state transitions whose complete chain is verified on open.
#[derive(Debug)]
pub struct FileStateJournal<T> {
    path: PathBuf,
    journal_kind: String,
    file: File,
    next_sequence: u64,
    tip_hash: ContentHash,
    latest: Option<T>,
}

impl<T> FileStateJournal<T>
where
    T: Clone + Serialize + DeserializeOwned,
{
    /// Open and replay a typed journal under a stable domain identifier.
    pub fn open(
        path: impl AsRef<Path>,
        journal_kind: impl Into<String>,
    ) -> Result<Self, StateJournalError> {
        let path = path.as_ref().to_path_buf();
        let journal_kind = journal_kind.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| StateJournalError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)
            .map_err(|source| StateJournalError::Io {
                path: path.clone(),
                source,
            })?;
        let mut tip_hash = genesis_hash(&journal_kind);
        let mut next_sequence = 0;
        let mut latest = None;
        for (index, line) in BufReader::new(&file).lines().enumerate() {
            let line_number = index + 1;
            let line = line.map_err(|source| StateJournalError::Io {
                path: path.clone(),
                source,
            })?;
            if line.trim().is_empty() {
                continue;
            }
            // Integrity is verified against the payload bytes exactly as they
            // were written, captured with `RawValue`. Re-serializing a decoded
            // `T` would instead make the digest a function of the *current*
            // struct definition, so adding any field to a journaled type would
            // invalidate every record ever written and the application would
            // refuse to start. Hashing the stored bytes detects tampering —
            // which is the actual goal — while leaving the type free to evolve.
            let raw: StateRecord<&serde_json::value::RawValue> = serde_json::from_str(&line)
                .map_err(|source| StateJournalError::Json {
                    path: path.clone(),
                    line: line_number,
                    source,
                })?;
            if raw.payload.journal_kind != journal_kind
                || raw.payload.sequence != next_sequence
                || raw.payload.previous_hash != tip_hash
            {
                return Err(integrity(&path, line_number, "chain metadata mismatch"));
            }
            let expected =
                hash_payload(&raw.payload).map_err(|source| StateJournalError::Json {
                    path: path.clone(),
                    line: line_number,
                    source,
                })?;
            if raw.record_hash != expected {
                return Err(integrity(&path, line_number, "record digest mismatch"));
            }
            // Decode the state only after the bytes have been proven intact.
            // Unknown fields are ignored and absent ones take serde defaults,
            // which is what lets an older record load into a newer type.
            let state: T = serde_json::from_str(raw.payload.state.get()).map_err(|source| {
                StateJournalError::Json {
                    path: path.clone(),
                    line: line_number,
                    source,
                }
            })?;
            tip_hash = raw.record_hash;
            next_sequence += 1;
            latest = Some(state);
        }
        Ok(Self {
            path,
            journal_kind,
            file,
            next_sequence,
            tip_hash,
            latest,
        })
    }

    /// Latest verified state.
    pub fn latest(&self) -> Option<&T> {
        self.latest.as_ref()
    }

    /// Number of verified transitions.
    pub const fn record_count(&self) -> u64 {
        self.next_sequence
    }

    /// Verified chain head.
    pub const fn tip_hash(&self) -> ContentHash {
        self.tip_hash
    }

    /// Append and fsync a complete state transition.
    pub fn append(
        &mut self,
        event: impl Into<String>,
        state: T,
        occurred_at: OffsetDateTime,
    ) -> Result<ContentHash, StateJournalError> {
        let payload = StatePayload {
            journal_kind: self.journal_kind.clone(),
            sequence: self.next_sequence,
            occurred_at,
            event: event.into(),
            state,
            previous_hash: self.tip_hash,
        };
        let line_number = self.next_sequence as usize + 1;
        let record_hash = hash_payload(&payload).map_err(|source| StateJournalError::Json {
            path: self.path.clone(),
            line: line_number,
            source,
        })?;
        let record = StateRecord {
            payload,
            record_hash,
        };
        serde_json::to_writer(&mut self.file, &record).map_err(|source| StateJournalError::Io {
            path: self.path.clone(),
            source: std::io::Error::other(source),
        })?;
        self.file
            .write_all(b"\n")
            .and_then(|_| self.file.flush())
            .and_then(|_| self.file.sync_data())
            .map_err(|source| StateJournalError::Io {
                path: self.path.clone(),
                source,
            })?;
        self.next_sequence += 1;
        self.tip_hash = record_hash;
        self.latest = Some(record.payload.state);
        Ok(record_hash)
    }
}

fn genesis_hash(journal_kind: &str) -> ContentHash {
    ContentHash::from_bytes(format!("prismatik:state-journal:v1:{journal_kind}").as_bytes())
}

fn hash_payload<T: Serialize>(payload: &StatePayload<T>) -> Result<ContentHash, serde_json::Error> {
    Ok(ContentHash::from_bytes(&serde_json::to_vec(payload)?))
}

fn integrity(path: &Path, line: usize, reason: &str) -> StateJournalError {
    StateJournalError::Integrity {
        path: path.to_path_buf(),
        line,
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_state_replays_and_detects_edits() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state.jsonl");
        let mut journal = FileStateJournal::<String>::open(&path, "test.v1").unwrap();
        journal
            .append("configured", "armed".into(), OffsetDateTime::UNIX_EPOCH)
            .unwrap();
        drop(journal);
        assert_eq!(
            FileStateJournal::<String>::open(&path, "test.v1")
                .unwrap()
                .latest(),
            Some(&"armed".to_owned())
        );
        let edited = std::fs::read_to_string(&path)
            .unwrap()
            .replace("armed", "stopped");
        std::fs::write(&path, edited).unwrap();
        assert!(matches!(
            FileStateJournal::<String>::open(&path, "test.v1"),
            Err(StateJournalError::Integrity { .. })
        ));
    }
}
