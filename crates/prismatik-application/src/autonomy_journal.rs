//! Durable append-only journal for monetary autonomy policy and usage.

use crate::AutonomyBudgetSnapshot;
use prismatik_determinism::ContentHash;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

/// Mutation recorded in the autonomy budget journal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyBudgetEvent {
    /// Initial default policy was established.
    Initialize,
    /// A user explicitly replaced the policy and reset usage.
    Configure,
    /// Spend or capital was reserved before an external action.
    Reserve,
    /// A definitive failed/rejected action released its reservation.
    Release,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct JournalPayload {
    sequence: u64,
    #[serde(with = "time::serde::rfc3339")]
    occurred_at: OffsetDateTime,
    event: AutonomyBudgetEvent,
    snapshot: AutonomyBudgetSnapshot,
    previous_hash: ContentHash,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct JournalRecord {
    payload: JournalPayload,
    record_hash: ContentHash,
}

/// Replay or append failure for the durable budget journal.
#[derive(Debug, thiserror::Error)]
pub enum AutonomyJournalError {
    /// File operation failed.
    #[error("autonomy journal I/O at {path}: {source}")]
    Io {
        /// Affected path.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
    /// A record was not valid JSON.
    #[error("invalid autonomy journal JSON at {path}, line {line}: {source}")]
    Decode {
        /// Affected path.
        path: PathBuf,
        /// One-based line number.
        line: usize,
        /// Underlying error.
        source: serde_json::Error,
    },
    /// Sequence, previous hash, or record digest did not verify.
    #[error("autonomy journal integrity failure at {path}, line {line}: {reason}")]
    Integrity {
        /// Affected path.
        path: PathBuf,
        /// One-based line number.
        line: usize,
        /// Verification detail.
        reason: String,
    },
}

/// JSONL journal that verifies its complete hash chain before exposing state.
#[derive(Debug)]
pub struct FileAutonomyJournal {
    path: PathBuf,
    file: File,
    next_sequence: u64,
    tip_hash: ContentHash,
    latest: Option<AutonomyBudgetSnapshot>,
}

impl FileAutonomyJournal {
    /// Open and replay an existing journal. Corruption fails startup closed.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AutonomyJournalError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| AutonomyJournalError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)
            .map_err(|source| AutonomyJournalError::Io {
                path: path.clone(),
                source,
            })?;
        let mut tip_hash = genesis_hash();
        let mut next_sequence = 0_u64;
        let mut latest = None;
        for (index, line) in BufReader::new(&file).lines().enumerate() {
            let line_number = index + 1;
            let line = line.map_err(|source| AutonomyJournalError::Io {
                path: path.clone(),
                source,
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let record: JournalRecord =
                serde_json::from_str(&line).map_err(|source| AutonomyJournalError::Decode {
                    path: path.clone(),
                    line: line_number,
                    source,
                })?;
            if record.payload.sequence != next_sequence {
                return Err(integrity(&path, line_number, "non-contiguous sequence"));
            }
            if record.payload.previous_hash != tip_hash {
                return Err(integrity(&path, line_number, "previous hash mismatch"));
            }
            let expected =
                hash_payload(&record.payload).map_err(|source| AutonomyJournalError::Decode {
                    path: path.clone(),
                    line: line_number,
                    source,
                })?;
            if record.record_hash != expected {
                return Err(integrity(&path, line_number, "record digest mismatch"));
            }
            tip_hash = record.record_hash;
            next_sequence += 1;
            latest = Some(record.payload.snapshot);
        }
        Ok(Self {
            path,
            file,
            next_sequence,
            tip_hash,
            latest,
        })
    }

    /// Latest verified state, if the journal has any entries.
    pub fn latest(&self) -> Option<&AutonomyBudgetSnapshot> {
        self.latest.as_ref()
    }

    /// Number of verified records in the journal.
    pub const fn record_count(&self) -> u64 {
        self.next_sequence
    }

    /// Current verified chain head.
    pub const fn tip_hash(&self) -> ContentHash {
        self.tip_hash
    }

    /// Append and fsync a new state before returning success.
    pub fn append(
        &mut self,
        event: AutonomyBudgetEvent,
        snapshot: AutonomyBudgetSnapshot,
        occurred_at: OffsetDateTime,
    ) -> Result<ContentHash, AutonomyJournalError> {
        let payload = JournalPayload {
            sequence: self.next_sequence,
            occurred_at,
            event,
            snapshot,
            previous_hash: self.tip_hash,
        };
        let record_hash =
            hash_payload(&payload).map_err(|source| AutonomyJournalError::Decode {
                path: self.path.clone(),
                line: self.next_sequence as usize + 1,
                source,
            })?;
        let record = JournalRecord {
            payload,
            record_hash,
        };
        serde_json::to_writer(&mut self.file, &record).map_err(|source| {
            AutonomyJournalError::Io {
                path: self.path.clone(),
                source: std::io::Error::other(source),
            }
        })?;
        self.file
            .write_all(b"\n")
            .and_then(|_| self.file.flush())
            .and_then(|_| self.file.sync_data())
            .map_err(|source| AutonomyJournalError::Io {
                path: self.path.clone(),
                source,
            })?;
        self.next_sequence += 1;
        self.tip_hash = record_hash;
        self.latest = Some(record.payload.snapshot);
        Ok(record_hash)
    }
}

fn genesis_hash() -> ContentHash {
    ContentHash::from_bytes(b"prismatik:autonomy-budget-journal:v1")
}

fn hash_payload(payload: &JournalPayload) -> Result<ContentHash, serde_json::Error> {
    Ok(ContentHash::from_bytes(&serde_json::to_vec(payload)?))
}

fn integrity(path: &Path, line: usize, reason: &str) -> AutonomyJournalError {
    AutonomyJournalError::Integrity {
        path: path.to_path_buf(),
        line,
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AutonomyBudgetPolicy, AutonomyPosture};

    fn snapshot(used: u64) -> AutonomyBudgetSnapshot {
        AutonomyBudgetSnapshot {
            policy: AutonomyBudgetPolicy {
                currency: "USD".into(),
                operations_limit_micros: 100,
                trading_limit_micros: 200,
                per_operation_limit_micros: 50,
                per_trade_limit_micros: 100,
            },
            operations_used_micros: used,
            trading_used_micros: 0,
            posture: AutonomyPosture::Active,
        }
    }

    #[test]
    fn exact_state_survives_restart() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("budget.jsonl");
        let mut journal = FileAutonomyJournal::open(&path).unwrap();
        journal
            .append(
                AutonomyBudgetEvent::Initialize,
                snapshot(0),
                OffsetDateTime::UNIX_EPOCH,
            )
            .unwrap();
        journal
            .append(
                AutonomyBudgetEvent::Reserve,
                snapshot(25),
                OffsetDateTime::UNIX_EPOCH,
            )
            .unwrap();
        drop(journal);
        let reopened = FileAutonomyJournal::open(&path).unwrap();
        assert_eq!(reopened.latest(), Some(&snapshot(25)));
    }

    #[test]
    fn modified_history_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("budget.jsonl");
        let mut journal = FileAutonomyJournal::open(&path).unwrap();
        journal
            .append(
                AutonomyBudgetEvent::Initialize,
                snapshot(0),
                OffsetDateTime::UNIX_EPOCH,
            )
            .unwrap();
        drop(journal);
        let text = std::fs::read_to_string(&path)
            .unwrap()
            .replace("\"USD\"", "\"EUR\"");
        std::fs::write(&path, text).unwrap();
        assert!(matches!(
            FileAutonomyJournal::open(&path),
            Err(AutonomyJournalError::Integrity { .. })
        ));
    }
}
