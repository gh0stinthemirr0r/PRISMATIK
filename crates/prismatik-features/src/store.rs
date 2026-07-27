//! Feature-store interface.

use crate::FeatureViewId;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

/// Entity/time selector.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityTimeFrame {
    /// Entity id.
    pub entity_id: String,
    /// As-of timestamp.
    pub as_of: OffsetDateTime,
}

/// Materialization report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializationReport {
    /// Rows materialized.
    pub rows: u64,
    /// View id.
    pub view_id: FeatureViewId,
}

/// Feature-store errors.
#[derive(Debug, Error)]
pub enum FeatureError {
    /// Unknown feature view.
    #[error("feature view not found: {0}")]
    ViewNotFound(String),
    /// PIT invariant violation.
    #[error("point-in-time violation: {0}")]
    PointInTimeViolation(String),
    /// Generic backend error.
    #[error("feature backend error: {0}")]
    Backend(String),
}

/// Feature-store API.
#[async_trait]
pub trait FeatureStore: Send + Sync {
    /// Materialize a view for a time frame.
    async fn materialize(
        &self,
        view_id: &FeatureViewId,
        timeframe: &EntityTimeFrame,
    ) -> Result<MaterializationReport, FeatureError>;
}
