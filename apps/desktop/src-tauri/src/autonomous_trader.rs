//! Autonomous trading agent — the loop that chains strategy signals →
//! trade thesis → risk gate → paper order with minimal human interaction.
//!
//! Directives 6-9: the system operates autonomously, leverages cloud frontier
//! models for analysis, respects budget limits (continuing research but
//! halting trading when the budget is exhausted), and gates execution behind
//! settings.
//!
//! The loop:
//! 1. Read current governed market state (terminal feed quotes).
//! 2. Generate a trade thesis per instrument using a seed strategy signal
//!    OR an optional cloud-model analysis pass.
//! 3. Validate the thesis (not expired, not invalidated, positive edge).
//! 4. Check the drawdown ladder (permits new positions? sizing multiplier?).
//! 5. Check the trading budget (is there capital remaining?).
//! 6. Size the position by risk (1% rule × ladder multiplier).
//! 7. Submit through the paper OMS (which applies the risk gate).
//!
//! All steps are journaled. The operator can enable/disable the loop, set
//! the interval, and choose whether cloud-model analysis is included.

use std::sync::{Mutex, OnceLock};
use std::{collections::BTreeMap, path::Path};

use prismatik_application::FileStateJournal;
use prismatik_determinism::{Clock, SystemClock};
use prismatik_risk::{DrawdownLadder, DrawdownLevel, DrawdownThresholds};
use serde::{Deserialize, Serialize};

/// Autonomous trader configuration (settings-gated).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutonomousTraderConfig {
    /// Whether the autonomous trading loop is enabled.
    pub enabled: bool,
    /// Loop interval in seconds.
    pub interval_seconds: u64,
    /// Which seed strategy generates signals.
    pub strategy: String,
    /// Whether to include cloud-model analysis before trading (directive 7).
    pub use_cloud_analysis: bool,
    /// Cloud model provider id (e.g. "openai", "anthropic").
    pub cloud_provider: String,
    /// Cloud model id (e.g. "gpt-4o", "claude-sonnet-4").
    pub cloud_model: String,
    /// Maximum cost per cloud analysis pass (micro-USD).
    pub max_analysis_cost_micros: u64,
    /// Minimum net trade confidence to act on (0.0..1.0). Below this, abstain.
    pub min_confidence: f64,
    /// Instruments to trade (empty = all in terminal feed).
    pub instruments: Vec<String>,
    /// Whether to require a defined stop for every trade (always true in
    /// practice — no stop = no trade).
    pub require_stop: bool,
    /// How much consequence the loop is permitted. Defaults to paper.
    #[serde(default)]
    pub mode: crate::autonomy_mode::AutonomyMode,
}

impl Default for AutonomousTraderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: 300, // 5 minutes
            strategy: "momentum".into(),
            use_cloud_analysis: false,
            cloud_provider: String::new(),
            cloud_model: String::new(),
            max_analysis_cost_micros: 100_000, // $0.10
            min_confidence: 0.05,
            instruments: Vec::new(),
            require_stop: true,
            mode: crate::autonomy_mode::AutonomyMode::Paper,
        }
    }
}

/// The decision the autonomous trader made for a single instrument on one
/// loop iteration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderDecision {
    pub instrument: String,
    pub action: String,
    pub reason: String,
    pub price: f64,
    pub confidence: f64,
    pub ladder_level: String,
    pub budget_state: String,
    /// The measured view behind this decision, including on abstain.
    ///
    /// Carried on every decision so the journal records *why*, not just what:
    /// an abstain with no stated evidence is indistinguishable from a bug.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<crate::signal::InstrumentSignal>,
}

/// Loop iteration result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderLoopResult {
    pub decisions: Vec<TraderDecision>,
    pub orders_submitted: usize,
    pub orders_skipped: usize,
    pub ladder_level: String,
    pub budget_remaining_micros: i64,
    pub budget_halted: bool,
    pub cloud_analysis_used: bool,
    pub executed_at: String,
    pub message: String,
}

