//! Ingestion task contracts.

use prismatik_domain::DataLayer;
use serde::{Deserialize, Serialize};

/// Pipeline task descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineTask {
    /// Task id.
    pub id: String,
    /// Source layer.
    pub from_layer: DataLayer,
    /// Target layer.
    pub to_layer: DataLayer,
}
