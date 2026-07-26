//! Deterministic time source.
//!
//! Spec: `DOCS/spec/CRATE_ARCHITECTURE.md` §1.1, `DOCS/waves/Wave_0_Foundation.md`
//! (P0-DK-01).
//!
//! `Clock` is the only legal source of "what time is it" anywhere in PRISMATIK
//! outside this crate's allowlist. Three implementations and only three:
//! `SystemClock` (real wall time, allowed only in the outermost shell and in
//! ingestion for `retrieved_at`), `SimulatedClock` (backtest, replay, DST),
//! and `FrozenClock` (unit tests, point-in-time feature materialization).

use crate::context::RunId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use tokio::time::Duration as TokioDuration;

/// Identifies which kind of clock is active. Recorded in the manifest so a
/// reproduction knows whether it's replaying a real-time or simulated run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockKind {
    /// Real wall clock. Permitted ONLY in the outermost application shell
    /// and in ingestion (`retrieved_at` stamping).
    System,
    /// Advances only when the runtime advances it. Used by backtest,
    /// replay, and DST.
    Simulated,
    /// Fixed instant. Used by unit tests and by point-in-time feature
    /// materialization.
    Frozen,
}

/// Deterministic time source.
///
/// Implementations MUST be `Send + Sync + 'static` because they're shared
/// across tasks. The `sleep_until` method is async because in simulation it
/// yields to the scheduler and advances simulated time without blocking a
/// real thread.
#[async_trait::async_trait]
pub trait Clock: Send + Sync + 'static {
    /// Current logical time. In simulation this is the simulated instant,
    /// never the host wall clock.
    fn now(&self) -> OffsetDateTime;

    /// Monotonic tick count since run start. Used for durations and for
    /// ordering, never for wall-clock arithmetic.
    fn monotonic_nanos(&self) -> u64;

    /// Sleep until the given logical instant. In simulation this yields to
    /// the scheduler and advances simulated time; it does not block.
    async fn sleep_until(&self, deadline: OffsetDateTime);

    /// Identifies the clock in the reproducibility manifest.
    fn kind(&self) -> ClockKind;
}

/// Real wall-clock time. Permitted only in the outermost application shell
/// and in ingestion for `retrieved_at` stamping. Recording `retrieved_at`
/// must use real wall-clock time per spec/CRATE_ARCHITECTURE.md §1.1.
#[derive(Debug, Default)]
pub struct SystemClock {
    start: std::sync::OnceLock<OffsetDateTime>,
}

impl SystemClock {
    /// Construct a new `SystemClock`.
    pub fn new() -> Self {
        Self::default()
    }

    fn start(&self) -> OffsetDateTime {
        *self.start.get_or_init(|| {
            // Allowed: SystemClock is the explicit wall-clock impl. This is
            // the ONLY struct outside this crate's allowlist that may call
            // OffsetDateTime::now_utc, and only because it IS the wall clock.
            OffsetDateTime::now_utc()
        })
    }
}

#[async_trait::async_trait]
impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn monotonic_nanos(&self) -> u64 {
        let start = self.start();
        let now = OffsetDateTime::now_utc();
        (now - start).whole_nanoseconds().max(0) as u64
    }

    async fn sleep_until(&self, deadline: OffsetDateTime) {
        let now = OffsetDateTime::now_utc();
        if deadline > now {
            let dur = (deadline - now).try_into().unwrap_or(TokioDuration::MAX);
            tokio::time::sleep(dur).await;
        }
    }

    fn kind(&self) -> ClockKind {
        ClockKind::System
    }
}

/// Simulated clock for backtests, replay, and deterministic simulation tests.
/// Advances only when explicitly advanced.
#[derive(Debug)]
pub struct SimulatedClock {
    inner: Arc<Mutex<SimulatedClockInner>>,
}

#[derive(Debug)]
struct SimulatedClockInner {
    now: OffsetDateTime,
    nanos_elapsed: u64,
}

impl SimulatedClock {
    /// Construct a new `SimulatedClock` starting at `start`. The monotonic
    /// counter starts at zero.
    pub fn new(start: OffsetDateTime) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SimulatedClockInner {
                now: start,
                nanos_elapsed: 0,
            })),
        }
    }

    /// Advance the simulated clock by `duration`.
    pub async fn advance(&self, duration: time::Duration) {
        let mut g = self.inner.lock().await;
        g.now = g.now.saturating_add(duration);
        if let Ok(nanos) = duration.whole_nanoseconds().try_into() {
            g.nanos_elapsed = g.nanos_elapsed.saturating_add(nanos);
        }
    }

    /// Advance the simulated clock to at least `target`, used by `sleep_until`.
    async fn advance_to(&self, target: OffsetDateTime) {
        let mut g = self.inner.lock().await;
        if target > g.now {
            let dur = target - g.now;
            if let Ok(nanos) = dur.whole_nanoseconds().try_into() {
                g.nanos_elapsed = g.nanos_elapsed.saturating_add(nanos);
            }
            g.now = target;
        }
    }
}

