//! HTTP API, versioned at /api/v1. Author: Aaron Stovall · Version 0.1.0 · 2026-07-07
//!
//! Loopback-bound by default; every API route except /api/v1/health requires a
//! bearer token generated at startup and injected into the served page (same
//! origin on loopback). CORS stays closed. Long-running work runs as jobs.
//! Versioning: additive changes stay on v1; breaking changes introduce /api/v2
//! while v1 remains until clients migrate.

use crate::broker::{AlpacaBroker, PaperBroker};
use crate::config::{Settings, LIVE_ACK_PHRASE, VALID_GRANULARITIES, VERSION};
use crate::data::CoinbaseData;
use crate::engine::backtest;
use crate::journal::Journal;
use crate::live::{run_session, Execution, Session, SessionConfig, SessionState};
use crate::strategy::{apply_vol_target, Strategy, StrategySpec};
use crate::types::downsample;
use crate::walkforward::{grid_for, walk_forward};
use axum::extract::ws::{Message as WsMessage, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use prismatik_determinism::{Clock, SystemClock};
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub settings: Settings,
    pub jobs: Arc<Mutex<BTreeMap<String, Value>>>,
    pub sessions: Arc<Mutex<BTreeMap<String, Arc<Session>>>>,
    pub ui_dir: Arc<std::path::PathBuf>,
    pub clock: Arc<SystemClock>,
    pub boot_nanos: u64,
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"detail": "missing or invalid bearer token"})),
    )
        .into_response()
}

fn fresh_id() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    uuid::Uuid::from_bytes(bytes).simple().to_string()[..12].to_string()
}

fn check_token(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(value) = headers.get("authorization").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let bearer = format!("Bearer {}", state.settings.api_token);
    constant_time_eq(value.as_bytes(), bearer.as_bytes())
        || constant_time_eq(value.as_bytes(), state.settings.api_token.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[derive(Deserialize)]
pub struct RunRequest {
    pub symbol: String,
    #[serde(default = "default_granularity")]
    pub granularity_s: u32,
    #[serde(default = "default_days")]
    pub days: u32,
    #[serde(default = "default_equity")]
    pub initial_equity_usd: f64,
    pub strategy: StrategySpec,
    #[serde(default = "default_folds")]
    pub folds: usize,
}
fn default_granularity() -> u32 {
    3600
}
fn default_days() -> u32 {
    180
}
fn default_equity() -> f64 {
    100.0
}
fn default_folds() -> usize {
    5
}

fn validate_run(req: &RunRequest) -> Result<(), String> {
    if req.symbol.is_empty()
        || req.symbol.len() > 24
        || !req
            .symbol
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '/')
    {
        return Err("invalid symbol".into());
    }
    if !VALID_GRANULARITIES.contains(&req.granularity_s) {
        return Err(format!(
            "granularity must be one of {VALID_GRANULARITIES:?}"
        ));
    }
    if !(7..=1500).contains(&req.days) {
        return Err("days must be in 7..=1500".into());
    }
    if !(req.initial_equity_usd > 0.0 && req.initial_equity_usd <= 1_000_000.0) {
        return Err("initial equity out of range".into());
    }
    if !(2..=12).contains(&req.folds) {
        return Err("folds must be in 2..=12".into());
    }
    Strategy::from_spec(&req.strategy).map(|_| ())
}

async fn health() -> Json<Value> {
    Json(json!({"status": "ok", "version": VERSION, "product": "prismatik"}))
}

async fn meta(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    Json(json!({
        "strategies": ["hold", "ma", "donchian", "rsi"],
        "granularities": VALID_GRANULARITIES,
        "alpaca_configured": state.settings.alpaca.configured(),
        "live_unlocked": state.settings.alpaca.live,
        "live_ack_phrase_required": LIVE_ACK_PHRASE,
        "risk": state.settings.risk,
    }))
    .into_response()
}

async fn health_detailed(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }

    // Check Coinbase REST connectivity
    let coinbase_rest_ok = {
        let data = CoinbaseData::new(
            &state.settings.coinbase_rest,
            state.settings.rest_timeout_secs,
        );
        data.is_ok()
    };

    // Check Alpaca connectivity (if configured)
    let alpaca_ok = if state.settings.alpaca.configured() {
        // Simple check: alpaca credentials are present
        state
            .settings
            .alpaca
            .key_id
            .as_ref()
            .is_some_and(|k| !k.is_empty())
    } else {
        true // Not configured is ok
    };

    let all_ok = coinbase_rest_ok && alpaca_ok;

    Json(json!({
        "version": VERSION,
        "status": if all_ok { "healthy" } else { "degraded" },
        "timestamp_uptime_seconds": state.clock.monotonic_nanos().saturating_sub(state.boot_nanos) / 1_000_000_000,
        "components": {
            "coinbase_rest": {
                "configured": !state.settings.coinbase_rest.is_empty(),
                "url": state.settings.coinbase_rest,
                "reachable": coinbase_rest_ok,
            },
            "coinbase_ws": {
                "configured": !state.settings.coinbase_ws.is_empty(),
                "url": state.settings.coinbase_ws,
            },
            "alpaca": {
                "configured": state.settings.alpaca.configured(),
                "live_unlocked": state.settings.alpaca.live,
                "authenticated": alpaca_ok,
            },
        },
        "capabilities": {
            "backtesting_available": coinbase_rest_ok,
            "live_trading_available": alpaca_ok && state.settings.alpaca.live,
            "walkforward_available": coinbase_rest_ok,
        },
    }))
    .into_response()
}