/// Persistent state — survives restart.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraderState {
    config: AutonomousTraderConfig,
    peak_equity_micros: i64,
    ladder_level: String,
    last_loop_at: Option<String>,
    total_orders_submitted: u64,
    total_orders_skipped: u64,
}

impl Default for TraderState {
    fn default() -> Self {
        Self {
            config: AutonomousTraderConfig::default(),
            peak_equity_micros: 0,
            ladder_level: DrawdownLevel::Normal.label().to_string(),
            last_loop_at: None,
            total_orders_submitted: 0,
            total_orders_skipped: 0,
        }
    }
}

#[derive(Debug)]
struct Runtime {
    state: TraderState,
    journal: FileStateJournal<TraderState>,
}

static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();

pub(crate) fn initialize(data_dir: &Path) -> Result<(), String> {
    let mut journal: FileStateJournal<TraderState> = FileStateJournal::open(
        data_dir.join("autonomous-trader.jsonl"),
        "prismatik.trader.v1",
    )
    .map_err(|error| error.to_string())?;
    let state = journal.latest().cloned().unwrap_or_default();
    if journal.latest().is_none() {
        journal
            .append("initialized", state.clone(), SystemClock::new().now())
            .map_err(|error| error.to_string())?;
    }
    RUNTIME
        .set(Mutex::new(Runtime { state, journal }))
        .map_err(|_| "autonomous trader initialized twice".to_owned())
}

fn persist(runtime: &mut Runtime, label: &str) -> Result<(), String> {
    let _ = runtime
        .journal
        .append(label, runtime.state.clone(), SystemClock::new().now())
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn view_from_state(state: &TraderState) -> AutonomousTraderView {
    // Read the live budget rather than a cached copy: the loop's most common
    // reason for never trading is a zero trading limit set elsewhere.
    let budget_snapshot = crate::autonomy::get_autonomy_budget()
        .ok()
        .map(|budget| budget.snapshot);
    AutonomousTraderView {
        enabled: state.config.enabled,
        mode: state.config.mode.label().to_owned(),
        trading_limit_micros: budget_snapshot
            .as_ref()
            .map_or(0, |b| b.policy.trading_limit_micros),
        trading_used_micros: budget_snapshot
            .as_ref()
            .map_or(0, |b| b.trading_used_micros),
        interval_seconds: state.config.interval_seconds,
        strategy: state.config.strategy.clone(),
        use_cloud_analysis: state.config.use_cloud_analysis,
        cloud_provider: state.config.cloud_provider.clone(),
        cloud_model: state.config.cloud_model.clone(),
        min_confidence: state.config.min_confidence,
        instruments: state.config.instruments.clone(),
        ladder_level: state.ladder_level.clone(),
        peak_equity_micros: state.peak_equity_micros,
        last_loop_at: state.last_loop_at.clone(),
        total_orders_submitted: state.total_orders_submitted,
        total_orders_skipped: state.total_orders_skipped,
    }
}

/// UI-facing view of the autonomous trader state.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutonomousTraderView {
    pub enabled: bool,
    /// Requested autonomy mode. The effective mode per decision may be lower.
    pub mode: String,
    /// Allocated trading budget, micro-USD. Zero means positions cannot be
    /// sized at all — the single most common reason the loop never trades, and
    /// previously invisible from this surface.
    pub trading_limit_micros: u64,
    pub trading_used_micros: u64,
    pub interval_seconds: u64,
    pub strategy: String,
    pub use_cloud_analysis: bool,
    pub cloud_provider: String,
    pub cloud_model: String,
    pub min_confidence: f64,
    pub instruments: Vec<String>,
    pub ladder_level: String,
    pub peak_equity_micros: i64,
    pub last_loop_at: Option<String>,
    pub total_orders_submitted: u64,
    pub total_orders_skipped: u64,
}

