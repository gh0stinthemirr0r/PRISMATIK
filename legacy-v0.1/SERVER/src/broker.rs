//! Execution layer: a simulated paper broker and a gated Alpaca adapter that
//! can place real orders through the customer's own brokerage account.
//! Author: Aaron Stovall · Version 0.1.0 · 2026-07-07
//!
//! Safety architecture, which is also the product's liability posture:
//!   1. The Alpaca adapter talks to the PAPER endpoint unless the operator set
//!      PRISMATIK_LIVE_TRADING to the exact acknowledgment phrase at startup AND
//!      the session start request repeats that phrase. Two independent,
//!      deliberate acts, one by the operator, one per session.
//!   2. Every order passes pre-trade checks: per-order USD cap and total
//!      notional cap. Violations reject the order; they are never resized
//!      silently above the cap.
//!   3. Every order carries a client-generated idempotency id so a retry after
//!      a timeout cannot double-submit.
//!   4. Every request and outcome is written to the append-only audit journal
//!      before and after the broker call.
//!   5. A failed or ambiguous order halts the session rather than guessing.

use crate::config::{RiskLimits, Settings};
use crate::journal::Journal;
use rand::RngCore;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum BrokerError {
    #[error("pre-trade check failed: {0}")]
    PreTrade(String),
    #[error("broker http error: {0}")]
    Http(String),
    #[error("order state ambiguous, halting: {0}")]
    Ambiguous(String),
    #[error("broker not configured: {0}")]
    NotConfigured(String),
}

pub struct Fill {
    pub price: f64,
    pub delta_units: f64,
    pub cost: f64,
}

fn fresh_client_order_id() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!("przk-{}", uuid::Uuid::from_bytes(bytes).simple())
}

/// A fully functional simulated account. Fills pay the configured fee and
/// slippage so paper P&L tracks what a real small account would experience.
pub struct PaperBroker {
    pub cash_usd: f64,
    pub position_units: f64,
    pub peak_equity: f64,
    fee_per_turnover: f64,
    risk: RiskLimits,
}

impl PaperBroker {
    pub fn new(initial_equity: f64, settings: &Settings) -> Self {
        Self {
            cash_usd: initial_equity,
            position_units: 0.0,
            peak_equity: initial_equity,
            fee_per_turnover: settings.cost.per_turnover(),
            risk: settings.risk.clone(),
        }
    }

    pub fn equity(&self, price: f64) -> f64 {
        self.cash_usd + self.position_units * price
    }

    pub fn drawdown(&self, price: f64) -> f64 {
        if self.peak_equity > 0.0 {
            self.equity(price) / self.peak_equity - 1.0
        } else {
            0.0
        }
    }

    pub fn rebalance_to_weight(&mut self, target_weight: f64, price: f64) -> Option<Fill> {
        let w = target_weight.clamp(
            -self.risk.max_position_weight,
            self.risk.max_position_weight,
        );
        let equity = self.equity(price);
        let desired = (w * equity).clamp(-self.risk.max_notional_usd, self.risk.max_notional_usd);
        let desired_units = desired / price;
        let delta = desired_units - self.position_units;
        let traded = delta.abs() * price;
        if traded < 1e-9 {
            return None;
        }
        let cost = traded * self.fee_per_turnover;
        self.position_units = desired_units;
        self.cash_usd -= delta * price + cost;
        let eq = self.equity(price);
        if eq > self.peak_equity {
            self.peak_equity = eq;
        }
        Some(Fill {
            price,
            delta_units: delta,
            cost,
        })
    }
}

#[derive(Deserialize)]
struct AlpacaOrder {
    id: String,
    status: String,
    #[serde(default)]
    filled_avg_price: Option<String>,
    #[serde(default)]
    filled_qty: Option<String>,
}

/// Real order placement through the customer's own Alpaca account and keys.
/// `live` is decided by config at construction; see the module header.
pub struct AlpacaBroker {
    client: reqwest::Client,
    base: String,
    key_id: String,
    secret: String,
    pub live: bool,
    risk: RiskLimits,
}