async fn validate_strategy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RunRequest>,
) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }

    let mut errors = Vec::new();

    if req.symbol.is_empty()
        || req.symbol.len() > 24
        || !req
            .symbol
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '/')
    {
        errors.push("symbol must be non-empty, <= 24 chars, and alphanumeric/dash/slash");
    }

    if !VALID_GRANULARITIES.contains(&req.granularity_s) {
        errors.push("granularity must be one of [60, 300, 900, 3600, 21600, 86400]");
    }

    if !(7..=1500).contains(&req.days) {
        errors.push("days must be in 7..=1500");
    }

    if !(req.initial_equity_usd > 0.0 && req.initial_equity_usd <= 1_000_000.0) {
        errors.push("initial equity must be in (0, 1_000_000]");
    }

    if !(2..=12).contains(&req.folds) {
        errors.push("folds must be in 2..=12");
    }

    let strategy_valid = Strategy::from_spec(&req.strategy).is_ok();
    if !strategy_valid {
        errors.push("strategy specification is invalid");
    }

    if errors.is_empty() {
        Json(json!({
            "valid": true,
            "symbol": req.symbol,
            "granularity_s": req.granularity_s,
            "days": req.days,
            "initial_equity_usd": req.initial_equity_usd,
            "folds": req.folds,
        }))
        .into_response()
    } else {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "valid": false,
                "errors": errors,
            })),
        )
            .into_response()
    }
}

async fn config(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    Json(json!({
        "version": VERSION,
        "api_version": "v1",
        "cost_model": {
            "fee_rate": state.settings.cost.fee_rate,
            "slippage_rate": state.settings.cost.slippage_rate,
            "total_per_turnover": state.settings.cost.per_turnover(),
        },
        "risk_limits": {
            "max_position_weight": state.settings.risk.max_position_weight,
            "max_drawdown_kill": state.settings.risk.max_drawdown_kill,
            "max_notional_usd": state.settings.risk.max_notional_usd,
            "max_order_usd": state.settings.risk.max_order_usd,
        },
        "broker": {
            "alpaca_configured": state.settings.alpaca.configured(),
            "live_trading_unlocked": state.settings.alpaca.live,
        },
        "data_sources": {
            "coinbase_rest": state.settings.coinbase_rest,
            "coinbase_ws": state.settings.coinbase_ws,
            "rest_timeout_secs": state.settings.rest_timeout_secs,
        },
        "valid_granularities_s": VALID_GRANULARITIES,
        "valid_strategies": ["hold", "ma", "donchian", "rsi"],
        "backtesting": {
            "min_days": 7,
            "max_days": 1500,
            "min_folds": 2,
            "max_folds": 12,
        },
        "session": {
            "min_window_bars": 20,
            "max_window_bars": 2000,
        },
        "live_trading_ack_phrase": LIVE_ACK_PHRASE,
    }))
    .into_response()
}

