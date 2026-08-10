//! Desktop autonomy policy with restart-safe, fail-closed monetary accounting.

use std::sync::{OnceLock, RwLock};
use std::{collections::BTreeMap, path::Path};

use prismatik_application::{
    AutonomyBudget, AutonomyBudgetEvent, AutonomyBudgetPolicy, AutonomyBudgetSnapshot, BudgetLane,
    FileAutonomyJournal, FileStateJournal,
};
use prismatik_determinism::{Clock, SystemClock};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BudgetStateV2 {
    snapshot: AutonomyBudgetSnapshot,
    pending_trading: BTreeMap<String, u64>,
    migrated_v1_head: String,
}

struct BudgetRuntime {
    budget: AutonomyBudget,
    state: BudgetStateV2,
    journal: FileStateJournal<BudgetStateV2>,
}

impl std::fmt::Debug for BudgetRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BudgetRuntime")
            .field("budget", &self.budget)
            .field("journal", &self.journal)
            .finish()
    }
}

static RUNTIME: OnceLock<RwLock<BudgetRuntime>> = OnceLock::new();

fn default_policy() -> AutonomyBudgetPolicy {
    AutonomyBudgetPolicy {
        currency: "USD".into(),
        operations_limit_micros: 25_000_000,
        trading_limit_micros: 0,
        per_operation_limit_micros: 1_000_000,
        per_trade_limit_micros: 1,
    }
}

