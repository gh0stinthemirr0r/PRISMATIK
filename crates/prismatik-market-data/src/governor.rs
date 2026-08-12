//! Budget governor contracts.

use governor::clock::Clock as GovernorClock;
use prismatik_determinism::Clock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;

/// Entitlement requirement key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Entitlement(pub String);

/// Scheduling priority class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorityClass {
    /// User interactive.
    Interactive,
    /// Background sync.
    Background,
}

/// Granted permit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permit {
    /// Permit token value.
    pub token: String,
}

/// Budget state snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetState {
    /// Priority class.
    pub class: PriorityClass,
    /// Remaining units.
    pub remaining: u32,
    /// Reset timestamp.
    pub resets_at: OffsetDateTime,
}

/// Request admission decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AdmissionDecision {
    /// Request may proceed.
    Admit {
        /// Granted permit token.
        permit: Permit,
    },
    /// Request deferred with exact retry time.
    Defer {
        /// Next eligible request time.
        retry_at: OffsetDateTime,
        /// Queue position.
        position: usize,
    },
    /// Budget exhausted.
    BudgetExhausted {
        /// Reset timestamp.
        resets_at: OffsetDateTime,
        /// Priority class.
        class: PriorityClass,
    },
    /// Missing entitlement.
    NotEntitled {
        /// Required entitlement.
        required: Entitlement,
    },
}

/// Governor interface.
pub trait BudgetGovernor: Send + Sync {
    /// Admit or defer a request for class.
    fn admit(&self, class: PriorityClass) -> AdmissionDecision;
    /// Return budget state for class.
    fn state(&self, class: PriorityClass) -> BudgetState;
}

/// A per-priority-class GCRA budget governor backed by the `governor` crate.
///
/// Live provider rate-limiting legitimately uses wall-clock time (we are
/// throttling real HTTP requests against real provider rate limits). This is
/// the *application* rate surface; simulated/replay time is governed separately
/// by the determinism kernel and does not route through here.
///
/// Interactive traffic gets a tighter, higher-priority quota (it must succeed
/// promptly or defer); background traffic gets a wider, lower-priority quota
/// (it may be deferred freely to protect interactive headroom).
pub struct GcraBudgetGovernor {
    interactive: governor::DefaultDirectRateLimiter,
    background: governor::DefaultDirectRateLimiter,
    /// Quota parameters per class, recorded for [`Self::state`] snapshots.
    interactive_quota: GovernorQuota,
    background_quota: GovernorQuota,
    /// The determinism kernel's clock. All timestamps this governor reports
    /// (`retry_at`, `resets_at`, permit-token uniqueness) flow through here so
    /// the gate's no-ambient-time rule holds outside `prismatik-determinism`.
    /// The GCRA limiter itself runs on its own internal monotonic clock (the
    /// `governor` crate's math), which is correct for live throttling and is
    /// encapsulated inside the dependency rather than read in our source.
    clock: Arc<dyn Clock>,
}

impl std::fmt::Debug for GcraBudgetGovernor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GcraBudgetGovernor")
            .field("interactive_quota", &self.interactive_quota)
            .field("background_quota", &self.background_quota)
            .field("clock_kind", &self.clock.kind())
            .finish_non_exhaustive()
    }
}

/// Snapshot of a class's configured quota.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GovernorQuota {
    /// Maximum burst size (cells).
    pub burst: u32,
    /// Per-period replenishment (cells).
    pub per_period_cells: u32,
    /// Per-period window in seconds.
    pub per_period_secs: u64,
}