async fn metrics(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    let jobs = state.jobs.lock().await;
    let sessions = state.sessions.lock().await;

    // Count jobs by status (simplified: if it has a "verdict" or "metrics" field, it's complete)
    let (jobs_complete, jobs_pending) = jobs.iter().fold((0, 0), |(c, p), (_, v)| {
        if v.get("verdict").is_some() || v.get("metrics").is_some() {
            (c + 1, p)
        } else {
            (c, p + 1)
        }
    });

    // Count sessions by state (running vs stopped)
    let sessions_running = sessions
        .iter()
        .filter(|(_, s)| !s.stop.load(Ordering::Relaxed))
        .count();
    let sessions_stopped = sessions.len() - sessions_running;

    let uptime_secs = state
        .clock
        .monotonic_nanos()
        .saturating_sub(state.boot_nanos)
        / 1_000_000_000;

    Json(json!({
        "version": VERSION,
        "uptime_seconds": uptime_secs,
        "jobs": {
            "total": jobs.len(),
            "complete": jobs_complete,
            "pending": jobs_pending,
        },
        "sessions": {
            "total": sessions.len(),
            "running": sessions_running,
            "stopped": sessions_stopped,
        },
        "data_sources": {
            "coinbase_rest": state.settings.coinbase_rest,
            "coinbase_ws": state.settings.coinbase_ws,
        },
        "broker": {
            "alpaca_configured": state.settings.alpaca.configured(),
            "alpaca_live_unlocked": state.settings.alpaca.live,
        },
        "capabilities": {
            "ui_served": state.ui_dir.join("index.html").exists(),
            "paper_trading_available": true,
            "live_trading_available": state.settings.alpaca.live,
        },
    }))
    .into_response()
}

async fn status(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    let jobs = state.jobs.lock().await;
    let sessions = state.sessions.lock().await;
    Json(json!({
        "version": VERSION,
        "uptime_seconds": state.clock.monotonic_nanos().saturating_sub(state.boot_nanos) / 1_000_000_000,
        "jobs_queued": jobs.len(),
        "sessions_open": sessions.len(),
        "alpaca_configured": state.settings.alpaca.configured(),
        "live_unlocked": state.settings.alpaca.live,
        "ui_present": state.ui_dir.join("index.html").exists(),
    })).into_response()
}

async fn overview(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    let job_entries: Vec<(String, Value)> = {
        let jobs = state.jobs.lock().await;
        jobs.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    };
    let session_entries: Vec<Arc<Session>> = {
        let sessions = state.sessions.lock().await;
        sessions.values().cloned().collect()
    };

    let latest_job = job_entries.last().map(|(job_id, entry)| {
        json!({
            "job_id": job_id,
            "status": entry.get("status").cloned().unwrap_or_else(|| json!("unknown")),
            "result": entry.get("result").cloned(),
            "error": entry.get("error").cloned(),
        })
    });

    let latest_session = if let Some(s) = session_entries.last() {
        let st = s.state.lock().await;
        let price = st.last_price.unwrap_or(0.0);
        let equity = match &st.execution {
            Execution::Paper(b) => b.equity(price),
            Execution::Alpaca {
                tracked_units,
                entry_equity,
                ..
            } => entry_equity + tracked_units * price,
        };
        Some(json!({
            "session_id": s.id,
            "running": !s.stop.load(Ordering::Relaxed),
            "halted": st.halted,
            "live": st.live,
            "symbol": s.cfg.symbol,
            "strategy": st.strategy_name,
            "last_price": st.last_price,
            "equity": equity,
        }))
    } else {
        None
    };

    Json(json!({
        "version": VERSION,
        "uptime_seconds": state.clock.monotonic_nanos().saturating_sub(state.boot_nanos) / 1_000_000_000,
        "jobs_queued": job_entries.len(),
        "sessions_open": session_entries.len(),
        "latest_job": latest_job,
        "latest_session": latest_session,
        "risk": {
            "max_order_usd": state.settings.risk.max_order_usd,
            "max_notional_usd": state.settings.risk.max_notional_usd,
            "max_drawdown_kill": state.settings.risk.max_drawdown_kill,
        },
        "readiness": if state.settings.alpaca.configured() { "broker_configured" } else { "paper_only" },
        "ui_present": state.ui_dir.join("index.html").exists(),
    })).into_response()
}

async fn jobs_list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    let jobs = state.jobs.lock().await;
    let items: Vec<Value> = jobs
        .iter()
        .map(|(id, entry)| {
            json!({
                "job_id": id,
                "status": entry.get("status").cloned().unwrap_or_else(|| json!("unknown")),
                "result": entry.get("result").cloned(),
                "error": entry.get("error").cloned(),
            })
        })
        .collect();
    Json(json!({ "jobs": items })).into_response()
}

