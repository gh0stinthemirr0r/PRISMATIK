//! Migration contracts.

use serde::{Deserialize, Serialize};

/// Forward-only migration descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Migration {
    /// Numeric migration id.
    pub id: u32,
    /// Migration name.
    pub name: String,
    /// SQL payload.
    pub sql: String,
}

impl Migration {
    /// Canonical file name in `M{N:04}_{name}.sql` format.
    pub fn file_name(&self) -> String {
        format!("M{:04}_{}.sql", self.id, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_name_format() {
        let m = Migration {
            id: 12,
            name: "create_tables".into(),
            sql: "select 1".into(),
        };
        assert_eq!(m.file_name(), "M0012_create_tables.sql");
    }
}
