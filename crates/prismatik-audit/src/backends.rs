//! Backend markers.

/// SQLite backend marker (desktop profile).
#[derive(Debug, Default, Clone, Copy)]
pub struct SqliteBackend;

/// Postgres backend marker (cloud/enterprise profile).
#[derive(Debug, Default, Clone, Copy)]
pub struct PostgresBackend;

/// TigerBeetle backend marker (future evaluation).
#[derive(Debug, Default, Clone, Copy)]
pub struct TigerBeetleBackend;
