//! Append-only registry connecting real evidence observations to intelligence surfaces.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

/// Native intelligence observation class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntelligenceObservationKind {
    /// Resolution rules or precedents.
    Resolution,
    /// Logical probability inputs.
    LogicalArbitrage,
    /// Resolved forecasts.
    Calibration,
    /// L2 book events/snapshots.
    L2Reconstruction,
    /// Event lifecycle observations.
    EventDiscovery,
    /// Transcript segments.
    Transcripts,
    /// Derivatives observations.
    Derivatives,
    /// Public-address observations.
    PublicAddress,
    /// Reconciliation cycles.
    ContinuousReconciliation,
    /// Vintage-aware macroeconomic series observations.
    MacroSeries,
    /// Regulatory filing accession observations.
    Filings,
}

impl IntelligenceObservationKind {
    /// Stable terminal capability identifier.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Resolution => "resolution",
            Self::LogicalArbitrage => "logical-arbitrage",
            Self::Calibration => "calibration",
            Self::L2Reconstruction => "l2-reconstruction",
            Self::EventDiscovery => "event-discovery",
            Self::Transcripts => "transcripts",
            Self::Derivatives => "derivatives",
            Self::PublicAddress => "public-address",
            Self::ContinuousReconciliation => "continuous-reconciliation",
            Self::MacroSeries => "macro-series",
            Self::Filings => "filings",
        }
    }
}

/// Observation counts by capability.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntelligenceCounts {
    /// Stable capability counts.
    pub by_capability: BTreeMap<String, u64>,
}

/// Append-only, deduplicated evidence registry. The evidence payload remains in
/// its governed domain store; this ledger only connects it to product surfaces.
#[derive(Clone, Debug, Default)]
pub struct IntelligenceLedger {
    seen: BTreeSet<(IntelligenceObservationKind, String)>,
    entries: Vec<(IntelligenceObservationKind, String, OffsetDateTime)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct IntelligenceLedgerEntry {
    kind: IntelligenceObservationKind,
    evidence_id: String,
    #[serde(with = "time::serde::rfc3339")]
    observed_at: OffsetDateTime,
}

/// Durable intelligence-ledger failure.
#[derive(Debug, thiserror::Error)]
pub enum IntelligenceLedgerError {
    /// Filesystem operation failed.
    #[error("intelligence ledger I/O at {path}: {source}")]
    Io {
        /// Affected path.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
    /// A persisted line could not be decoded.
    #[error("invalid intelligence ledger entry at {path}, line {line}: {source}")]
    Decode {
        /// Affected path.
        path: PathBuf,
        /// One-based line number.
        line: usize,
        /// Underlying error.
        source: serde_json::Error,
    },
    /// The evidence identifier was empty.
    #[error("evidence id is required")]
    EmptyEvidenceId,
}

/// Append-only JSONL-backed evidence registry. Existing entries are replayed
/// and validated on open; duplicate deliveries are not written again.
#[derive(Debug)]
pub struct FileIntelligenceLedger {
    path: PathBuf,
    file: File,
    ledger: IntelligenceLedger,
}

impl IntelligenceLedger {
    /// Register a real evidence record; duplicate delivery is idempotent.
    pub fn append(
        &mut self,
        kind: IntelligenceObservationKind,
        evidence_id: impl Into<String>,
        observed_at: OffsetDateTime,
    ) -> Result<bool, &'static str> {
        let evidence_id = evidence_id.into();
        if evidence_id.trim().is_empty() {
            return Err("evidence id is required");
        }
        let key = (kind, evidence_id.clone());
        if !self.seen.insert(key) {
            return Ok(false);
        }
        self.entries.push((kind, evidence_id, observed_at));
        Ok(true)
    }
    /// Count registered evidence by terminal capability.
    pub fn counts(&self) -> IntelligenceCounts {
        let mut by_capability = BTreeMap::new();
        for (kind, _, _) in &self.entries {
            *by_capability.entry(kind.id().into()).or_default() += 1;
        }
        IntelligenceCounts { by_capability }
    }

