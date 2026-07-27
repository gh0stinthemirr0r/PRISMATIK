//! Storage backend markers.

/// SQLite backend marker.
#[derive(Debug, Default, Clone, Copy)]
pub struct SqliteBackend;

/// DuckDB backend marker.
#[derive(Debug, Default, Clone, Copy)]
pub struct DuckdbBackend;

/// LanceDB backend marker.
#[derive(Debug, Default, Clone, Copy)]
pub struct LancedbBackend;