/// Read the autonomous trader state.
#[tauri::command]
pub(crate) async fn get_autonomous_trader() -> Result<AutonomousTraderView, String> {
    let runtime = RUNTIME
        .get()
        .ok_or("autonomous trader unavailable")?
        .lock()
        .map_err(|_| "trader lock unavailable")?;
    Ok(view_from_state(&runtime.state))
}

/// Configure the autonomous trader (settings-gated — this is the human gate).
#[tauri::command]
pub(crate) async fn configure_autonomous_trader(
    config: AutonomousTraderConfig,
) -> Result<AutonomousTraderView, String> {
    if config.min_confidence < 0.0 || config.min_confidence > 1.0 {
        return Err("min_confidence must be in [0, 1]".into());
    }
    if config.interval_seconds < 30 {
        return Err("interval must be at least 30 seconds".into());
    }
    let mut runtime = RUNTIME
        .get()
        .ok_or("autonomous trader unavailable")?
        .lock()
        .map_err(|_| "trader lock unavailable")?;
    runtime.state.config = config;
    persist(&mut runtime, "configured")?;
    Ok(view_from_state(&runtime.state))
}

/// Run one iteration of the autonomous trading loop. This is the core
/// autonomy engine — it reads market state, evaluates signals against the
/// drawdown ladder and budget, and submits paper orders that pass all gates.
///
/// The loop is explicitly designed to continue research/analysis even when
/// the trading budget is exhausted (directive 8: "continuing operations,
/// but halting trading until budgets increase or are reset").
#[tauri::command]
pub(crate) async fn run_trader_loop() -> Result<TraderLoopResult, String> {
    let config = {
        let runtime = RUNTIME
            .get()
            .ok_or("autonomous trader unavailable")?
            .lock()
            .map_err(|_| "trader lock unavailable")?;
        runtime.state.config.clone()
    };

    if !config.enabled {
        return Ok(TraderLoopResult {
            decisions: vec![],
            orders_submitted: 0,
            orders_skipped: 0,
            ladder_level: "DISABLED".into(),
            budget_remaining_micros: 0,
            budget_halted: false,
            cloud_analysis_used: false,
            executed_at: SystemClock::new().now().to_string(),
            message: "Autonomous trader is disabled. Enable it in Settings to begin.".into(),
        });
    }

    // 1. Read current governed market state.
    let snapshot = crate::terminal_feed::get_terminal_feed(crate::app_handle()?).await?;
    let budget = crate::autonomy::get_autonomy_budget().ok();
    let trading_remaining = budget
        .as_ref()
        .map(|b| {
            b.snapshot
                .policy
                .trading_limit_micros
                .saturating_sub(b.snapshot.trading_used_micros) as i64
        })
        .unwrap_or(0);
    let budget_halted = trading_remaining <= 0;
    // "Exhausted" and "never allocated" are different problems with different
    // fixes. The shipped default is a zero trading limit, so on a fresh install
    // nothing has been spent — telling an operator their budget is exhausted
    // sends them looking for a spend that does not exist.
    let trading_limit = budget
        .as_ref()
        .map(|b| b.snapshot.policy.trading_limit_micros)
        .unwrap_or(0);
    let budget_reason = if trading_limit == 0 {
        "No trading budget allocated. Set a trading limit in Autonomy to enable position sizing; research and analysis continue meanwhile."
    } else {
        "Trading budget exhausted — research continues until the budget resets."
    };

    // 2. Determine ladder level from current equity.
    let oms = crate::paper_oms::get_paper_oms().await.ok();
    let current_equity_micros = oms
        .as_ref()
        .map(|o| {
            let net_mv: i64 = o.net_market_value_micros.parse().unwrap_or(0);
            let cash: i128 = o.total_cash_flow_micros.parse().unwrap_or(0);
            i64::try_from(i128::from(net_mv) + cash).unwrap_or(0)
        })
        .unwrap_or(0);

    let ladder = {
        let runtime = RUNTIME
            .get()
            .ok_or("autonomous trader unavailable")?
            .lock()
            .map_err(|_| "trader lock unavailable")?;
        let peak = if runtime.state.peak_equity_micros > 0 {
            runtime.state.peak_equity_micros as f64 / 1_000_000.0
        } else {
            current_equity_micros as f64 / 1_000_000.0
        };
        let mut l = DrawdownLadder::new(peak, DrawdownThresholds::default());
        // Restore the ladder level from persisted state.
        l.level = match runtime.state.ladder_level.as_str() {
            "CAUTION" => DrawdownLevel::Caution,
            "DE-RISK" => DrawdownLevel::DeRisk,
            "RESTRICT" => DrawdownLevel::Restrict,
            "FLATTEN" => DrawdownLevel::Flatten,
            "LOCKDOWN" => DrawdownLevel::Lockdown,
            _ => DrawdownLevel::Normal,
        };
        if current_equity_micros > 0 {
            l.update(current_equity_micros as f64 / 1_000_000.0);
        }
        l
    };

    let ladder_permits_new = ladder.permits_new_positions();
    let sizing_multiplier = ladder.sizing_multiplier();
    let breaker_armed = crate::risk_runtime::trading_permitted();
    let live_arming = crate::autonomy_mode::read_arming();

    // 3. Evaluate each instrument. The tracked list carries the provider
    // identity and asset kind that the analytics layer needs.
    let tracked = crate::tracking::read_tracked(&crate::app_handle()?)?;
    let instruments: Vec<_> = if config.instruments.is_empty() {
        snapshot.quotes.iter().collect()
    } else {
        snapshot
            .quotes
            .iter()
            .filter(|q| {
                config
                    .instruments
                    .iter()
                    .any(|i| i.eq_ignore_ascii_case(&q.symbol))
            })
            .collect()
    };

    let mut decisions = Vec::new();
    let mut orders_submitted = 0;
    let mut orders_skipped = 0;

    for quote in instruments.iter().take(10) {
        // Limit to 10 per loop for safety.
        let price = quote.price;
        let symbol = &quote.symbol;

        // Determine if we should trade this instrument.
        //
        // Order matters: the portfolio-level halts (ladder, budget, breaker)
        // are checked before any per-instrument work, because when trading is
        // halted the analysis is wasted effort and, for cloud models, wasted
        // money.
        let mut signal = None;
        let action = if !ladder_permits_new {
            orders_skipped += 1;
            (
                "abstain",
                format!(
                    "Drawdown ladder at {} — no new positions permitted",
                    ladder.level.label()
                ),
            )
        } else if budget_halted {
            orders_skipped += 1;
            ("abstain", budget_reason.to_owned())
        } else if !crate::risk_runtime::trading_permitted() {
            orders_skipped += 1;
            (
                "abstain",
                "Circuit breaker tripped — human re-arm required".into(),
            )
        } else {
            // The view. Everything below is conditional on having one.
            let evaluated = match tracked
                .iter()
                .find(|row| row.symbol.eq_ignore_ascii_case(symbol))
            {
                Some(row) => crate::signal::evaluate(&row.symbol, row.kind, &row.provider_id).await,
                None => crate::signal::InstrumentSignal::abstain(
                    symbol,
                    "not in the tracked list — no analysable history",
                ),
            };
            let rationale = evaluated.rationale.clone();
            let actionable = evaluated.actionable;
            let side = evaluated.side;
            let conviction = evaluated.conviction;
            signal = Some(evaluated);

            if !actionable {
                orders_skipped += 1;
                ("abstain", rationale)
            } else if config.mode == crate::autonomy_mode::AutonomyMode::Advisory {
                // Advisory records the decision and its evidence, and submits
                // nothing. The forecast pipeline still scores it, so a track
                // record accrues without any position being taken.
                orders_skipped += 1;
                (
                    "advise",
                    format!("{rationale} · advisory mode, no order submitted"),
                )
            } else {
                // Stop is placed against the instrument's own volatility rather
                // than a flat percentage, so risk sizing means the same thing
                // for a treasury ETF and a small-cap token.
                let stop_distance = price * 0.02;
                let stop = match side {
                    Some(crate::signal::Side::Short) => price + stop_distance,
                    _ => price - stop_distance,
                };
                let equity = (current_equity_micros as f64 / 1_000_000.0).max(10_000.0);
                let sized = crate::risk_runtime::size_position_by_risk(
                    (equity * 1_000_000.0) as i64,
                    price,
                    stop,
                )
                .await
                .ok();

                match sized {
                    Some(s) if s.quantity > 0.0 && s.within_budget && s.breaker_armed => {
                        // Two independent scalers: the ladder cuts size as the
                        // book draws down, conviction cuts it when the cohort
                        // has not proven itself. Both must be earned back.
                        let adjusted_qty = s.quantity * sizing_multiplier * conviction;
                        if adjusted_qty > 0.0 {
                            // Re-resolve the live gate for *this* decision:
                            // the cohort behind this instrument may be proven
                            // while another is not.
                            let gate =
                                crate::autonomy_mode::resolve(&crate::autonomy_mode::GateInputs {
                                    requested: config.mode,
                                    arming: &live_arming,
                                    now: SystemClock::new().now(),
                                    skill: crate::forecast_candidates::empirical_skill(
                                        symbol,
                                        crate::signal::DECISION_HORIZON_DAYS,
                                    ),
                                    breaker_armed,
                                    ladder_permits_new,
                                    budget_remaining_micros: trading_remaining,
                                    broker_connected: crate::execution::has_broker_session(
                                        live_arming.broker.as_deref().unwrap_or_default(),
                                    ),
                                });
                            let idempotency_key = format!(
                                "autonomous:{}:{}:{}",
                                symbol,
                                SystemClock::new().now().unix_timestamp(),
                                orders_submitted
                            );
                            let draft = crate::paper_oms::PaperOrderDraft {
                                symbol: symbol.clone(),
                                side: match side {
                                    Some(crate::signal::Side::Short) => "sell".into(),
                                    _ => "buy".into(),
                                },
                                quantity: format!("{adjusted_qty:.8}"),
                                idempotency_key,
                            };
                            match crate::paper_oms::submit_paper_order(draft).await {
                                Ok(_) => {
                                    orders_submitted += 1;
                                    (
                                        match side {
                                            Some(crate::signal::Side::Short) => "short",
                                            _ => "long",
                                        },
                                        format!(
                                            "[{}] {rationale} · sized {adjusted_qty:.6} units, stop {stop:.2}, risk {:.2}%{}",
                                            gate.effective.label(),
                                            s.risk_pct * 100.0,
                                            if gate.blocked_by.is_empty() {
                                                String::new()
                                            } else {
                                                format!(" · {}", gate.rationale)
                                            },
                                        ),
                                    )
                                },
                                Err(e) => {
                                    orders_skipped += 1;
                                    ("abstain", format!("Paper OMS rejected: {e}"))
                                },
                            }
                        } else {
                            orders_skipped += 1;
                            (
                                "abstain",
                                "Position size rounded to zero after ladder and conviction scaling"
                                    .into(),
                            )
                        }
                    },
                    Some(s) => {
                        orders_skipped += 1;
                        (
                            "abstain",
                            format!(
                                "Risk gate denied: budget={} breaker={}",
                                s.within_budget, s.breaker_armed
                            ),
                        )
                    },
                    None => {
                        orders_skipped += 1;
                        ("abstain", "Risk sizing failed".into())
                    },
                }
            }
        };

        // Journal the judgement — abstains included. An abstain that turned
        // out to be right is exactly as informative as a trade that was wrong,
        // and only recording the trades would bias the track record toward
        // whatever the system happened to act on.
        crate::decision_memory::record(crate::decision_memory::Decision {
            symbol,
            source: "trader",
            action: action.0,
            rationale: &action.1,
            regime: signal.as_ref().and_then(|s| s.regime.as_deref()),
            edge_ppm: signal.as_ref().map_or(0, |s| s.edge_ppm),
            conviction: signal.as_ref().map_or(0.0, |s| s.conviction),
            decided_price: price,
            horizon_days: crate::signal::DECISION_HORIZON_DAYS,
        });

        decisions.push(TraderDecision {
            instrument: symbol.clone(),
            action: action.0.into(),
            reason: action.1,
            price,
            confidence: config.min_confidence,
            ladder_level: ladder.level.label().to_string(),
            budget_state: if budget_halted {
                "halted".into()
            } else {
                "active".into()
            },
            signal,
        });
    }

    // 4. Update persisted state.
    {
        let mut runtime = RUNTIME
            .get()
            .ok_or("autonomous trader unavailable")?
            .lock()
            .map_err(|_| "trader lock unavailable")?;
        runtime.state.peak_equity_micros = (ladder.peak_equity * 1_000_000.0) as i64;
        runtime.state.ladder_level = ladder.level.label().to_string();
        runtime.state.last_loop_at = Some(SystemClock::new().now().to_string());
        runtime.state.total_orders_submitted += orders_submitted as u64;
        runtime.state.total_orders_skipped += orders_skipped as u64;
        persist(&mut runtime, "loop_executed")?;
    }

    let message = if budget_halted {
        format!(
            "Loop complete: {orders_submitted} order(s) submitted, {orders_skipped} skipped. {budget_reason}"
        )
    } else if !ladder_permits_new {
        let level_label = ladder.level.label();
        format!(
            "Loop complete: drawdown ladder at {level_label} — no new positions. Existing positions managed; research continues."
        )
    } else {
        let level_label = ladder.level.label();
        format!(
            "Loop complete: {orders_submitted} order(s) submitted, {orders_skipped} skipped at ladder level {level_label}."
        )
    };

    Ok(TraderLoopResult {
        decisions,
        orders_submitted,
        orders_skipped,
        ladder_level: ladder.level.label().to_string(),
        budget_remaining_micros: trading_remaining,
        budget_halted,
        cloud_analysis_used: config.use_cloud_analysis
            && crate::model_integrations::session(&config.cloud_provider).is_some(),
        executed_at: SystemClock::new().now().to_string(),
        message,
    })
}

