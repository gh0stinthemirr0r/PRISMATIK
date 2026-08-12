//! The supervised autonomy loop.
//!
//! Before this existed, `interval_seconds` was configuration with nothing
//! reading it: the "autonomous" trader only ran when a human clicked *Run
//! once*. This module is what makes the system actually unattended.
//!
//! Three properties matter more than the scheduling itself:
//!
//! - **It stops itself.** A tripped breaker, an exhausted budget or a lockdown
//!   ladder halts the loop rather than letting it spin against a wall.
//! - **It cannot be started twice.** A second start is a no-op, not a second
//!   task racing the first through the same risk gates.
//! - **It records every tick**, including the ones that did nothing, so a quiet
//!   period is distinguishable from a dead scheduler.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, OnceLock,
};

use prismatik_determinism::{Clock, SystemClock};
use serde::{Deserialize, Serialize};

/// Shortest interval the scheduler will honour, in seconds.
///
/// Each tick fetches history for every tracked instrument; a tighter loop would
/// exhaust provider rate budgets without producing a single new daily bar.
const MIN_INTERVAL_SECONDS: u64 = 30;

/// Consecutive failing ticks before the loop suspends itself.
const MAX_CONSECUTIVE_FAILURES: u32 = 5;

static RUNNING: AtomicBool = AtomicBool::new(false);
static HISTORY: OnceLock<Mutex<Vec<TickRecord>>> = OnceLock::new();

/// One completed pass of the loop.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TickRecord {
    pub at: String,
    pub ok: bool,
    pub orders_submitted: usize,
    pub orders_skipped: usize,
    pub ladder_level: String,
    pub message: String,
}

/// Scheduler status for the operator.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerStatus {
    pub running: bool,
    pub interval_seconds: u64,
    pub mode: String,
    pub recent_ticks: Vec<TickRecord>,
    pub message: String,
}

fn history() -> &'static Mutex<Vec<TickRecord>> {
    HISTORY.get_or_init(|| Mutex::new(Vec::new()))
}

fn record(tick: TickRecord) {
    if let Ok(mut log) = history().lock() {
        log.push(tick);
        // Bounded: this is an operator-facing trace, not the audit record. The
        // durable history lives in the trader's own journal.
        let len = log.len();
        if len > 200 {
            log.drain(..len - 200);
        }
    }
}

/// Start the autonomy loop.
///
/// Returns an error rather than silently succeeding when the trader is
/// disabled — starting a scheduler for a loop that will refuse to act on every
/// tick is a configuration mistake worth surfacing.
#[tauri::command]
pub(crate) fn start_autonomy_loop() -> Result<SchedulerStatus, String> {
    let config = crate::autonomous_trader::current_config()?;
    if !config.enabled {
        return Err("enable the autonomous trader before starting the loop".into());
    }
    // `swap` rather than load-then-store: two concurrent starts must not both
    // observe "not running" and each spawn a task.
    if RUNNING.swap(true, Ordering::SeqCst) {
        return status();
    }

    tauri::async_runtime::spawn(async move {
        let mut consecutive_failures = 0_u32;
        loop {
            if !RUNNING.load(Ordering::SeqCst) {
                break;
            }

            // Re-read config every tick so an interval or mode change takes
            // effect without restarting the loop.
            let live_config = match crate::autonomous_trader::current_config() {
                Ok(config) if config.enabled => config,
                Ok(_) => {
                    record(TickRecord {
                        at: SystemClock::new().now().to_string(),
                        ok: false,
                        orders_submitted: 0,
                        orders_skipped: 0,
                        ladder_level: String::new(),
                        message: "trader disabled — loop stopping".into(),
                    });
                    RUNNING.store(false, Ordering::SeqCst);
                    break;
                },
                Err(error) => {
                    record(TickRecord {
                        at: SystemClock::new().now().to_string(),
                        ok: false,
                        orders_submitted: 0,
                        orders_skipped: 0,
                        ladder_level: String::new(),
                        message: format!("config unavailable: {error}"),
                    });
                    RUNNING.store(false, Ordering::SeqCst);
                    break;
                },
            };

            match crate::autonomous_trader::run_trader_loop().await {
                Ok(result) => {
                    consecutive_failures = 0;
                    let halted = result.budget_halted || result.ladder_level == "LOCKDOWN";
                    record(TickRecord {
                        at: result.executed_at.clone(),
                        ok: true,
                        orders_submitted: result.orders_submitted,
                        orders_skipped: result.orders_skipped,
                        ladder_level: result.ladder_level.clone(),
                        message: result.message.clone(),
                    });
                    // A halted book means every further tick would abstain for
                    // the same reason. Stop and require a human to re-arm.
                    if halted {
                        record(TickRecord {
                            at: SystemClock::new().now().to_string(),
                            ok: false,
                            orders_submitted: 0,
                            orders_skipped: 0,
                            ladder_level: result.ladder_level,
                            message: "loop suspended — budget exhausted or ladder in lockdown"
                                .into(),
                        });
                        RUNNING.store(false, Ordering::SeqCst);
                        break;
                    }
                },
                Err(error) => {
                    consecutive_failures += 1;
                    record(TickRecord {
                        at: SystemClock::new().now().to_string(),
                        ok: false,
                        orders_submitted: 0,
                        orders_skipped: 0,
                        ladder_level: String::new(),
                        message: format!("tick failed: {error}"),
                    });
                    if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                        record(TickRecord {
                            at: SystemClock::new().now().to_string(),
                            ok: false,
                            orders_submitted: 0,
                            orders_skipped: 0,
                            ladder_level: String::new(),
                            message: format!(
                                "loop suspended after {MAX_CONSECUTIVE_FAILURES} consecutive failures"
                            ),
                        });
                        RUNNING.store(false, Ordering::SeqCst);
                        break;
                    }
                },
            }

            let wait = live_config.interval_seconds.max(MIN_INTERVAL_SECONDS);
            tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        }
    });

    status()
}

