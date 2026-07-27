//! 3-layer memory loop scaffolding (`P6-EX-02` / prism-insight clean-room).

use crate::entry::{EntryId, OutcomeTag};
use serde::{Deserialize, Serialize};

/// Memory-loop layer by age band.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLayer {
    /// Layer 1 (0–7d): detailed records.
    Detail,
    /// Layer 2 (8–30d): `"{sector} + {trigger} → {action} → {result}"`.
    Summary,
    /// Layer 3 (31+d): `"{condition} = {principle}"` with hit-rate stats.
    Principle,
}

/// A single memory-loop record promoted from journal entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRecord {
    /// Source journal entry.
    pub entry_id: EntryId,
    /// Layer this record currently occupies.
    pub layer: MemoryLayer,
    /// Compact textual form for the layer.
    pub text: String,
    /// Outcome when graded.
    pub outcome: Option<OutcomeTag>,
    /// Age in whole days at last promotion check.
    pub age_days: u32,
}

impl MemoryRecord {
    /// Choose the normative layer for an age in days.
    pub fn layer_for_age(age_days: u32) -> MemoryLayer {
        match age_days {
            0..=7 => MemoryLayer::Detail,
            8..=30 => MemoryLayer::Summary,
            _ => MemoryLayer::Principle,
        }
    }
}

/// In-memory 3-layer loop store (floor).
#[derive(Clone, Debug, Default)]
pub struct MemoryLoop {
    records: Vec<MemoryRecord>,
}

impl MemoryLoop {
    /// Empty loop.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest or refresh a record, assigning layer from `age_days`.
    pub fn upsert(&mut self, mut record: MemoryRecord) {
        record.layer = MemoryRecord::layer_for_age(record.age_days);
        if let Some(existing) = self
            .records
            .iter_mut()
            .find(|r| r.entry_id == record.entry_id)
        {
            *existing = record;
        } else {
            self.records.push(record);
        }
    }

    /// Records currently in a layer.
    pub fn in_layer(&self, layer: MemoryLayer) -> impl Iterator<Item = &MemoryRecord> {
        self.records.iter().filter(move |r| r.layer == layer)
    }

    /// Count of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_bands() {
        assert_eq!(MemoryRecord::layer_for_age(0), MemoryLayer::Detail);
        assert_eq!(MemoryRecord::layer_for_age(7), MemoryLayer::Detail);
        assert_eq!(MemoryRecord::layer_for_age(8), MemoryLayer::Summary);
        assert_eq!(MemoryRecord::layer_for_age(30), MemoryLayer::Summary);
        assert_eq!(MemoryRecord::layer_for_age(31), MemoryLayer::Principle);
    }
}