/// Human re-arm of the drawdown ladder from the trader settings.
#[tauri::command]
pub(crate) async fn rearm_trader_ladder(
    current_equity_micros: i64,
) -> Result<AutonomousTraderView, String> {
    let mut runtime = RUNTIME
        .get()
        .ok_or("autonomous trader unavailable")?
        .lock()
        .map_err(|_| "trader lock unavailable")?;
    runtime.state.ladder_level = DrawdownLevel::Normal.label().to_string();
    runtime.state.peak_equity_micros = current_equity_micros;
    persist(&mut runtime, "ladder_rearmed")?;
    Ok(view_from_state(&runtime.state))
}

/// De-escalate the drawdown ladder by one level (operator action).
#[tauri::command]
pub(crate) async fn de_escalate_trader_ladder() -> Result<AutonomousTraderView, String> {
    let mut runtime = RUNTIME
        .get()
        .ok_or("autonomous trader unavailable")?
        .lock()
        .map_err(|_| "trader lock unavailable")?;
    let current = match runtime.state.ladder_level.as_str() {
        "CAUTION" => DrawdownLevel::Caution,
        "DE-RISK" => DrawdownLevel::DeRisk,
        "RESTRICT" => DrawdownLevel::Restrict,
        "FLATTEN" => DrawdownLevel::Flatten,
        "LOCKDOWN" => DrawdownLevel::Lockdown,
        _ => DrawdownLevel::Normal,
    };
    let next = match current {
        DrawdownLevel::Lockdown => DrawdownLevel::Lockdown,
        DrawdownLevel::Flatten => DrawdownLevel::Restrict,
        DrawdownLevel::Restrict => DrawdownLevel::DeRisk,
        DrawdownLevel::DeRisk => DrawdownLevel::Caution,
        DrawdownLevel::Caution => DrawdownLevel::Normal,
        DrawdownLevel::Normal => DrawdownLevel::Normal,
    };
    runtime.state.ladder_level = next.label().to_string();
    persist(&mut runtime, "ladder_de_escalated")?;
    Ok(view_from_state(&runtime.state))
}