/// Stop the loop. Idempotent.
#[tauri::command]
pub(crate) fn stop_autonomy_loop() -> Result<SchedulerStatus, String> {
    RUNNING.store(false, Ordering::SeqCst);
    record(TickRecord {
        at: SystemClock::new().now().to_string(),
        ok: true,
        orders_submitted: 0,
        orders_skipped: 0,
        ladder_level: String::new(),
        message: "loop stopped by operator".into(),
    });
    status()
}

/// Current scheduler state and recent ticks.
#[tauri::command]
pub(crate) fn autonomy_loop_status() -> Result<SchedulerStatus, String> {
    status()
}

fn status() -> Result<SchedulerStatus, String> {
    let config = crate::autonomous_trader::current_config()?;
    let running = RUNNING.load(Ordering::SeqCst);
    let recent_ticks = history()
        .lock()
        .map(|log| log.iter().rev().take(25).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    Ok(SchedulerStatus {
        running,
        interval_seconds: config.interval_seconds.max(MIN_INTERVAL_SECONDS),
        mode: config.mode.label().to_owned(),
        message: if running {
            format!(
                "loop active every {}s in {} mode",
                config.interval_seconds.max(MIN_INTERVAL_SECONDS),
                config.mode.label()
            )
        } else {
            "loop stopped".to_owned()
        },
        recent_ticks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_running_flag_is_a_one_way_latch_per_start() {
        // Models the guarantee the scheduler relies on: only the first of two
        // concurrent starts observes `false`, so only one task is ever spawned.
        RUNNING.store(false, Ordering::SeqCst);
        assert!(!RUNNING.swap(true, Ordering::SeqCst), "first start wins");
        assert!(
            RUNNING.swap(true, Ordering::SeqCst),
            "second start is a no-op"
        );
        RUNNING.store(false, Ordering::SeqCst);
    }

    #[test]
    fn tick_history_is_bounded() {
        for i in 0..260 {
            record(TickRecord {
                at: format!("t{i}"),
                ok: true,
                orders_submitted: 0,
                orders_skipped: 0,
                ladder_level: String::new(),
                message: String::new(),
            });
        }
        let len = history().lock().unwrap().len();
        assert!(len <= 200, "history grew to {len}");
    }

    #[test]
    fn the_minimum_interval_is_enforced_upward() {
        assert_eq!(1_u64.max(MIN_INTERVAL_SECONDS), MIN_INTERVAL_SECONDS);
        assert_eq!(600_u64.max(MIN_INTERVAL_SECONDS), 600);
    }
}
