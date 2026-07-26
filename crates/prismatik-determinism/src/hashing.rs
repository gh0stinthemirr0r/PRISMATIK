//! Deterministic hasher and map/set types.
//!
//! Spec: `DOCS/spec/CRATE_ARCHITECTURE.md` §1.1.
//!
//! `HashMap`/`HashSet` use `RandomState` by default, which randomizes
//! iteration order per process for DoS resistance. That's good security but
//! bad determinism. `DetHasher` uses `FxHasher` with no randomization, so
//! iteration order is stable across runs and across machines for the same
//! insert sequence.

use rustc_hash::FxHasher;
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;

/// Deterministic hasher. Stable across runs and machines.
pub type DetHasher = BuildHasherDefault<FxHasher>;

/// Deterministic `HashMap`. Drop-in replacement for `std::collections::HashMap`
/// for any map whose iteration order is observable.
///
/// **WARNING:** `FxHasher` is NOT a DoS-resistant hash. Never use `DetMap`
/// for structures populated from untrusted input without bounding the input
/// size or rate. For trusted internal use only.
pub type DetMap<K, V> = HashMap<K, V, DetHasher>;

/// Deterministic `HashSet`. Drop-in replacement for
/// `std::collections::HashSet` for any set whose iteration order is
/// observable.
pub type DetSet<T> = HashSet<T, DetHasher>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detmap_iteration_order_stable() {
        // Two maps with identical inserts → identical iteration order.
        // (This is the load-bearing determinism property for FxHasher.)
        let mut a: DetMap<u64, u64> = DetMap::default();
        let mut b: DetMap<u64, u64> = DetMap::default();
        for i in 0..100u64 {
            // Insert in pseudo-random order to surface any hash-ordering bugs.
            let k = (i.wrapping_mul(2654435761)) % 1000;
            a.insert(k, i);
            b.insert(k, i);
        }
        let a_keys: Vec<_> = a.keys().copied().collect();
        let b_keys: Vec<_> = b.keys().copied().collect();
        assert_eq!(a_keys, b_keys);
    }

    #[test]
    fn detset_iteration_order_stable() {
        let mut a: DetSet<u64> = DetSet::default();
        let mut b: DetSet<u64> = DetSet::default();
        for i in 0..100u64 {
            let k = (i.wrapping_mul(40503)) % 1000;
            a.insert(k);
            b.insert(k);
        }
        let a: Vec<_> = a.iter().copied().collect();
        let b: Vec<_> = b.iter().copied().collect();
        assert_eq!(a, b);
    }
}