async fn session_payload(session: Arc<Session>, tail: usize) -> Value {
    let st = session.state.lock().await;
    let price = st.last_price.unwrap_or(0.0);
    let (equity, cash, units) = match &st.execution {
        Execution::Paper(b) => (b.equity(price), b.cash_usd, b.position_units),
        Execution::Alpaca {
            tracked_units,
            entry_equity,
            ..
        } => (
            entry_equity + tracked_units * price,
            f64::NAN,
            *tracked_units,
        ),
    };
    json!({
        "session_id": session.id,
        "running": !session.stop.load(Ordering::Relaxed),
        "halted": st.halted,
        "live": st.live,
        "symbol": session.cfg.symbol,
        "strategy": st.strategy_name,
        "last_price": st.last_price,
        "equity": equity,
        "cash_usd": cash,
        "position_units": units,
        "events": session.journal.read_tail(tail),
    })
}

async fn sessions_list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    let session_refs: Vec<Arc<Session>> = {
        let sessions = state.sessions.lock().await;
        sessions.values().cloned().collect()
    };
    let mut items = Vec::with_capacity(session_refs.len());
    for s in session_refs {
        items.push(session_payload(s, 0).await);
    }
    Json(json!({ "sessions": items })).into_response()
}

#[derive(Deserialize)]
struct EventTailQuery {
    #[serde(default = "default_event_tail")]
    limit: usize,
}

fn default_event_tail() -> usize {
    200
}

#[derive(Deserialize)]
struct JobSearchQuery {
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    status: Option<String>, // "running", "done", "error"
    #[serde(default = "default_search_limit")]
    limit: usize,
}

fn default_search_limit() -> usize {
    50
}

async fn jobs_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<JobSearchQuery>,
) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    let limit = query.limit.min(1000);
    let jobs = state.jobs.lock().await;
    let mut items: Vec<Value> = jobs
        .iter()
        .filter_map(|(job_id, entry)| {
            let status = entry.get("status").and_then(|s| s.as_str());
            let result = entry.get("result");

            // Filter by status if provided
            if let Some(ref target_status) = query.status {
                if status != Some(target_status.as_str()) {
                    return None;
                }
            }

            // Filter by symbol if provided
            if let Some(ref target_symbol) = query.symbol {
                if let Some(sym) = result
                    .and_then(|r| r.get("symbol"))
                    .and_then(|s| s.as_str())
                {
                    if !sym.contains(target_symbol.as_str()) {
                        return None;
                    }
                } else {
                    return None;
                }
            }

            Some(json!({
                "job_id": job_id,
                "status": status.unwrap_or("unknown"),
                "symbol": result.and_then(|r| r.get("symbol")).cloned().unwrap_or(json!(null)),
                "kind": result.and_then(|r| r.get("kind")).cloned().unwrap_or(json!(null)),
                "verdict": result.and_then(|r| r.get("verdict")).cloned(),
                "error": entry.get("error").cloned(),
            }))
        })
        .take(limit)
        .collect();
    items.reverse(); // Most recent first
    Json(json!({ "jobs": items, "count": items.len(), "limit": limit })).into_response()
}

async fn session_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<EventTailQuery>,
) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    if !(1..=1000).contains(&query.limit) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"detail": "limit must be in 1..=1000"})),
        )
            .into_response();
    }
    let session = {
        let sessions = state.sessions.lock().await;
        sessions.get(&id).cloned()
    };
    match session {
        Some(session) => Json(json!({
            "session_id": id,
            "events": session.journal.read_tail(query.limit),
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"detail": "unknown session"})),
        )
            .into_response(),
    }
}

async fn run_job(state: AppState, kind: &'static str, req: RunRequest) -> Response {
    if let Err(e) = validate_run(&req) {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"detail": e}))).into_response();
    }
    let job_id = fresh_id();
    {
        state
            .jobs
            .lock()
            .await
            .insert(job_id.clone(), json!({"status": "running"}));
    }

    let jobs = state.jobs.clone();
    let settings = state.settings.clone();
    let id = job_id.clone();
    tokio::spawn(async move {
        let result = execute_job(&settings, kind, &req).await;
        let entry = match result {
            Ok(r) => json!({"status": "done", "result": r}),
            Err(e) => json!({"status": "error", "error": e}),
        };
        jobs.lock().await.insert(id, entry);
    });
    Json(json!({"job_id": job_id})).into_response()
}

