//! Deterministic state-transition replay suite skeleton (P0-QM-01).
//!
//! Captures a trivial pipeline trace and asserts byte-identical digests across seeds.

use blake3::Hasher;
use prismatik_determinism::{Clock, ContentHash, Entropy, FrozenClock, SplitEntropy};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;

/// One captured TRACE event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Monotonic sequence.
    pub seq: u64,
    /// Event label.
    pub label: String,
    /// Hex payload digest.
    pub payload_digest: String,
}

/// Result of a single-seed replay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayResult {
    /// Seed used.
    pub seed: u64,
    /// Ordered events.
    pub events: Vec<TraceEvent>,
    /// BLAKE3 over the canonical event stream.
    pub trace_digest: ContentHash,
}

/// Run a trivial deterministic pipeline: split entropy → hash chunks → emit events.
pub fn run_trivial_pipeline(seed: u64, steps: u32) -> ReplayResult {
    let clock: Arc<dyn Clock> = Arc::new(FrozenClock::new(OffsetDateTime::UNIX_EPOCH));
    let entropy = SplitEntropy::from_seed(seed);
    let mut events = Vec::with_capacity(steps as usize);
    let mut trace = Hasher::new();
    for seq in 0..steps {
        let _now = clock.now();
        let mut child = entropy.split(&format!("step-{seq}"));
        let mut buf = [0u8; 32];
        child.fill_bytes(&mut buf);
        let payload_digest = ContentHash::from_bytes(&buf);
        let event = TraceEvent {
            seq: seq as u64,
            label: format!("step-{seq}"),
            payload_digest: payload_digest.to_string(),
        };
        let bytes = serde_json::to_vec(&event).expect("trace event json");
        trace.update(&bytes);
        events.push(event);
    }
    ReplayResult {
        seed,
        events,
        trace_digest: ContentHash(trace.finalize()),
    }
}

/// Assert the same seed always produces the same digest.
pub fn assert_seed_stable(seed: u64, steps: u32) -> ReplayResult {
    let a = run_trivial_pipeline(seed, steps);
    let b = run_trivial_pipeline(seed, steps);
    assert_eq!(a.trace_digest, b.trace_digest);
    assert_eq!(a.events, b.events);
    a
}

/// Run the skeleton suite over the provided seeds.
pub fn dst_replay_suite(seeds: &[u64], steps: u32) -> Vec<ReplayResult> {
    seeds
        .iter()
        .copied()
        .map(|seed| assert_seed_stable(seed, steps))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dst_replay_suite_64_seeds() {
        let seeds: Vec<u64> = (1..=64).collect();
        let results = dst_replay_suite(&seeds, 8);
        assert_eq!(results.len(), 64);
        // Distinct seeds should not collide on this trivial pipeline.
        let digests: std::collections::BTreeSet<_> =
            results.iter().map(|r| r.trace_digest.to_string()).collect();
        assert_eq!(digests.len(), 64);
    }

    /// Extended suite (P0-QM-01 / turbo): 256 seeds with the same `assert_seed_stable`.
    /// Cheap enough for default CI (~4× the 64-seed suite).
    #[test]
    fn dst_replay_suite_256() {
        let seeds: Vec<u64> = (1..=256).collect();
        let results = dst_replay_suite(&seeds, 8);
        assert_eq!(results.len(), 256);
        let digests: std::collections::BTreeSet<_> =
            results.iter().map(|r| r.trace_digest.to_string()).collect();
        assert_eq!(digests.len(), 256);
    }
}