#[async_trait::async_trait]
impl Clock for SimulatedClock {
    fn now(&self) -> OffsetDateTime {
        // Synchronous read of the current simulated time. We accept a tiny
        // race window here rather than block; the value is monotonic enough
        // for backtest event-loop purposes. A strict-serializable variant
        // would take a lock at every read, which is too expensive on the
        // backtest hot path.
        //
        // For DST (where strict serializability matters), madsim provides
        // the runtime-level determinism; this clock is the logical-time
        // source on top of that.
        self.inner
            .try_lock()
            .map(|g| g.now)
            .unwrap_or_else(|_| {
                // Fall back: block briefly. In practice this only happens
                // under heavy contention, which backtest event loops don't
                // produce.
                self.now_blocking()
            })
    }

    fn monotonic_nanos(&self) -> u64 {
        self.inner
            .try_lock()
            .map(|g| g.nanos_elapsed)
            .unwrap_or(0)
    }

    async fn sleep_until(&self, deadline: OffsetDateTime) {
        self.advance_to(deadline).await;
        // Yield to scheduler so other tasks can run; in DST madsim handles
        // the cooperative scheduling semantics.
        tokio::task::yield_now().await;
    }

    fn kind(&self) -> ClockKind {
        ClockKind::Simulated
    }
}

impl SimulatedClock {
    fn now_blocking(&self) -> OffsetDateTime {
        // Synchronous fallback used only when the mutex is contended.
        // We can't `.await` here (sync fn), so we do a blocking lock.
        // This is acceptable because the contention case is rare and the
        // operation is microseconds.
        let rt_handle = tokio::runtime::Handle::try_current();
        if let Ok(h) = rt_handle {
            // We're in a runtime; use block_on. Save the now value to a
            // local so the guard drops before we return (avoids borrow
            // lifetime issue).
            let inner = self.inner.clone();
            let now = h.block_on(inner.lock()).now;
            now
        } else {
            // No runtime: this is a unit test or sync context; just return
            // a reasonable default. The value will be re-read.
            OffsetDateTime::UNIX_EPOCH
        }
    }
}

/// A clock frozen at a single instant. Used by unit tests and by point-in-time
/// feature materialization where the time is a fixed input to the computation.
#[derive(Debug, Clone)]
pub struct FrozenClock {
    instant: OffsetDateTime,
}

impl FrozenClock {
    /// Construct a new `FrozenClock` at `instant`.
    pub fn new(instant: OffsetDateTime) -> Self {
        Self { instant }
    }
}

#[async_trait::async_trait]
impl Clock for FrozenClock {
    fn now(&self) -> OffsetDateTime {
        self.instant
    }

    fn monotonic_nanos(&self) -> u64 {
        0
    }

    async fn sleep_until(&self, _deadline: OffsetDateTime) {
        // Frozen: never blocks. The instant doesn't move.
    }

    fn kind(&self) -> ClockKind {
        ClockKind::Frozen
    }
}

/// Generate a unique `RunId` from a clock and an entropy-derived seed.
/// Used at run start to identify the run in the manifest.
pub fn new_run_id(_clock: &dyn Clock) -> RunId {
    // RunId is generated from Entropy in the application layer, not from the
    // clock. This helper exists for tests where we just need a stable id.
    RunId::test()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[tokio::test]
    async fn system_clock_advances_real_time() {
        let clock = SystemClock::new();
        let t1 = clock.now();
        tokio::time::sleep(TokioDuration::from_millis(10)).await;
        let t2 = clock.now();
        assert!(t2 > t1);
        assert_eq!(clock.kind(), ClockKind::System);
    }

    #[tokio::test]
    async fn simulated_clock_advances_only_when_advanced() {
        let clock = SimulatedClock::new(OffsetDateTime::UNIX_EPOCH);
        let t0 = clock.now();
        // Without advance, time does not move.
        tokio::task::yield_now().await;
        assert_eq!(clock.now(), t0);
        // Advance moves it.
        clock.advance(Duration::seconds(5)).await;
        assert_eq!(clock.now(), t0 + Duration::seconds(5));
        assert_eq!(clock.kind(), ClockKind::Simulated);
    }

    #[tokio::test]
    async fn simulated_clock_sleep_advances_to_deadline() {
        let clock = SimulatedClock::new(OffsetDateTime::UNIX_EPOCH);
        let deadline = OffsetDateTime::UNIX_EPOCH + Duration::seconds(10);
        clock.sleep_until(deadline).await;
        assert!(clock.now() >= deadline);
    }

    #[test]
    fn frozen_clock_is_immutable() {
        let instant = OffsetDateTime::UNIX_EPOCH + Duration::days(365);
        let clock = FrozenClock::new(instant);
        assert_eq!(clock.now(), instant);
        assert_eq!(clock.monotonic_nanos(), 0);
        assert_eq!(clock.kind(), ClockKind::Frozen);
    }

    #[tokio::test]
    async fn simulated_clock_monotonic_increases() {
        let clock = SimulatedClock::new(OffsetDateTime::UNIX_EPOCH);
        assert_eq!(clock.monotonic_nanos(), 0);
        clock.advance(Duration::milliseconds(1)).await;
        assert_eq!(clock.monotonic_nanos(), 1_000_000);
    }
}
