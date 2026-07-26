//! Prismatik configuration.
//! Author: Aaron Stovall · Version 0.1.0 · 2026-07-07
//!
//! Every setting is injectable from the environment. No secrets in source, no
//! secrets in logs. Trading defaults to the paper endpoint; the live endpoint
//! requires both credentials and an explicit typed acknowledgment, because a
//! commercial product must make the dangerous path deliberate.

use serde::Serialize;
use std::env;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const LIVE_ACK_PHRASE: &str = "I_ACCEPT_FULL_RESPONSIBILITY_FOR_LIVE_TRADING";

#[derive(Clone, Debug, Serialize)]
pub struct CostModel {
    /// Taker fee as a fraction of traded notional.
    pub fee_rate: f64,
    /// Modeled adverse fill as a fraction of traded notional.
    pub slippage_rate: f64,
}

impl CostModel {
    pub fn per_turnover(&self) -> f64 {
        self.fee_rate + self.slippage_rate
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RiskLimits {
    /// Never above fully invested.
    pub max_position_weight: f64,
    /// Halt a session at this drawdown.
    pub max_drawdown_kill: f64,
    /// Absolute exposure cap in USD.
    pub max_notional_usd: f64,
    /// Per-order cap in USD, the last line before any broker call.
    pub max_order_usd: f64,
}

impl RiskLimits {
    pub fn validate(&self) -> Result<(), String> {
        if !(self.max_position_weight > 0.0 && self.max_position_weight <= 1.0) {
            return Err("max_position_weight must be in (0, 1]".into());
        }
        if !(self.max_drawdown_kill > 0.0 && self.max_drawdown_kill < 1.0) {
            return Err("max_drawdown_kill must be in (0, 1)".into());
        }
        if self.max_notional_usd <= 0.0 || self.max_order_usd <= 0.0 {
            return Err("notional caps must be positive".into());
        }
        Ok(())
    }
}

/// Alpaca credentials and endpoint selection. `live` can only become true when
/// the operator sets PRISMATIK_LIVE_TRADING to the exact acknowledgment phrase
/// AND provides credentials. Everything else resolves to the paper endpoint.
#[derive(Clone)]
pub struct AlpacaCreds {
    pub key_id: Option<String>,
    pub secret_key: Option<String>,
    pub live: bool,
}

impl AlpacaCreds {
    pub fn from_env() -> Self {
        let ack = env::var("PRISMATIK_LIVE_TRADING").unwrap_or_default();
        let key_id = env::var("ALPACA_KEY_ID").ok().filter(|s| !s.is_empty());
        let secret_key = env::var("ALPACA_SECRET_KEY").ok().filter(|s| !s.is_empty());
        let live = ack == LIVE_ACK_PHRASE && key_id.is_some() && secret_key.is_some();
        Self { key_id, secret_key, live }
    }

    pub fn configured(&self) -> bool {
        self.key_id.is_some() && self.secret_key.is_some()
    }

    pub fn trading_base(&self) -> &'static str {
        if self.live {
            "https://api.alpaca.markets"
        } else {
            "https://paper-api.alpaca.markets"
        }
    }

    pub fn redacted_key(&self) -> String {
        match &self.key_id {
            Some(k) if k.len() > 4 => format!("{}...", &k[..4]),
            Some(_) => "***".into(),
            None => "unset".into(),
        }
    }
}

fn env_f64(name: &str, default: f64) -> f64 {
    env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[derive(Clone)]
pub struct Settings {
    pub cost: CostModel,
    pub risk: RiskLimits,
    pub alpaca: AlpacaCreds,
    pub coinbase_rest: String,
    pub coinbase_ws: String,
    pub rest_timeout_secs: u64,
    pub host: String,
    pub port: u16,
    pub api_token: String,
}

impl Settings {
    pub fn from_env() -> Result<Self, String> {
        let risk = RiskLimits {
            max_position_weight: env_f64("PRISMATIK_MAX_WEIGHT", 1.0),
            max_drawdown_kill: env_f64("PRISMATIK_MAX_DD_KILL", 0.20),
            max_notional_usd: env_f64("PRISMATIK_MAX_NOTIONAL", 1000.0),
            max_order_usd: env_f64("PRISMATIK_MAX_ORDER", 250.0),
        };
        risk.validate()?;
        let api_token = env::var("PRISMATIK_API_TOKEN").ok().filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                use rand::Rng;
                let bytes: Vec<u8> = rand::thread_rng().sample_iter(rand::distributions::Alphanumeric).take(32).collect();
                String::from_utf8(bytes).expect("alphanumeric is utf8")
            });
        Ok(Self {
            cost: CostModel {
                fee_rate: env_f64("PRISMATIK_FEE_RATE", 0.006),
                slippage_rate: env_f64("PRISMATIK_SLIPPAGE_RATE", 0.0005),
            },
            risk,
            alpaca: AlpacaCreds::from_env(),
            coinbase_rest: env::var("PRISMATIK_COINBASE_REST")
                .unwrap_or_else(|_| "https://api.exchange.coinbase.com".into()),
            coinbase_ws: env::var("PRISMATIK_COINBASE_WS")
                .unwrap_or_else(|_| "wss://ws-feed.exchange.coinbase.com".into()),
            rest_timeout_secs: 10,
            host: env::var("PRISMATIK_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: env::var("PRISMATIK_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8787),
            api_token,
        })
    }

    pub fn periods_per_year(granularity_s: u32) -> f64 {
        (365.0 * 24.0 * 3600.0) / granularity_s as f64
    }
}

pub const VALID_GRANULARITIES: [u32; 6] = [60, 300, 900, 3600, 21600, 86400];
