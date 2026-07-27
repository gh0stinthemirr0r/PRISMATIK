//! # prismatik-determinism
//!
//! The single source of time, entropy, and deterministic ordering for the
//! entire platform.
//!
//! Spec: `DOCS/spec/CRATE_ARCHITECTURE.md` §1.1
//! Wave plan: `DOCS/waves/Wave_0_Foundation.md` (P0-DK-01..P0-DK-04)
//!
//! ## Why this crate exists
//!
//! A Rust application of PRISMATIK's size has at least seven doors through
//! which nondeterminism enters: ambient `Instant::now`/`SystemTime::now`,
//! libc-level `getrandom`, `HashMap`/`HashSet` randomized iteration order,
//! multi-threaded tokio task scheduling order, Rayon floating-point reduction
//! order, network/disk timing, and silently upgraded reference data. A seed
//! in a manifest addresses door one partially and nothing else. This crate
//! addresses all seven by being the only thing every other crate borrows
//! time, entropy, and ordering from.
//!
//! ## Enforcement
//!
//! Three mechanisms, all in CI:
//! 1. `clippy.toml` `disallowed-methods`/`disallowed-types` (this crate has a
//!    scoped `#![allow]`; all others do not).
//! 2. The `determinism-grep` script (catches ambient usage the linter misses).
//! 3. `dst_replay_suite` (madsim-based byte-identical replay).

// This crate is the *only* place allowed to touch ambient time and entropy.
// The clippy `disallowed-methods` config has an allowlist scoped to this crate
// via the module-level allows below. Every other crate must use the traits
// defined here.
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]
#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub mod artifact_ref;
pub mod clock;
pub mod context;
pub mod entropy;
pub mod error;
pub mod hashing;
pub mod signature;

pub use artifact_ref::{ArtifactId, ArtifactKind, ArtifactRef, ContentHash, SemanticVersion};
pub use clock::{Clock, ClockKind, FrozenClock, SimulatedClock, SystemClock};
pub use context::{DeterminismContext, PinnedArtifactSet, RunId};
pub use entropy::{Entropy, EntropyError, SplitEntropy};
pub use error::DeterminismError;
pub use hashing::{DetHasher, DetMap, DetSet};
pub use signature::{DualSignature, SigningIdentity};

/// Re-export of common types for ergonomic imports.
///
/// ```
/// use prismatik_determinism::prelude::*;
/// ```
pub mod prelude {
    pub use crate::artifact_ref::{
        ArtifactId, ArtifactKind, ArtifactRef, ContentHash, SemanticVersion,
    };
    pub use crate::clock::{Clock, ClockKind, FrozenClock, SimulatedClock, SystemClock};
    pub use crate::context::{DeterminismContext, PinnedArtifactSet, RunId};
    pub use crate::entropy::{Entropy, EntropyError, SplitEntropy};
    pub use crate::error::DeterminismError;
    pub use crate::hashing::{DetHasher, DetMap, DetSet};
    pub use crate::signature::{DualSignature, SigningIdentity};
}

pub mod artifact_store;
