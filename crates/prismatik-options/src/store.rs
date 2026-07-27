//! Durable desktop options flow / chain stores (JSONL partitions).
//!
//! Layer 2 must not depend on `prismatik-storage`. These stores use append-only
//! JSONL under a caller-provided root so the application profile can map them
//! onto Parquet later. ClickHouse stays cloud/enterprise-only.

use crate::{OptionContract, OptionRight, OptionsFlowPrint};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use time::{Date, OffsetDateTime};

/// Durable store failures.
#[derive(Debug, Error)]
pub enum OptionsStoreError {
    /// Filesystem failure.
    #[error("options store io: {0}")]
    Io(String),
    /// Serialization failure.
    #[error("options store codec: {0}")]
    Codec(String),
}

/// Append-only print-level flow store for desktop profiles.
#[derive(Debug)]
pub struct JsonlFlowStore {
    root: PathBuf,
}

impl JsonlFlowStore {
    /// Open or create a store rooted at `root`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, OptionsStoreError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|error| OptionsStoreError::Io(error.to_string()))?;
        Ok(Self { root })
    }

    fn path_for(&self, underlying_root: &str, day: Date) -> PathBuf {
        self.root.join(format!(
            "flow/{underlying_root}/{year:04}/{month:02}/{day:02}.jsonl",
            year = day.year(),
            month = u8::from(day.month()),
            day = day.day(),
        ))
    }

    /// Append prints; partitions by underlying OCC root and UTC calendar day.
    pub fn append(&self, prints: &[OptionsFlowPrint]) -> Result<(), OptionsStoreError> {
        for print in prints {
            let path = self.path_for(&print.contract.root, print.executed_at.date());
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| OptionsStoreError::Io(error.to_string()))?;
            }
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|error| OptionsStoreError::Io(error.to_string()))?;
            let line = serde_json::to_string(print)
                .map_err(|error| OptionsStoreError::Codec(error.to_string()))?;
            writeln!(file, "{line}").map_err(|error| OptionsStoreError::Io(error.to_string()))?;
        }
        Ok(())
    }

    /// Query prints for an underlying OCC root in `[from, to)`.
    pub fn query(
        &self,
        underlying_root: &str,
        from: OffsetDateTime,
        to: OffsetDateTime,
    ) -> Result<Vec<OptionsFlowPrint>, OptionsStoreError> {
        let mut out = Vec::new();
        let dir = self.root.join(format!("flow/{underlying_root}"));
        if !dir.exists() {
            return Ok(out);
        }
        for year in walk_dirs(&dir)? {
            for month in walk_dirs(&year)? {
                for file in walk_files(&month)? {
                    if file.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                        continue;
                    }
                    read_jsonl_prints(&file, from, to, &mut out)?;
                }
            }
        }
        out.sort_by_key(|print| print.executed_at);
        Ok(out)
    }
}

