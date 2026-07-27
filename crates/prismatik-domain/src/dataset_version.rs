//! Dataset version references.

use prismatik_determinism::ContentHash;
use serde::{Deserialize, Serialize};

/// Stable dataset version id.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DatasetVersionId(pub String);

/// Dataset version reference with optional content hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetVersionRef {
    /// Version id.
    pub id: DatasetVersionId,
    /// Optional immutable content hash.
    pub content_hash: Option<ContentHash>,
}
