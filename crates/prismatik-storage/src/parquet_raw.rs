//! Append-only partitioned Parquet raw writer (P1-DP-06).

use arrow::array::{Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use time::OffsetDateTime;

/// Raw observation to append.
#[derive(Clone, Debug)]
pub struct RawRecord {
    /// Provider id string.
    pub provider: String,
    /// Dataset name (e.g. `coingecko_markets`).
    pub dataset: String,
    /// Event time unix micros.
    pub event_time_us: i64,
    /// Retrieved at unix micros.
    pub retrieved_at_us: i64,
    /// Payload JSON.
    pub payload_json: String,
    /// Evidence / content hash hex.
    pub content_hash_hex: String,
}

/// Append errors.
#[derive(Debug, Error)]
pub enum RawAppendError {
    /// IO.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Arrow / parquet.
    #[error("arrow: {0}")]
    Arrow(String),
    /// Append-only violation.
    #[error("append-only: path already exists: {0}")]
    AlreadyExists(PathBuf),
}

/// Parquet raw writer with date partitioning.
#[derive(Debug, Clone)]
pub struct ParquetRawWriter {
    root: PathBuf,
}

impl ParquetRawWriter {
    /// Construct under raw root.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Partition path: `{root}/{provider}/{dataset}/yyyy/mm/dd/{content_hash}.parquet`
    pub fn path_for(&self, rec: &RawRecord, day: OffsetDateTime) -> PathBuf {
        self.root
            .join(&rec.provider)
            .join(&rec.dataset)
            .join(format!("{:04}", day.year()))
            .join(format!("{:02}", u8::from(day.month())))
            .join(format!("{:02}", day.day()))
            .join(format!("{}.parquet", rec.content_hash_hex))
    }

    /// Append a single-row parquet file. Rejects overwrite.
    pub fn append(&self, rec: &RawRecord) -> Result<PathBuf, RawAppendError> {
        let day = OffsetDateTime::from_unix_timestamp_nanos(rec.retrieved_at_us as i128 * 1000)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        let path = self.path_for(rec, day);
        if path.exists() {
            return Err(RawAppendError::AlreadyExists(path));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let schema = Arc::new(Schema::new(vec![
            Field::new("provider", DataType::Utf8, false),
            Field::new("dataset", DataType::Utf8, false),
            Field::new("event_time_us", DataType::Int64, false),
            Field::new("retrieved_at_us", DataType::Int64, false),
            Field::new("payload_json", DataType::Utf8, false),
            Field::new("content_hash_hex", DataType::Utf8, false),
        ]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec![rec.provider.as_str()])),
                Arc::new(StringArray::from(vec![rec.dataset.as_str()])),
                Arc::new(Int64Array::from(vec![rec.event_time_us])),
                Arc::new(Int64Array::from(vec![rec.retrieved_at_us])),
                Arc::new(StringArray::from(vec![rec.payload_json.as_str()])),
                Arc::new(StringArray::from(vec![rec.content_hash_hex.as_str()])),
            ],
        )
        .map_err(|e| RawAppendError::Arrow(e.to_string()))?;

        let file = File::create(&path)?;
        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(Default::default()))
            .build();
        let mut writer = ArrowWriter::try_new(file, schema, Some(props))
            .map_err(|e| RawAppendError::Arrow(e.to_string()))?;
        writer
            .write(&batch)
            .map_err(|e| RawAppendError::Arrow(e.to_string()))?;
        writer
            .close()
            .map_err(|e| RawAppendError::Arrow(e.to_string()))?;
        Ok(path)
    }

    /// Root.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_overwrite() {
        let dir = tempdir().unwrap();
        let w = ParquetRawWriter::new(dir.path().to_path_buf());
        let rec = RawRecord {
            provider: "coingecko".into(),
            dataset: "markets".into(),
            event_time_us: 1,
            retrieved_at_us: 1_700_000_000_000_000,
            payload_json: "{}".into(),
            content_hash_hex: "abc123".into(),
        };
        w.append(&rec).unwrap();
        assert!(matches!(
            w.append(&rec),
            Err(RawAppendError::AlreadyExists(_))
        ));
    }
}
