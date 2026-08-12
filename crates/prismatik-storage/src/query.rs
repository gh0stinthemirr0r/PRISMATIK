//! Governed point-in-time query contracts for DuckDB/DataFusion implementations.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

/// Query resource budget.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryBudget {
    /// Maximum rows.
    pub maximum_rows: u64,
    /// Maximum bytes scanned.
    pub maximum_scan_bytes: u64,
    /// Maximum runtime milliseconds.
    pub maximum_runtime_ms: u64,
}

/// Provider-derived export rights.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportPolicy {
    /// Results may be exported.
    Permitted,
    /// Results stay local.
    LocalOnly,
    /// Export is prohibited.
    Prohibited,
}

/// Saved point-in-time query plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointInTimeQuery {
    /// Stable query identifier.
    pub id: String,
    /// Read-only SQL text.
    pub sql: String,
    /// Required observation cutoff.
    pub as_of: OffsetDateTime,
    /// Resource budget.
    pub budget: QueryBudget,
    /// Effective export policy.
    pub export_policy: ExportPolicy,
    /// Dataset/manifest digests used by the query.
    pub pinned_inputs: Vec<String>,
}

/// Query validation failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum QueryValidationError {
    /// Query is not read only.
    #[error("only a single SELECT/WITH query is permitted")]
    NotReadOnly,
    /// Missing point-in-time cutoff.
    #[error("query has no pinned input")]
    MissingPinnedInput,
    /// Budget has zero dimension.
    #[error("query budget must be nonzero")]
    InvalidBudget,
    /// Export requested against policy.
    #[error("export is not permitted")]
    ExportDenied,
}

impl PointInTimeQuery {
    /// Validate the plan before handing it to an execution backend.
    pub fn validate(&self, export_requested: bool) -> Result<(), QueryValidationError> {
        let normalized = self.sql.trim().to_ascii_lowercase();
        if !(normalized.starts_with("select ") || normalized.starts_with("with "))
            || normalized.contains(';')
        {
            return Err(QueryValidationError::NotReadOnly);
        }
        if self.pinned_inputs.is_empty() {
            return Err(QueryValidationError::MissingPinnedInput);
        }
        if self.budget.maximum_rows == 0
            || self.budget.maximum_scan_bytes == 0
            || self.budget.maximum_runtime_ms == 0
        {
            return Err(QueryValidationError::InvalidBudget);
        }
        if export_requested && self.export_policy != ExportPolicy::Permitted {
            return Err(QueryValidationError::ExportDenied);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_mutation_and_export() {
        let mut query = PointInTimeQuery {
            id: "q".into(),
            sql: "delete from x".into(),
            as_of: OffsetDateTime::UNIX_EPOCH,
            budget: QueryBudget {
                maximum_rows: 1,
                maximum_scan_bytes: 1,
                maximum_runtime_ms: 1,
            },
            export_policy: ExportPolicy::LocalOnly,
            pinned_inputs: vec!["digest".into()],
        };
        assert_eq!(
            query.validate(false),
            Err(QueryValidationError::NotReadOnly)
        );
        query.sql = "select * from x".into();
        assert_eq!(
            query.validate(true),
            Err(QueryValidationError::ExportDenied)
        );
    }
}