    fn contains(&self, kind: IntelligenceObservationKind, evidence_id: &str) -> bool {
        self.seen.contains(&(kind, evidence_id.to_owned()))
    }
}

impl FileIntelligenceLedger {
    /// Open or create a ledger at `path`, replaying every complete entry.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, IntelligenceLedgerError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| IntelligenceLedgerError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let read_file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)
            .map_err(|source| IntelligenceLedgerError::Io {
                path: path.clone(),
                source,
            })?;
        let mut ledger = IntelligenceLedger::default();
        for (index, line) in BufReader::new(&read_file).lines().enumerate() {
            let line = line.map_err(|source| IntelligenceLedgerError::Io {
                path: path.clone(),
                source,
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: IntelligenceLedgerEntry =
                serde_json::from_str(&line).map_err(|source| IntelligenceLedgerError::Decode {
                    path: path.clone(),
                    line: index + 1,
                    source,
                })?;
            ledger
                .append(entry.kind, entry.evidence_id, entry.observed_at)
                .map_err(|_| IntelligenceLedgerError::EmptyEvidenceId)?;
        }
        Ok(Self {
            path,
            file: read_file,
            ledger,
        })
    }

    /// Durably register evidence. The JSON line is flushed before success is returned.
    pub fn append(
        &mut self,
        kind: IntelligenceObservationKind,
        evidence_id: impl Into<String>,
        observed_at: OffsetDateTime,
    ) -> Result<bool, IntelligenceLedgerError> {
        let evidence_id = evidence_id.into();
        if evidence_id.trim().is_empty() {
            return Err(IntelligenceLedgerError::EmptyEvidenceId);
        }
        if self.ledger.contains(kind, &evidence_id) {
            return Ok(false);
        }
        let entry = IntelligenceLedgerEntry {
            kind,
            evidence_id: evidence_id.clone(),
            observed_at,
        };
        serde_json::to_writer(&mut self.file, &entry).map_err(|source| {
            IntelligenceLedgerError::Io {
                path: self.path.clone(),
                source: std::io::Error::other(source),
            }
        })?;
        self.file
            .write_all(b"\n")
            .and_then(|_| self.file.flush())
            .and_then(|_| self.file.sync_data())
            .map_err(|source| IntelligenceLedgerError::Io {
                path: self.path.clone(),
                source,
            })?;
        self.ledger
            .append(kind, evidence_id, observed_at)
            .map_err(|_| IntelligenceLedgerError::EmptyEvidenceId)
    }

    /// Count durable evidence by terminal capability.
    pub fn counts(&self) -> IntelligenceCounts {
        self.ledger.counts()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_evidence_is_idempotent() {
        let mut ledger = IntelligenceLedger::default();
        assert!(ledger
            .append(
                IntelligenceObservationKind::Resolution,
                "e",
                OffsetDateTime::UNIX_EPOCH
            )
            .unwrap());
        assert!(!ledger
            .append(
                IntelligenceObservationKind::Resolution,
                "e",
                OffsetDateTime::UNIX_EPOCH
            )
            .unwrap());
        assert_eq!(ledger.counts().by_capability["resolution"], 1);
    }

    #[test]
    fn file_ledger_survives_restart_without_duplicate_lines() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("intelligence.jsonl");
        {
            let mut ledger = FileIntelligenceLedger::open(&path).unwrap();
            assert!(ledger
                .append(
                    IntelligenceObservationKind::Transcripts,
                    "segment:1",
                    OffsetDateTime::UNIX_EPOCH
                )
                .unwrap());
            assert!(!ledger
                .append(
                    IntelligenceObservationKind::Transcripts,
                    "segment:1",
                    OffsetDateTime::UNIX_EPOCH
                )
                .unwrap());
        }
        let reopened = FileIntelligenceLedger::open(&path).unwrap();
        assert_eq!(reopened.counts().by_capability["transcripts"], 1);
        assert_eq!(std::fs::read_to_string(path).unwrap().lines().count(), 1);
    }
}