pub(crate) fn initialize(data_dir: &Path) -> Result<(), String> {
    // The immutable v1 chain remains part of startup verification and its head
    // is pinned into the first v2 state. All new mutations use v2 so tagged
    // reservations can be recovered without changing v1 serialization.
    let mut v1 = FileAutonomyJournal::open(data_dir.join("autonomy-budget.jsonl"))
        .map_err(|error| error.to_string())?;
    let migrated_snapshot = match v1.latest().cloned() {
        Some(snapshot) => snapshot,
        None => {
            let budget =
                AutonomyBudget::new(default_policy()).map_err(|error| error.to_string())?;
            let snapshot = budget.snapshot().map_err(|error| error.to_string())?;
            v1.append(
                AutonomyBudgetEvent::Initialize,
                snapshot.clone(),
                SystemClock::new().now(),
            )
            .map_err(|error| error.to_string())?;
            snapshot
        },
    };
    let mut journal: FileStateJournal<BudgetStateV2> = FileStateJournal::open(
        data_dir.join("autonomy-budget-v2.jsonl"),
        "prismatik.autonomy-budget.v2",
    )
    .map_err(|error| error.to_string())?;
    let state = journal.latest().cloned().unwrap_or(BudgetStateV2 {
        snapshot: migrated_snapshot,
        pending_trading: BTreeMap::new(),
        migrated_v1_head: v1.tip_hash().to_string(),
    });
    if state.migrated_v1_head != v1.tip_hash().to_string() {
        return Err(
            "autonomy budget v1 migration anchor no longer matches verified history".into(),
        );
    }
    if journal.latest().is_none() {
        journal
            .append("migrated_v1", state.clone(), SystemClock::new().now())
            .map_err(|error| error.to_string())?;
    }
    let budget =
        AutonomyBudget::from_snapshot(state.snapshot.clone()).map_err(|error| error.to_string())?;
    RUNTIME
        .set(RwLock::new(BudgetRuntime {
            budget,
            state,
            journal,
        }))
        .map_err(|_| "autonomy budget runtime was initialized twice".to_owned())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutonomyBudgetView {
    #[serde(flatten)]
    pub(crate) snapshot: AutonomyBudgetSnapshot,
    pub(crate) durable: bool,
    pub(crate) journal_records: u64,
    pub(crate) journal_head: String,
}

fn view(runtime: &BudgetRuntime) -> Result<AutonomyBudgetView, String> {
    Ok(AutonomyBudgetView {
        snapshot: runtime
            .budget
            .snapshot()
            .map_err(|error| error.to_string())?,
        durable: true,
        journal_records: runtime.journal.record_count(),
        journal_head: runtime.journal.tip_hash().to_string(),
    })
}

#[tauri::command]
pub(crate) fn get_autonomy_budget() -> Result<AutonomyBudgetView, String> {
    let runtime = RUNTIME
        .get()
        .ok_or("autonomy budget runtime is not initialized")?
        .read()
        .map_err(|_| "autonomy budget state is unavailable".to_owned())?;
    view(&runtime)
}

#[tauri::command]
pub(crate) fn configure_autonomy_budget(
    policy: AutonomyBudgetPolicy,
) -> Result<AutonomyBudgetView, String> {
    let next = AutonomyBudget::new(policy).map_err(|error| error.to_string())?;
    let snapshot = next.snapshot().map_err(|error| error.to_string())?;
    let mut runtime = RUNTIME
        .get()
        .ok_or("autonomy budget runtime is not initialized")?
        .write()
        .map_err(|_| "autonomy budget state is unavailable".to_owned())?;
    if !runtime.state.pending_trading.is_empty() {
        return Err("budget policy cannot change while paper capital reservations are pending reconciliation".into());
    }
    let mut state = runtime.state.clone();
    state.snapshot = snapshot;
    runtime
        .journal
        .append("configured", state.clone(), SystemClock::new().now())
        .map_err(|error| error.to_string())?;
    runtime.budget = next;
    runtime.state = state;
    view(&runtime)
}

pub(crate) fn reserve_operations(amount_micros: u64) -> Result<AutonomyBudgetSnapshot, String> {
    mutate(BudgetLane::Operations, amount_micros, true)
}

pub(crate) fn release_operations(amount_micros: u64) -> Result<AutonomyBudgetSnapshot, String> {
    mutate(BudgetLane::Operations, amount_micros, false)
}

pub(crate) fn reserve_paper_trading(
    reservation_id: &str,
    amount_micros: u64,
) -> Result<AutonomyBudgetSnapshot, String> {
    if reservation_id.trim().is_empty() {
        return Err("paper reservation id is required".into());
    }
    let mut runtime = runtime_write()?;
    if let Some(existing) = runtime.state.pending_trading.get(reservation_id) {
        return if *existing == amount_micros {
            Ok(runtime.state.snapshot.clone())
        } else {
            Err("paper reservation id conflicts with a different notional".into())
        };
    }
    let candidate = cloned_budget(&runtime)?;
    let snapshot = candidate
        .reserve(BudgetLane::Trading, amount_micros)
        .map_err(|error| error.to_string())?;
    let mut state = runtime.state.clone();
    state.snapshot = snapshot.clone();
    state
        .pending_trading
        .insert(reservation_id.to_owned(), amount_micros);
    runtime
        .journal
        .append(
            format!("paper_reserved:{reservation_id}"),
            state.clone(),
            SystemClock::new().now(),
        )
        .map_err(|error| format!("paper reservation was not committed: {error}"))?;
    runtime.budget = candidate;
    runtime.state = state;
    Ok(snapshot)
}

pub(crate) fn commit_paper_trading(reservation_id: &str) -> Result<(), String> {
    let mut runtime = runtime_write()?;
    if !runtime.state.pending_trading.contains_key(reservation_id) {
        return Ok(());
    }
    let mut state = runtime.state.clone();
    state.pending_trading.remove(reservation_id);
    runtime
        .journal
        .append(
            format!("paper_committed:{reservation_id}"),
            state.clone(),
            SystemClock::new().now(),
        )
        .map_err(|error| error.to_string())?;
    runtime.state = state;
    Ok(())
}

pub(crate) fn release_paper_trading(reservation_id: &str) -> Result<(), String> {
    let mut runtime = runtime_write()?;
    let Some(amount) = runtime.state.pending_trading.get(reservation_id).copied() else {
        return Ok(());
    };
    let candidate = cloned_budget(&runtime)?;
    let snapshot = candidate
        .release(BudgetLane::Trading, amount)
        .map_err(|error| error.to_string())?;
    let mut state = runtime.state.clone();
    state.snapshot = snapshot;
    state.pending_trading.remove(reservation_id);
    runtime
        .journal
        .append(
            format!("paper_released:{reservation_id}"),
            state.clone(),
            SystemClock::new().now(),
        )
        .map_err(|error| error.to_string())?;
    runtime.budget = candidate;
    runtime.state = state;
    Ok(())
}

pub(crate) fn pending_paper_trading() -> Result<BTreeMap<String, u64>, String> {
    Ok(RUNTIME
        .get()
        .ok_or("autonomy budget runtime is not initialized")?
        .read()
        .map_err(|_| "autonomy budget state is unavailable".to_owned())?
        .state
        .pending_trading
        .clone())
}

fn mutate(
    lane: BudgetLane,
    amount_micros: u64,
    reserve: bool,
) -> Result<AutonomyBudgetSnapshot, String> {
    let mut runtime = runtime_write()?;
    let candidate = cloned_budget(&runtime)?;
    let snapshot = if reserve {
        candidate.reserve(lane, amount_micros)
    } else {
        candidate.release(lane, amount_micros)
    }
    .map_err(|error| error.to_string())?;
    let mut state = runtime.state.clone();
    state.snapshot = snapshot.clone();
    let event = format!(
        "{}_{}",
        if reserve { "reserved" } else { "released" },
        match lane {
            BudgetLane::Operations => "operations",
            BudgetLane::Trading => "trading",
        }
    );
    runtime
        .journal
        .append(event, state.clone(), SystemClock::new().now())
        .map_err(|error| {
            format!("budget journal persistence failed; mutation was not published: {error}")
        })?;
    runtime.budget = candidate;
    runtime.state = state;
    Ok(snapshot)
}

fn runtime_write() -> Result<std::sync::RwLockWriteGuard<'static, BudgetRuntime>, String> {
    RUNTIME
        .get()
        .ok_or("autonomy budget runtime is not initialized")?
        .write()
        .map_err(|_| "autonomy budget state is unavailable".to_owned())
}

fn cloned_budget(runtime: &BudgetRuntime) -> Result<AutonomyBudget, String> {
    AutonomyBudget::from_snapshot(runtime.state.snapshot.clone()).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::default_policy;

    #[test]
    fn built_in_policy_starts_with_trading_disabled() {
        let policy = default_policy();
        assert_eq!(policy.trading_limit_micros, 0);
        assert!(policy.operations_limit_micros > 0);
    }
}
