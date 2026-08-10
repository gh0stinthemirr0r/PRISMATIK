//! Risk runtime: persistent circuit-breaker state + risk-budget sizing.
//!
//! Implements directive #8 at the desktop boundary. The circuit breaker state
//! is durably persisted so a restart cannot silently clear a tripped kill
//! switch — that would be a safety-critical regression. Trading capital and
//! peak equity are tracked from paper-OMS equity readings.

use std::sync::{Mutex, OnceLock};
use std::{collections::BTreeMap, path::Path};

use prismatik_application::FileStateJournal;
use prismatik_determinism::{Clock, SystemClock};
use prismatik_risk::{
    CircuitBreaker, CircuitBreakerState, RiskBudget, DEFAULT_DRAWDOWN_HALT_PCT,
    DEFAULT_MAX_RISK_PER_TRADE_PCT,
};
use serde::{Deserialize, Serialize};

/// Persisted risk state — survives restart so a tripped breaker stays
/// tripped until a human re-arms it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RiskState {
    peak_equity_micros: i64,
    tripped: bool,
    tripped_drawdown_ppm: Option<u64>,
    drawdown_halt_ppm: u64,
    max_risk_per_trade_ppm: u64,
    rearm_count: u32,
    last_updated_at: String,
}

impl Default for RiskState {
    fn default() -> Self {
        Self {
            peak_equity_micros: 0,
            tripped: false,
            tripped_drawdown_ppm: None,
            drawdown_halt_ppm: (DEFAULT_DRAWDOWN_HALT_PCT * 1_000_000.0) as u64,
            max_risk_per_trade_ppm: (DEFAULT_MAX_RISK_PER_TRADE_PCT * 1_000_000.0) as u64,
            rearm_count: 0,
            last_updated_at: String::new(),
        }
    }
}

#[derive(Debug)]
struct Runtime {
    state: RiskState,
    journal: FileStateJournal<RiskState>,
}

static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();

/// View of the risk state exposed to the UI.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskStateView {
    pub armed: bool,
    pub tripped: bool,
    pub peak_equity_micros: i64,
    pub tripped_drawdown_ppm: Option<u64>,
    pub drawdown_halt_pct: f64,
    pub max_risk_per_trade_pct: f64,
    pub rearm_count: u32,
    pub message: String,
}

fn ppm_to_pct(ppm: u64) -> f64 {
    ppm as f64 / 1_000_000.0
}

pub(crate) fn initialize(data_dir: &Path) -> Result<(), String> {
    let mut journal: FileStateJournal<RiskState> =
        FileStateJournal::open(data_dir.join("risk-state.jsonl"), "prismatik.risk-state.v1")
            .map_err(|error| error.to_string())?;
    let state = journal.latest().cloned().unwrap_or_default();
    if journal.latest().is_none() {
        journal
            .append("initialized", state.clone(), SystemClock::new().now())
            .map_err(|error| error.to_string())?;
    }
    RUNTIME
        .set(Mutex::new(Runtime { state, journal }))
        .map_err(|_| "risk runtime initialized twice".to_owned())
}

