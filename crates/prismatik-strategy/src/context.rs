//! Strategy and determinism context placeholders (`P4-QM-06` prep).
//!
//! **Gate status:** Wave 3 is BLOCKED on incomplete `P0-REMAINDER`.
//! These are scaffolding only — not the full kernel
//! [`prismatik_determinism::DeterminismContext`] wiring, scoped data access,
//! or DataFusion-backed universe resolution.

/// Prep-only determinism bundle carried into strategy callbacks.
///
/// This is intentionally a thin placeholder (seed only). Wave 3 will wrap the
/// real kernel `DeterminismContext` (clock, entropy, pinned artifacts) once the
/// hard gate clears. Do not treat this type as Wave 3 exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeterminismContext {
    /// Deterministic seed for this run (u64 floor; entropy stream deferred).
    pub seed: u64,
}

impl DeterminismContext {
    /// Construct from a seed.
    pub const fn from_seed(seed: u64) -> Self {
        Self { seed }
    }
}

/// Per-invocation context passed into [`crate::Strategy`] callbacks.
///
/// Floor fields only: identity, bar cursor, as-of time, and determinism seed.
/// Clock/entropy traits, symbology, and scoped data access are deferred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrategyContext {
    /// Stable strategy id (matches [`crate::StrategyIr::strategy_id`]).
    pub strategy_id: String,
    /// Zero-based bar index in the current run.
    pub bar_index: u64,
    /// As-of timestamp in UTC microseconds since the Unix epoch.
    pub as_of_micros: u64,
    /// Determinism placeholder (seed for now).
    pub determinism: DeterminismContext,
}

impl StrategyContext {
    /// Construct a minimal context for unit tests and stubs.
    pub fn new(
        strategy_id: impl Into<String>,
        bar_index: u64,
        as_of_micros: u64,
        seed: u64,
    ) -> Self {
        Self {
            strategy_id: strategy_id.into(),
            bar_index,
            as_of_micros,
            determinism: DeterminismContext::from_seed(seed),
        }
    }

    /// Deterministic seed from the nested determinism placeholder.
    pub fn seed(&self) -> u64 {
        self.determinism.seed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_carries_floor_fields() {
        let ctx = StrategyContext::new("strat-1", 3, 1_700_000_000_000_000, 42);
        assert_eq!(ctx.strategy_id, "strat-1");
        assert_eq!(ctx.bar_index, 3);
        assert_eq!(ctx.as_of_micros, 1_700_000_000_000_000);
        assert_eq!(ctx.seed(), 42);
        assert_eq!(ctx.determinism.seed, 42);
    }
}