async fn execute_job(settings: &Settings, kind: &str, req: &RunRequest) -> Result<Value, String> {
    let data = CoinbaseData::new(&settings.coinbase_rest, settings.rest_timeout_secs)
        .map_err(|e| e.to_string())?;
    let candles = data
        .fetch_candles(&req.symbol, req.granularity_s, req.days)
        .await
        .map_err(|e| e.to_string())?;
    let ppy = Settings::periods_per_year(req.granularity_s);
    let cost = settings.cost.clone();
    let max_w = settings.risk.max_position_weight;
    let equity0 = req.initial_equity_usd;
    let spec = req.strategy.clone();
    let kind = kind.to_string();
    let folds = req.folds;
    let symbol = req.symbol.clone();

    // Engine work is CPU-bound; keep it off the reactor.
    tokio::task::spawn_blocking(move || {
        if kind == "backtest" {
            let strat = Strategy::from_spec(&spec)?;
            let mut weights = strat.target_weights(&candles);
            if let Some(vt) = spec.vol_target {
                apply_vol_target(&mut weights, &candles, vt, 48, ppy, max_w);
            }
            let r = backtest(&candles, &weights, &cost, max_w, equity0, ppy)?;
            let beats = r.metrics.total_return > r.benchmark_metrics.total_return;
            Ok(json!({
                "kind": "backtest",
                "symbol": symbol,
                "strategy": strat.name(),
                "bars": candles.len(),
                "metrics": r.metrics,
                "benchmark_metrics": r.benchmark_metrics,
                "n_trades": r.n_trades,
                "cost_drag": r.total_costs,
                "beats_benchmark": beats,
                "equity": downsample(&r.equity, 1200),
                "benchmark_equity": downsample(&r.benchmark_equity, 1200),
                "verdict": if beats {
                    "Strategy beats buy and hold on this window, net of costs."
                } else {
                    "No edge survives costs on this window. Buy and hold wins."
                },
            }))
        } else {
            let grid = grid_for(&spec.kind)?;
            let rep = walk_forward(
                &candles, &spec, &grid, folds, 0.6, &cost, max_w, equity0, ppy,
            )?;
            let positive = rep.oos_metrics.total_return > 0.0;
            let verdict = if positive {
                if rep.stable {
                    "Out of sample result is positive. Necessary, not sufficient.".to_string()
                } else {
                    "Out of sample result is positive, but parameters are unstable across \
                     folds. Necessary, not sufficient."
                        .to_string()
                }
            } else {
                "Out of sample result is negative. This configuration has no demonstrated edge."
                    .to_string()
            };
            Ok(json!({
                "kind": "walkforward",
                "symbol": symbol,
                "strategy": spec.kind,
                "bars": candles.len(),
                "oos_metrics": rep.oos_metrics,
                "oos_equity": downsample(&rep.oos_equity, 1200),
                "chosen_params": rep.chosen_params,
                "param_stability": rep.param_stability,
                "stable": rep.stable,
                "n_folds": rep.n_folds,
                "configs_searched": rep.configs_searched,
                "caution": rep.caution,
                "bootstrap": rep.bootstrap,
                "verdict": verdict,
            }))
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn post_backtest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RunRequest>,
) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    run_job(state, "backtest", req).await
}

async fn post_walkforward(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RunRequest>,
) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    run_job(state, "walkforward", req).await
}

async fn get_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    match state.jobs.lock().await.get(&id) {
        Some(v) => Json(v.clone()).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"detail": "unknown job"})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct BatchJobRequest {
    jobs: Vec<RunRequest>,
}

async fn batch_backtest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<BatchJobRequest>,
) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    if req.jobs.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"detail": "batch must contain at least 1 job"})),
        )
            .into_response();
    }
    if req.jobs.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"detail": "batch cannot exceed 100 jobs"})),
        )
            .into_response();
    }
    let mut job_ids = Vec::with_capacity(req.jobs.len());
    for job_req in req.jobs {
        if let Err(e) = validate_run(&job_req) {
            return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"detail": e}))).into_response();
        }
        let job_id = fresh_id();
        job_ids.push(job_id.clone());
        {
            state
                .jobs
                .lock()
                .await
                .insert(job_id.clone(), json!({"status": "running"}));
        }
        let jobs = state.jobs.clone();
        let settings = state.settings.clone();
        let id = job_id;
        tokio::spawn(async move {
            let result = execute_job(&settings, "backtest", &job_req).await;
            let entry = match result {
                Ok(r) => json!({"status": "done", "result": r}),
                Err(e) => json!({"status": "error", "error": e}),
            };
            jobs.lock().await.insert(id, entry);
        });
    }
    Json(json!({"batch_job_ids": job_ids, "count": job_ids.len()})).into_response()
}