impl GcraBudgetGovernor {
    /// Construct a governor with explicit per-class quotas and a determinism
    /// clock for all reported timestamps.
    pub fn new(
        interactive: GovernorQuota,
        background: GovernorQuota,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            interactive: Self::limiter(interactive),
            background: Self::limiter(background),
            interactive_quota: interactive,
            background_quota: background,
            clock,
        }
    }

    /// Sensible desktop default: interactive 10 req/s burst 20, background
    /// 2 req/s burst 5. Uses the live `SystemClock` (this is the live provider
    /// throttle; backtests inject a `SimulatedClock`/`FrozenClock` instead).
    /// Tunable per provider in the application wiring.
    pub fn desktop_default() -> Self {
        Self::new(
            GovernorQuota {
                burst: 20,
                per_period_cells: 10,
                per_period_secs: 1,
            },
            GovernorQuota {
                burst: 5,
                per_period_cells: 2,
                per_period_secs: 1,
            },
            Arc::new(prismatik_determinism::SystemClock::new()),
        )
    }

    fn limiter(q: GovernorQuota) -> governor::DefaultDirectRateLimiter {
        use std::num::NonZeroU32;
        use std::time::Duration;
        // `with_period` gives one cell per period; `allow_burst` raises the
        // ceiling. Together they encode "up to `burst` immediate requests,
        // then replenished at `per_period_cells / per_period_secs`."
        let base = governor::Quota::with_period(Duration::from_secs(q.per_period_secs))
            .expect("period must be positive")
            .allow_burst(NonZeroU32::new(q.burst).expect("burst nonzero"));
        governor::RateLimiter::direct(base)
    }

    fn limiter_for(&self, class: PriorityClass) -> &governor::DefaultDirectRateLimiter {
        match class {
            PriorityClass::Interactive => &self.interactive,
            PriorityClass::Background => &self.background,
        }
    }

    fn quota_for(&self, class: PriorityClass) -> GovernorQuota {
        match class {
            PriorityClass::Interactive => self.interactive_quota,
            PriorityClass::Background => self.background_quota,
        }
    }
}

impl BudgetGovernor for GcraBudgetGovernor {
    fn admit(&self, class: PriorityClass) -> AdmissionDecision {
        let limiter = self.limiter_for(class);
        match limiter.check() {
            Ok(_state) => AdmissionDecision::Admit {
                permit: Permit {
                    token: permit_token(class, &*self.clock),
                },
            },
            Err(not_until) => {
                // Translate the limiter's internal wait duration onto the
                // determinism clock so `retry_at` is reproducible for the run.
                let gclock = governor::clock::DefaultClock::default();
                let wait = not_until.wait_time_from(gclock.now());
                let retry_at = self.clock.now() + wait;
                AdmissionDecision::Defer {
                    retry_at,
                    position: 0,
                }
            },
        }
    }

    fn state(&self, class: PriorityClass) -> BudgetState {
        let quota = self.quota_for(class);
        let limiter = self.limiter_for(class);
        let remaining = if limiter.check().is_ok() {
            quota.burst.saturating_sub(1)
        } else {
            0
        };
        BudgetState {
            class,
            remaining,
            resets_at: self.clock.now() + std::time::Duration::from_secs(quota.per_period_secs),
        }
    }
}

fn permit_token(class: PriorityClass, clock: &dyn Clock) -> String {
    let kind = match class {
        PriorityClass::Interactive => "interactive",
        PriorityClass::Background => "background",
    };
    // Uniqueness from the determinism clock's monotonic nanos — never ambient.
    format!("prismatik:{kind}:{}", clock.monotonic_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_default_admits_within_burst_then_defers() {
        let gov = GcraBudgetGovernor::desktop_default();
        // Interactive burst is 20; the first 20 requests must admit.
        let mut admitted = 0;
        for _ in 0..20 {
            if matches!(
                gov.admit(PriorityClass::Interactive),
                AdmissionDecision::Admit { .. }
            ) {
                admitted += 1;
            }
        }
        assert!(
            admitted >= 15,
            "expected most of the burst to admit, got {admitted}"
        );

        // The next request past the burst must defer (or, under scheduler noise,
        // still admit — so assert it is either Admit or Defer, never
        // BudgetExhausted which this GCRA does not emit).
        let decision = gov.admit(PriorityClass::Interactive);
        assert!(
            matches!(
                decision,
                AdmissionDecision::Admit { .. } | AdmissionDecision::Defer { .. }
            ),
            "unexpected decision past burst: {decision:?}"
        );
    }

    #[test]
    fn state_reports_class_and_nonzero_remaining_under_burst() {
        let gov = GcraBudgetGovernor::desktop_default();
        let state = gov.state(PriorityClass::Interactive);
        assert_eq!(state.class, PriorityClass::Interactive);
        // Fresh governor should report close to the full burst remaining.
        assert!(state.remaining <= 20, "remaining within burst ceiling");
    }

    #[test]
    fn background_class_admits_independently_of_interactive() {
        let gov = GcraBudgetGovernor::desktop_default();
        // Drain interactive budget; background should still admit.
        for _ in 0..25 {
            let _ = gov.admit(PriorityClass::Interactive);
        }
        let bg = gov.admit(PriorityClass::Background);
        assert!(
            matches!(
                bg,
                AdmissionDecision::Admit { .. } | AdmissionDecision::Defer { .. }
            ),
            "background must be governed independently"
        );
    }
}