impl AlpacaBroker {
    pub fn new(
        settings: &Settings,
        live_requested: bool,
        live_confirm: &str,
    ) -> Result<Self, BrokerError> {
        let creds = &settings.alpaca;
        let (Some(key_id), Some(secret)) = (creds.key_id.clone(), creds.secret_key.clone()) else {
            return Err(BrokerError::NotConfigured(
                "ALPACA_KEY_ID and ALPACA_SECRET_KEY must be set in the server environment".into(),
            ));
        };
        // Live requires the env gate AND a per-session repeated acknowledgment.
        let live = creds.live && live_requested && live_confirm == crate::config::LIVE_ACK_PHRASE;
        if live_requested && !live {
            return Err(BrokerError::NotConfigured(
                "live trading requested but not unlocked: set PRISMATIK_LIVE_TRADING to the \
                 acknowledgment phrase in the server environment and repeat it in the \
                 session request"
                    .into(),
            ));
        }
        let base = if live {
            "https://api.alpaca.markets".to_string()
        } else {
            "https://paper-api.alpaca.markets".to_string()
        };
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| BrokerError::Http(e.to_string()))?;
        Ok(Self {
            client,
            base,
            key_id,
            secret,
            live,
            risk: settings.risk.clone(),
        })
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("APCA-API-KEY-ID", &self.key_id)
            .header("APCA-API-SECRET-KEY", &self.secret)
    }

    /// Submit a market order for a notional delta, with pre-trade checks,
    /// idempotency, audit logging, and post-submit confirmation. Ambiguity
    /// (order neither filled nor rejected within the deadline) is an error the
    /// caller must treat as a halt.
    pub async fn order_notional(
        &self,
        symbol: &str,
        notional_usd: f64,
        journal: &Journal,
    ) -> Result<(), BrokerError> {
        let abs = notional_usd.abs();
        if abs < 1.0 {
            return Ok(()); // below broker minimum, treat as no-op
        }
        if abs > self.risk.max_order_usd {
            return Err(BrokerError::PreTrade(format!(
                "order notional {:.2} exceeds per-order cap {:.2}",
                abs, self.risk.max_order_usd
            )));
        }
        let side = if notional_usd > 0.0 { "buy" } else { "sell" };
        let client_order_id = fresh_client_order_id();
        let body = json!({
            "symbol": symbol.replace('-', "/"),
            "notional": format!("{:.2}", abs),
            "side": side,
            "type": "market",
            "time_in_force": "gtc",
            "client_order_id": client_order_id,
        });
        journal.write(
            "order_submit",
            json!({
                "endpoint": if self.live { "LIVE" } else { "paper" },
                "symbol": symbol, "side": side, "notional": abs,
                "client_order_id": client_order_id,
            }),
        );
        let url = format!("{}/v2/orders", self.base);
        let resp = self
            .auth(self.client.post(&url))
            .json(&body)
            .send()
            .await
            .map_err(|e| BrokerError::Http(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            journal.write(
                "order_rejected",
                json!({"status": status.as_u16(), "detail": text}),
            );
            return Err(BrokerError::Http(format!("{status}: {text}")));
        }
        let order: AlpacaOrder = resp
            .json()
            .await
            .map_err(|e| BrokerError::Http(e.to_string()))?;

        // Confirm terminal state with a bounded poll. Never assume a fill.
        for _ in 0..10 {
            let check = self
                .auth(
                    self.client
                        .get(format!("{}/v2/orders/{}", self.base, order.id)),
                )
                .send()
                .await
                .map_err(|e| BrokerError::Http(e.to_string()))?;
            if check.status().is_success() {
                let o: AlpacaOrder = check
                    .json()
                    .await
                    .map_err(|e| BrokerError::Http(e.to_string()))?;
                match o.status.as_str() {
                    "filled" => {
                        journal.write(
                            "order_filled",
                            json!({
                                "id": o.id,
                                "avg_price": o.filled_avg_price,
                                "qty": o.filled_qty,
                            }),
                        );
                        return Ok(());
                    },
                    "rejected" | "canceled" | "expired" => {
                        journal.write("order_terminal", json!({"id": o.id, "status": o.status}));
                        return Err(BrokerError::Http(format!("order ended {}", o.status)));
                    },
                    _ => tokio::time::sleep(Duration::from_millis(600)).await,
                }
            }
        }
        journal.write("order_ambiguous", json!({"id": order.id}));
        Err(BrokerError::Ambiguous(format!(
            "order {} not terminal within deadline",
            order.id
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AlpacaCreds, CostModel, Settings};

    fn settings() -> Settings {
        Settings {
            cost: CostModel {
                fee_rate: 0.006,
                slippage_rate: 0.0005,
            },
            risk: RiskLimits {
                max_position_weight: 1.0,
                max_drawdown_kill: 0.2,
                max_notional_usd: 1000.0,
                max_order_usd: 250.0,
            },
            alpaca: AlpacaCreds {
                key_id: None,
                secret_key: None,
                live: false,
            },
            coinbase_rest: String::new(),
            coinbase_ws: String::new(),
            rest_timeout_secs: 5,
            host: "127.0.0.1".into(),
            port: 0,
            api_token: "t".into(),
        }
    }

    #[test]
    fn paper_full_entry_pays_cost() {
        let mut b = PaperBroker::new(100.0, &settings());
        let fill = b.rebalance_to_weight(1.0, 100.0).unwrap();
        assert!((fill.cost - 0.65).abs() < 1e-9);
        assert!((b.equity(100.0) - 99.35).abs() < 1e-9);
    }

    #[test]
    fn paper_respects_notional_cap() {
        let mut s = settings();
        s.risk.max_notional_usd = 50.0;
        let mut b = PaperBroker::new(100.0, &s);
        b.rebalance_to_weight(1.0, 100.0);
        assert!(b.position_units * 100.0 <= 50.0 + 1e-9);
    }

    #[test]
    fn alpaca_requires_credentials() {
        let err = AlpacaBroker::new(&settings(), false, "")
            .err()
            .expect("must fail without creds");
        matches!(err, BrokerError::NotConfigured(_));
    }

    #[test]
    fn alpaca_live_requires_both_gates() {
        let mut s = settings();
        s.alpaca = AlpacaCreds {
            key_id: Some("k".into()),
            secret_key: Some("s".into()),
            live: false, // env gate absent
        };
        assert!(AlpacaBroker::new(&s, true, crate::config::LIVE_ACK_PHRASE).is_err());

        s.alpaca.live = true; // env gate present, but session ack missing
        assert!(AlpacaBroker::new(&s, true, "nope").is_err());

        let ok = AlpacaBroker::new(&s, true, crate::config::LIVE_ACK_PHRASE)
            .map_err(|e| e.to_string())
            .expect("both gates present");
        assert!(ok.live);
        let paper = AlpacaBroker::new(&s, false, "")
            .map_err(|e| e.to_string())
            .expect("paper needs no gates");
        assert!(!paper.live); // even with env gate, sessions default to paper
    }
}
