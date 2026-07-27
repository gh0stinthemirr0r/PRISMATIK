//! Storage profile definitions.

use serde::{Deserialize, Serialize};

/// Storage profile selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageProfile {
    /// Single-user local profile.
    Desktop,
    /// Cloud profile.
    Cloud,
    /// Enterprise profile.
    Enterprise,
}

/// Desktop storage profile configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopProfile {
    /// Base data directory.
    pub data_dir: String,
}

/// Cloud storage profile configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudProfile {
    /// DSN/connection string.
    pub connection_string: String,
}

/// Enterprise storage profile configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnterpriseProfile {
    /// DSN/connection string.
    pub connection_string: String,
    /// Tenant identifier.
    pub tenant_id: String,
}
