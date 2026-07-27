//! Live session engine. Author: Aaron Stovall · Version 0.1.0 · 2026-07-07
//!
//! A session connects to the real Coinbase WebSocket ticker, aggregates trades
//! into bars, runs the strategy at each bar close, and routes execution to the
//! selected broker: the internal paper simulator (default), or the Alpaca
//! adapter, which itself defaults to Alpaca's paper endpoint and reaches the
//! live endpoint only through the double gate described in broker.rs. The
//! drawdown kill switch halts the session, and for the Alpaca route any
//! ambiguous order state also halts rather than guessing. Reconnects use
//! exponential backoff. Every event is journaled and broadcast to subscribers.

use crate::broker::{AlpacaBroker, PaperBroker};
use crate::config::Settings;
use crate::journal::Journal;
use crate::strategy::{apply_vol_target, Strategy, StrategySpec};
use crate::types::Candle;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

pub enum Execution {
    Paper(PaperBroker),
    Alpaca {
        broker: AlpacaBroker,
        tracked_units: f64,
        entry_equity: f64,
        peak_equity: f64,
    },
}

pub struct SessionConfig {
    pub symbol: String,
    pub granularity_s: u32,
    pub spec: StrategySpec,
    pub initial_equity: f64,
    pub window_bars: usize,
}

pub struct Session {
    pub id: String,
    pub cfg: SessionConfig,
    pub stop: Arc<AtomicBool>,
    pub journal: Arc<Journal>,
    pub events: broadcast::Sender<serde_json::Value>,
    pub state: Arc<tokio::sync::Mutex<SessionState>>,
}

pub struct SessionState {
    pub execution: Execution,
    pub last_price: Option<f64>,
    pub halted: bool,
    pub live: bool,
    pub strategy_name: String,
}

impl Session {
    pub fn emit(&self, event: &str, fields: serde_json::Value) {
        self.journal.write(event, fields.clone());
        let mut msg = json!({"event": event});
        if let (Some(dst), Some(src)) = (msg.as_object_mut(), fields.as_object()) {
            for (k, v) in src {
                dst.insert(k.clone(), v.clone());
            }
        }
        let _ = self.events.send(msg);
    }
}

pub async fn run_session(session: Arc<Session>, settings: Settings, strategy: Strategy) {
    let live = { session.state.lock().await.live };
    session.emit(
        "session_start",
        json!({
            "symbol": session.cfg.symbol,
            "strategy": strategy.name(),
            "granularity_s": session.cfg.granularity_s,
            "initial_equity": session.cfg.initial_equity,
            "execution": if live { "alpaca_LIVE" } else { "paper" },
        }),
    );

    let mut bars: VecDeque<Candle> = VecDeque::with_capacity(session.cfg.window_bars);
    let mut cur: Option<Candle> = None;
    let mut backoff = 1u64;

    while !session.stop.load(Ordering::Relaxed) {
        match stream_ticks(&session, &settings, &strategy, &mut bars, &mut cur).await {
            Ok(()) => break, // stopped cleanly
            Err(e) => {
                if session.stop.load(Ordering::Relaxed) {
                    break;
                }
                tracing::warn!(error = %e, backoff, "feed_error");
                session.emit("feed_error", json!({"error": e, "reconnect_in_s": backoff}));
                tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                backoff = (backoff * 2).min(60);
            },
        }
    }

    let final_equity = {
        let st = session.state.lock().await;
        let price = st.last_price.unwrap_or(0.0);
        match &st.execution {
            Execution::Paper(b) => b.equity(price),
            Execution::Alpaca {
                tracked_units,
                entry_equity,
                ..
            } => entry_equity + tracked_units * price,
        }
    };
    session.emit("session_stop", json!({"final_equity": final_equity}));
}