fn read_jsonl_prints(
    file: &Path,
    from: OffsetDateTime,
    to: OffsetDateTime,
    out: &mut Vec<OptionsFlowPrint>,
) -> Result<(), OptionsStoreError> {
    let reader = BufReader::new(
        fs::File::open(file).map_err(|error| OptionsStoreError::Io(error.to_string()))?,
    );
    for line in reader.lines() {
        let line = line.map_err(|error| OptionsStoreError::Io(error.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        let print: OptionsFlowPrint = serde_json::from_str(&line)
            .map_err(|error| OptionsStoreError::Codec(error.to_string()))?;
        if print.executed_at >= from && print.executed_at < to {
            out.push(print);
        }
    }
    Ok(())
}

/// One row of a historical chain snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChainSnapshotRow {
    /// Contract identity.
    pub contract: OptionContract,
    /// Snapshot market time.
    pub as_of: OffsetDateTime,
    /// Mid IV.
    pub implied_volatility: f64,
    /// Bid.
    pub bid: f64,
    /// Ask.
    pub ask: f64,
    /// Open interest.
    pub open_interest: u64,
}

#[derive(Serialize, Deserialize)]
struct CompactChainRow {
    root: String,
    expiry: String,
    right: char,
    strike_millis: u64,
    as_of: OffsetDateTime,
    implied_volatility: f64,
    bid: f64,
    ask: f64,
    open_interest: u64,
}

/// Historical chain / IV surface store with dictionary-style root compression.
#[derive(Debug)]
pub struct JsonlChainStore {
    root: PathBuf,
}

impl JsonlChainStore {
    /// Open or create a chain store.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, OptionsStoreError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|error| OptionsStoreError::Io(error.to_string()))?;
        Ok(Self { root })
    }

    fn path_for(&self, underlying_root: &str, day: Date) -> PathBuf {
        self.root.join(format!(
            "chain/{underlying_root}/{year:04}/{month:02}/{day:02}.jsonl",
            year = day.year(),
            month = u8::from(day.month()),
            day = day.day(),
        ))
    }

    /// Append chain rows.
    pub fn append(&self, rows: &[ChainSnapshotRow]) -> Result<(), OptionsStoreError> {
        for row in rows {
            let path = self.path_for(&row.contract.root, row.as_of.date());
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| OptionsStoreError::Io(error.to_string()))?;
            }
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|error| OptionsStoreError::Io(error.to_string()))?;
            let compact = CompactChainRow {
                root: row.contract.root.clone(),
                expiry: row.contract.expiry.to_string(),
                right: match row.contract.right {
                    OptionRight::Call => 'C',
                    OptionRight::Put => 'P',
                },
                strike_millis: row.contract.strike_millis,
                as_of: row.as_of,
                implied_volatility: row.implied_volatility,
                bid: row.bid,
                ask: row.ask,
                open_interest: row.open_interest,
            };
            let line = serde_json::to_string(&compact)
                .map_err(|error| OptionsStoreError::Codec(error.to_string()))?;
            writeln!(file, "{line}").map_err(|error| OptionsStoreError::Io(error.to_string()))?;
        }
        Ok(())
    }

    /// Load IV surface points for an underlying root on/after `from`.
    pub fn iv_surface(
        &self,
        underlying_root: &str,
        from: OffsetDateTime,
    ) -> Result<Vec<ChainSnapshotRow>, OptionsStoreError> {
        let mut out = Vec::new();
        let dir = self.root.join(format!("chain/{underlying_root}"));
        if !dir.exists() {
            return Ok(out);
        }
        for year in walk_dirs(&dir)? {
            for month in walk_dirs(&year)? {
                for file in walk_files(&month)? {
                    let reader = BufReader::new(
                        fs::File::open(&file)
                            .map_err(|error| OptionsStoreError::Io(error.to_string()))?,
                    );
                    for line in reader.lines() {
                        let line =
                            line.map_err(|error| OptionsStoreError::Io(error.to_string()))?;
                        if line.trim().is_empty() {
                            continue;
                        }
                        let compact: CompactChainRow = serde_json::from_str(&line)
                            .map_err(|error| OptionsStoreError::Codec(error.to_string()))?;
                        if compact.as_of < from {
                            continue;
                        }
                        let expiry = parse_ymd(&compact.expiry)?;
                        out.push(ChainSnapshotRow {
                            contract: OptionContract {
                                underlying: prismatik_identity::AssetId::from_canonical_bytes(
                                    compact.root.as_bytes(),
                                ),
                                root: compact.root,
                                expiry,
                                right: if compact.right == 'C' {
                                    OptionRight::Call
                                } else {
                                    OptionRight::Put
                                },
                                strike_millis: compact.strike_millis,
                            },
                            as_of: compact.as_of,
                            implied_volatility: compact.implied_volatility,
                            bid: compact.bid,
                            ask: compact.ask,
                            open_interest: compact.open_interest,
                        });
                    }
                }
            }
        }
        Ok(out)
    }
}

fn parse_ymd(value: &str) -> Result<Date, OptionsStoreError> {
    let format = time::format_description::parse_borrowed::<2>("[year]-[month]-[day]")
        .map_err(|error| OptionsStoreError::Codec(error.to_string()))?;
    Date::parse(value, &format).map_err(|error| OptionsStoreError::Codec(error.to_string()))
}

fn walk_dirs(path: &Path) -> Result<Vec<PathBuf>, OptionsStoreError> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(path).map_err(|error| OptionsStoreError::Io(error.to_string()))? {
        let entry = entry.map_err(|error| OptionsStoreError::Io(error.to_string()))?;
        if entry
            .file_type()
            .map_err(|error| OptionsStoreError::Io(error.to_string()))?
            .is_dir()
        {
            dirs.push(entry.path());
        }
    }
    dirs.sort();
    Ok(dirs)
}

fn walk_files(path: &Path) -> Result<Vec<PathBuf>, OptionsStoreError> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path).map_err(|error| OptionsStoreError::Io(error.to_string()))? {
        let entry = entry.map_err(|error| OptionsStoreError::Io(error.to_string()))?;
        if entry
            .file_type()
            .map_err(|error| OptionsStoreError::Io(error.to_string()))?
            .is_file()
        {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(files)
}