async fn batch_walkforward(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<BatchJobRequest>,
) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    if req.jobs.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"detail": "batch must contain at least 1 job"})),
        )
            .into_response();
    }
    if req.jobs.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"detail": "batch cannot exceed 100 jobs"})),
        )
            .into_response();
    }
    let mut job_ids = Vec::with_capacity(req.jobs.len());
    for job_req in req.jobs {
        if let Err(e) = validate_run(&job_req) {
            return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"detail": e}))).into_response();
        }
        let job_id = fresh_id();
        job_ids.push(job_id.clone());
        {
            state
                .jobs
                .lock()
                .await
                .insert(job_id.clone(), json!({"status": "running"}));
        }
        let jobs = state.jobs.clone();
        let settings = state.settings.clone();
        let id = job_id;
        tokio::spawn(async move {
            let result = execute_job(&settings, "walkforward", &job_req).await;
            let entry = match result {
                Ok(r) => json!({"status": "done", "result": r}),
                Err(e) => json!({"status": "error", "error": e}),
            };
            jobs.lock().await.insert(id, entry);
        });
    }
    Json(json!({"batch_job_ids": job_ids, "count": job_ids.len()})).into_response()
}

#[derive(Deserialize)]
pub struct SessionStart {
    pub symbol: String,
    #[serde(default = "default_granularity")]
    pub granularity_s: u32,
    pub strategy: StrategySpec,
    #[serde(default = "default_equity")]
    pub initial_equity_usd: f64,
    #[serde(default = "default_window")]
    pub window_bars: usize,
    /// "paper" (internal simulator, default) or "alpaca" (customer's account).
    #[serde(default = "default_execution")]
    pub execution: String,
    /// For alpaca execution: request the live endpoint. Requires the server
    /// env gate AND live_confirm to equal the acknowledgment phrase.
    #[serde(default)]
    pub live: bool,
    #[serde(default)]
    pub live_confirm: String,
}
fn default_window() -> usize {
    200
}
fn default_execution() -> String {
    "paper".into()
}

async fn session_start(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SessionStart>,
) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    if req.symbol.is_empty() || req.symbol.len() > 24 {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"detail": "invalid symbol"})),
        )
            .into_response();
    }
    if !VALID_GRANULARITIES.contains(&req.granularity_s) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(
                json!({"detail": format!("granularity must be one of {:?}", VALID_GRANULARITIES)}),
            ),
        )
            .into_response();
    }
    let strategy = match Strategy::from_spec(&req.strategy) {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"detail": e}))).into_response()
        },
    };

    let (execution, live) = match req.execution.as_str() {
        "paper" => (
            Execution::Paper(PaperBroker::new(req.initial_equity_usd, &state.settings)),
            false,
        ),
        "alpaca" => match AlpacaBroker::new(&state.settings, req.live, &req.live_confirm) {
            Ok(broker) => {
                let live = broker.live;
                (
                    Execution::Alpaca {
                        broker,
                        tracked_units: 0.0,
                        entry_equity: req.initial_equity_usd,
                        peak_equity: req.initial_equity_usd,
                    },
                    live,
                )
            },
            Err(e) => {
                return (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({"detail": e.to_string()})),
                )
                    .into_response()
            },
        },
        other => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({"detail": format!("unknown execution {other:?}")})),
            )
                .into_response()
        },
    };

    let id = fresh_id();
    let journal = match Journal::open(&format!("session-{id}")) {
        Ok(j) => Arc::new(j),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"detail": e.to_string()})),
            )
                .into_response()
        },
    };
    let (tx, _) = broadcast::channel(512);
    let session = Arc::new(Session {
        id: id.clone(),
        cfg: SessionConfig {
            symbol: req.symbol.clone(),
            granularity_s: req.granularity_s,
            spec: req.strategy.clone(),
            initial_equity: req.initial_equity_usd,
            window_bars: req.window_bars.clamp(20, 2000),
        },
        stop: Arc::new(AtomicBool::new(false)),
        journal,
        events: tx,
        state: Arc::new(Mutex::new(SessionState {
            execution,
            last_price: None,
            halted: false,
            live,
            strategy_name: strategy.name(),
        })),
    });

    state
        .sessions
        .lock()
        .await
        .insert(id.clone(), session.clone());
    let settings = state.settings.clone();
    tokio::spawn(run_session(session, settings, strategy));
    Json(json!({"session_id": id, "mode": if live { "alpaca_LIVE" } else { "paper" }}))
        .into_response()
}