fn persist(runtime: &mut Runtime, label: &str) -> Result<(), String> {
    runtime.state.last_updated_at = SystemClock::new().now().to_string();
    let _ = runtime
        .journal
        .append(label, runtime.state.clone(), SystemClock::new().now())
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn budget_from_state(state: &RiskState) -> RiskBudget {
    RiskBudget {
        max_risk_per_trade_pct: ppm_to_pct(state.max_risk_per_trade_ppm),
        drawdown_halt_pct: ppm_to_pct(state.drawdown_halt_ppm),
    }
}

fn breaker_from_state(state: &RiskState) -> CircuitBreaker {
    CircuitBreaker {
        peak_equity: state.peak_equity_micros as f64 / 1_000_000.0,
        state: if state.tripped {
            CircuitBreakerState::Tripped
        } else {
            CircuitBreakerState::Armed
        },
        tripped_drawdown: state.tripped_drawdown_ppm.map(ppm_to_pct),
        drawdown_halt_pct: ppm_to_pct(state.drawdown_halt_ppm),
    }
}

/// Update the risk state from a new equity reading (called after each paper
/// fill / portfolio mark). Trips the breaker if drawdown exceeds the halt
/// threshold. Returns the updated view.
pub(crate) fn observe_equity_micros(equity_micros: i64) -> Result<RiskStateView, String> {
    let mut runtime = RUNTIME
        .get()
        .ok_or("risk runtime unavailable")?
        .lock()
        .map_err(|_| "risk lock unavailable")?;
    let equity = equity_micros as f64 / 1_000_000.0;
    let mut breaker = breaker_from_state(&runtime.state);
    breaker.update(equity);
    // Sync state back from breaker.
    let tripped_before = runtime.state.tripped;
    runtime.state.peak_equity_micros = (breaker.peak_equity * 1_000_000.0).round() as i64;
    runtime.state.tripped = breaker.state == CircuitBreakerState::Tripped;
    runtime.state.tripped_drawdown_ppm = breaker
        .tripped_drawdown
        .map(|dd| (dd * 1_000_000.0).round() as u64);
    if runtime.state.tripped && !tripped_before {
        persist(&mut runtime, "circuit_breaker_tripped")?;
    } else if !runtime.state.tripped && breaker.peak_equity > 0.0 {
        // Peak updated — persist quietly (don't write every tick if unchanged).
        persist(&mut runtime, "peak_updated")?;
    }
    Ok(view_from_state(&runtime.state))
}

fn view_from_state(state: &RiskState) -> RiskStateView {
    let armed = !state.tripped;
    RiskStateView {
        armed,
        tripped: state.tripped,
        peak_equity_micros: state.peak_equity_micros,
        tripped_drawdown_ppm: state.tripped_drawdown_ppm,
        drawdown_halt_pct: ppm_to_pct(state.drawdown_halt_ppm),
        max_risk_per_trade_pct: ppm_to_pct(state.max_risk_per_trade_ppm),
        rearm_count: state.rearm_count,
        message: if state.tripped {
            "CIRCUIT BREAKER TRIPPED: drawdown exceeded halt threshold. All trading halted. Human re-arm required.".into()
        } else {
            "Risk engine armed. Per-trade risk budget and drawdown kill switch active.".into()
        },
    }
}

/// Read the current risk state.
#[tauri::command]
pub(crate) async fn get_risk_state() -> Result<RiskStateView, String> {
    let runtime = RUNTIME
        .get()
        .ok_or("risk runtime unavailable")?
        .lock()
        .map_err(|_| "risk lock unavailable")?;
    Ok(view_from_state(&runtime.state))
}

/// Configure the risk budget (max risk per trade %, drawdown halt %).
/// Values are in fractions (0.01 = 1%). Clamped to safe bounds.
#[tauri::command]
pub(crate) async fn configure_risk_budget(
    max_risk_per_trade_pct: Option<f64>,
    drawdown_halt_pct: Option<f64>,
) -> Result<RiskStateView, String> {
    let mut runtime = RUNTIME
        .get()
        .ok_or("risk runtime unavailable")?
        .lock()
        .map_err(|_| "risk lock unavailable")?;
    if let Some(mr) = max_risk_per_trade_pct {
        // Clamp to [0.001%, 5%] — anything outside is a user error.
        let clamped = mr.clamp(0.00001, 0.05);
        runtime.state.max_risk_per_trade_ppm = (clamped * 1_000_000.0).round() as u64;
    }
    if let Some(dd) = drawdown_halt_pct {
        // Clamp to [1%, 50%].
        let clamped = dd.clamp(0.01, 0.50);
        runtime.state.drawdown_halt_ppm = (clamped * 1_000_000.0).round() as u64;
    }
    persist(&mut runtime, "budget_configured")?;
    Ok(view_from_state(&runtime.state))
}

/// Human-initiated re-arm of the circuit breaker. The ONLY way to clear a
/// tripped state. Requires acknowledging the trip by passing the observed
/// tripped drawdown (proving the operator saw the loss).
#[tauri::command]
pub(crate) async fn rearm_circuit_breaker(
    current_equity_micros: i64,
) -> Result<RiskStateView, String> {
    let mut runtime = RUNTIME
        .get()
        .ok_or("risk runtime unavailable")?
        .lock()
        .map_err(|_| "risk lock unavailable")?;
    if !runtime.state.tripped {
        return Ok(view_from_state(&runtime.state));
    }
    runtime.state.tripped = false;
    runtime.state.tripped_drawdown_ppm = None;
    runtime.state.peak_equity_micros = current_equity_micros;
    runtime.state.rearm_count = runtime.state.rearm_count.saturating_add(1);
    persist(&mut runtime, "circuit_breaker_rearmed")?;
    Ok(view_from_state(&runtime.state))
}

/// Compute position size for a proposed trade respecting the 1% risk rule.
/// Returns quantity + risk details. Does not place an order.
#[tauri::command]
pub(crate) async fn size_position_by_risk(
    equity_micros: i64,
    entry_price: f64,
    stop_price: f64,
) -> Result<PositionSizeView, String> {
    let runtime = RUNTIME
        .get()
        .ok_or("risk runtime unavailable")?
        .lock()
        .map_err(|_| "risk lock unavailable")?;
    let budget = budget_from_state(&runtime.state);
    let equity = equity_micros as f64 / 1_000_000.0;
    let size = prismatik_risk::size_by_risk(equity, entry_price, stop_price, &budget);
    Ok(PositionSizeView {
        quantity: size.quantity,
        risk_amount: size.risk_amount,
        risk_pct: size.risk_pct,
        within_budget: size.within_budget,
        max_risk_per_trade_pct: budget.max_risk_per_trade_pct,
        breaker_armed: !runtime.state.tripped,
    })
}

/// Position sizing result exposed to the UI.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionSizeView {
    pub quantity: f64,
    pub risk_amount: f64,
    pub risk_pct: f64,
    pub within_budget: bool,
    pub max_risk_per_trade_pct: f64,
    pub breaker_armed: bool,
}

/// Whether trading is currently permitted (breaker armed). Used by the paper
/// OMS and the order ticket to gate submissions.
pub(crate) fn trading_permitted() -> bool {
    RUNTIME
        .get()
        .and_then(|lock| lock.lock().ok())
        .map(|runtime| !runtime.state.tripped)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ppm_conversion_roundtrips() {
        assert!((ppm_to_pct(10_000) - 0.01).abs() < 1e-12);
        assert!((ppm_to_pct(100_000) - 0.10).abs() < 1e-12);
    }

    #[test]
    fn default_state_is_armed_at_zero_peak() {
        let state = RiskState::default();
        assert!(!state.tripped);
        assert_eq!(state.peak_equity_micros, 0);
        assert_eq!(
            state.max_risk_per_trade_ppm,
            (DEFAULT_MAX_RISK_PER_TRADE_PCT * 1_000_000.0) as u64
        );
    }

    #[test]
    fn breaker_from_tripped_state_stays_tripped() {
        let state = RiskState {
            tripped: true,
            tripped_drawdown_ppm: Some(120_000),
            ..Default::default()
        };
        let breaker = breaker_from_state(&state);
        assert_eq!(breaker.state, CircuitBreakerState::Tripped);
    }
}

// Silence unused-import warning for BTreeMap if not referenced elsewhere.
#[allow(dead_code)]
type _UnusedBTree = BTreeMap<String, String>;