async fn stream_ticks(
    session: &Arc<Session>,
    settings: &Settings,
    strategy: &Strategy,
    bars: &mut VecDeque<Candle>,
    cur: &mut Option<Candle>,
) -> Result<(), String> {
    let (mut ws, _) = tokio_tungstenite::connect_async(&settings.coinbase_ws)
        .await
        .map_err(|e| e.to_string())?;
    let sub = json!({
        "type": "subscribe",
        "product_ids": [session.cfg.symbol],
        "channels": ["ticker"],
    });
    ws.send(Message::Text(sub.to_string()))
        .await
        .map_err(|e| e.to_string())?;
    tracing::info!(symbol = %session.cfg.symbol, "ws_connected");

    loop {
        if session.stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let msg = tokio::time::timeout(std::time::Duration::from_secs(60), ws.next())
            .await
            .map_err(|_| "ws receive timeout".to_string())?;
        let Some(msg) = msg else {
            return Err("ws closed".into());
        };
        let msg = msg.map_err(|e| e.to_string())?;
        let Message::Text(text) = msg else { continue };
        let v: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("type").and_then(|t| t.as_str()) != Some("ticker") {
            continue;
        }
        let Some(price) = v
            .get("price")
            .and_then(|p| p.as_str())
            .and_then(|p| p.parse::<f64>().ok())
        else {
            continue;
        };

        let now = Utc::now();
        let bucket = now.timestamp() / session.cfg.granularity_s as i64;
        let closed_bar = match cur {
            Some(bar) if bar.time.timestamp() / session.cfg.granularity_s as i64 == bucket => {
                bar.high = bar.high.max(price);
                bar.low = bar.low.min(price);
                bar.close = price;
                None
            },
            _ => {
                let prev = cur.take();
                *cur = Some(Candle {
                    time: now,
                    open: price,
                    high: price,
                    low: price,
                    close: price,
                    volume: 0.0,
                });
                prev
            },
        };

        if let Some(bar) = closed_bar {
            bars.push_back(bar);
            while bars.len() > session.cfg.window_bars {
                bars.pop_front();
            }
            on_bar_close(session, strategy, bars, bar.close).await?;
        }
        {
            session.state.lock().await.last_price = Some(price);
        }
    }
}

async fn on_bar_close(
    session: &Arc<Session>,
    strategy: &Strategy,
    bars: &VecDeque<Candle>,
    price: f64,
) -> Result<(), String> {
    let mut st = session.state.lock().await;
    let equity = match &st.execution {
        Execution::Paper(b) => b.equity(price),
        Execution::Alpaca {
            tracked_units,
            entry_equity,
            ..
        } => entry_equity + tracked_units * price,
    };
    session.emit("bar_close", json!({"price": price, "equity": equity}));

    // Kill switch, identical for paper and live.
    let (dd, kill_limit) = match &mut st.execution {
        Execution::Paper(b) => (b.drawdown(price), session_kill_limit()),
        Execution::Alpaca {
            tracked_units,
            entry_equity,
            peak_equity,
            ..
        } => {
            let eq = *entry_equity + *tracked_units * price;
            if eq > *peak_equity {
                *peak_equity = eq;
            }
            (
                if *peak_equity > 0.0 {
                    eq / *peak_equity - 1.0
                } else {
                    0.0
                },
                session_kill_limit(),
            )
        },
    };
    if dd <= -kill_limit {
        st.halted = true;
        session.stop.store(true, Ordering::Relaxed);
        drop(st);
        session.emit("kill_switch", json!({"drawdown": dd, "limit": -kill_limit}));
        return Ok(());
    }

    if bars.len() < session.cfg.window_bars / 2 {
        return Ok(());
    }

    let window: Vec<Candle> = bars.iter().copied().collect();
    let mut weights = strategy.target_weights(&window);
    if let Some(vt) = session.cfg.spec.vol_target {
        apply_vol_target(&mut weights, &window, vt, 48, 8760.0, 1.0);
    }
    let target = *weights.last().unwrap_or(&0.0);

    match &mut st.execution {
        Execution::Paper(b) => {
            if let Some(fill) = b.rebalance_to_weight(target, price) {
                let payload = json!({
                    "price": fill.price, "delta_units": fill.delta_units,
                    "cost": fill.cost, "target_weight": target,
                    "equity": b.equity(price), "cash_usd": b.cash_usd,
                    "position_units": b.position_units,
                });
                drop(st);
                session.emit("fill", payload);
            }
        },
        Execution::Alpaca {
            broker,
            tracked_units,
            entry_equity,
            ..
        } => {
            let equity = *entry_equity + *tracked_units * price;
            let desired_units = target * equity / price;
            let delta_units = desired_units - *tracked_units;
            let notional = delta_units * price;
            if notional.abs() >= 1.0 {
                let journal = session.journal.clone();
                match broker
                    .order_notional(&session.cfg.symbol, notional, &journal)
                    .await
                {
                    Ok(()) => {
                        *tracked_units = desired_units;
                        let payload = json!({
                            "price": price, "delta_units": delta_units,
                            "target_weight": target, "position_units": desired_units,
                        });
                        drop(st);
                        session.emit("fill", payload);
                    },
                    Err(e) => {
                        st.halted = true;
                        session.stop.store(true, Ordering::Relaxed);
                        drop(st);
                        session.emit("broker_halt", json!({"error": e.to_string()}));
                        return Ok(());
                    },
                }
            }
        },
    }
    Ok(())
}

fn session_kill_limit() -> f64 {
    std::env::var("PRISMATIK_MAX_DD_KILL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.20)
}