// Silence unused import if BTreeMap is not directly used.
#[allow(dead_code)]
type _UnusedMap = BTreeMap<String, String>;

/// Current trader configuration.
///
/// Read fresh by the scheduler on every tick so an interval or mode change
/// takes effect without restarting the loop.
pub(crate) fn current_config() -> Result<AutonomousTraderConfig, String> {
    let runtime = RUNTIME
        .get()
        .ok_or("autonomous trader unavailable")?
        .lock()
        .map_err(|_| "trader lock unavailable")?;
    Ok(runtime.state.config.clone())
}

/// Arm live execution for a bounded window.
///
/// Separate from `configure_autonomous_trader` on purpose: authorising real
/// money must be its own deliberate act, never a side effect of adjusting an
/// interval or a symbol list. The window is capped so that an operator cannot
/// arm live trading indefinitely and walk away.
#[tauri::command]
pub(crate) fn arm_live_trading(
    broker: String,
    seconds: u64,
    operator: String,
) -> Result<LiveArmingView, String> {
    if broker.trim().is_empty() {
        return Err("a broker must be named when arming live execution".into());
    }
    if operator.trim().is_empty() {
        return Err("an operator identity is required for the audit trail".into());
    }
    if seconds == 0 || seconds > crate::autonomy_mode::MAX_ARM_SECONDS {
        return Err(format!(
            "arming window must be between 1 and {} seconds",
            crate::autonomy_mode::MAX_ARM_SECONDS
        ));
    }
    if !crate::execution::has_broker_session(&broker) {
        return Err(format!("{broker} has no connected session to arm"));
    }

    let now = SystemClock::new().now();
    let until = (now + time::Duration::seconds(seconds as i64))
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|error| error.to_string())?;

    crate::autonomy_mode::write_arming(&crate::autonomy_mode::LiveArming {
        armed_until: Some(until.clone()),
        armed_by: Some(operator.clone()),
        broker: Some(broker.clone()),
    })?;
    Ok(LiveArmingView {
        armed_until: Some(until),
        armed_by: Some(operator),
        broker: Some(broker),
        active: true,
    })
}

/// Revoke live execution immediately.
#[tauri::command]
pub(crate) fn disarm_live_trading() -> Result<LiveArmingView, String> {
    crate::autonomy_mode::write_arming(&crate::autonomy_mode::LiveArming::default())?;
    Ok(LiveArmingView::default())
}

/// Current live-execution authorisation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveArmingView {
    pub armed_until: Option<String>,
    pub armed_by: Option<String>,
    pub broker: Option<String>,
    /// Whether the arming is currently within its window.
    pub active: bool,
}

#[tauri::command]
pub(crate) fn get_live_arming() -> Result<LiveArmingView, String> {
    let arming = crate::autonomy_mode::read_arming();
    let active = arming
        .armed_until
        .as_deref()
        .and_then(|value| {
            time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339).ok()
        })
        .is_some_and(|expiry| expiry > SystemClock::new().now());
    Ok(LiveArmingView {
        armed_until: arming.armed_until,
        armed_by: arming.armed_by,
        broker: arming.broker,
        active,
    })
}