async fn session_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    let sessions = state.sessions.lock().await;
    let Some(s) = sessions.get(&id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"detail": "unknown session"})),
        )
            .into_response();
    };
    Json(session_payload(s.clone(), 200).await).into_response()
}

async fn session_stop(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if !check_token(&state, &headers) {
        return unauthorized();
    }
    let sessions = state.sessions.lock().await;
    let Some(s) = sessions.get(&id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"detail": "unknown session"})),
        )
            .into_response();
    };
    s.stop.store(true, Ordering::Relaxed);
    Json(json!({"session_id": id, "stopped": true})).into_response()
}

#[derive(Deserialize)]
struct WsAuth {
    token: String,
}

async fn session_stream(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(auth): Query<WsAuth>,
    ws: WebSocketUpgrade,
) -> Response {
    if !constant_time_eq(auth.token.as_bytes(), state.settings.api_token.as_bytes()) {
        return unauthorized();
    }
    let rx = {
        let sessions = state.sessions.lock().await;
        match sessions.get(&id) {
            Some(s) => s.events.subscribe(),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({"detail": "unknown session"})),
                )
                    .into_response()
            },
        }
    };
    ws.on_upgrade(move |socket| pump_events(socket, rx))
}

async fn pump_events(mut socket: WebSocket, mut rx: broadcast::Receiver<Value>) {
    loop {
        match rx.recv().await {
            Ok(msg) => {
                if socket.send(WsMessage::Text(msg.to_string())).await.is_err() {
                    break;
                }
            },
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}

async fn index(State(state): State<AppState>) -> Response {
    let path = state.ui_dir.join("index.html");
    match std::fs::read_to_string(&path) {
        Ok(html) => {
            let html = html
                .replace("__PRISMATIK_TOKEN__", &state.settings.api_token)
                .replace("__PRISMATIK_VERSION__", VERSION);
            Html(html).into_response()
        },
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Html(
                "<h1>Prismatik</h1><p>UI build not found. Run <code>npm run build</code> in ui/ \
             and set PRISMATIK_UI_DIR, or use the API directly at /api/v1.</p>"
                    .to_string(),
            ),
        )
            .into_response(),
    }
}

pub fn router(state: AppState) -> Router {
    let assets = tower_http::services::ServeDir::new(state.ui_dir.as_ref().clone());
    Router::new()
        .route("/", get(index))
        .route("/api/v1/health", get(health))
        .route("/api/v1/health/detailed", get(health_detailed))
        .route("/api/v1/meta", get(meta))
        .route("/api/v1/config", get(config))
        .route("/api/v1/validate/strategy", post(validate_strategy))
        .route("/api/v1/metrics", get(metrics))
        .route("/api/v1/status", get(status))
        .route("/api/v1/overview", get(overview))
        .route("/api/v1/jobs", get(jobs_list))
        .route("/api/v1/jobs/search", get(jobs_search))
        .route("/api/v1/sessions", get(sessions_list))
        .route("/api/v1/jobs/backtest", post(post_backtest))
        .route("/api/v1/jobs/walkforward", post(post_walkforward))
        .route("/api/v1/jobs/batch/backtest", post(batch_backtest))
        .route("/api/v1/jobs/batch/walkforward", post(batch_walkforward))
        .route("/api/v1/jobs/:id", get(get_job))
        .route("/api/v1/sessions/start", post(session_start))
        .route("/api/v1/sessions/:id", get(session_status))
        .route("/api/v1/sessions/:id/events", get(session_events))
        .route("/api/v1/sessions/:id/stop", post(session_stop))
        .route("/api/v1/sessions/:id/stream", get(session_stream))
        .nest_service("/app", assets)
        .with_state(state)
}
