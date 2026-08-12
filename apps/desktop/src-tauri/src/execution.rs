use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{LazyLock, RwLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionOrder {
    pub(crate) id: String,
    pub(crate) symbol: String,
    pub(crate) side: String,
    pub(crate) qty: f64,
    pub(crate) order_type: String,
    pub(crate) limit_price: Option<f64>,
    pub(crate) status: String,
    pub(crate) filled_qty: f64,
    pub(crate) filled_avg_price: Option<f64>,
    pub(crate) submitted_at: String,
    pub(crate) filled_at: Option<String>,
    pub(crate) broker: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BrokerAccount {
    pub(crate) broker: String,
    pub(crate) account_id: String,
    pub(crate) buying_power: f64,
    pub(crate) cash: f64,
    pub(crate) portfolio_value: f64,
    pub(crate) positions: Vec<Position>,
    pub(crate) mode: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Position {
    pub(crate) symbol: String,
    pub(crate) qty: f64,
    pub(crate) avg_entry: f64,
    pub(crate) current_price: f64,
    pub(crate) market_value: f64,
    pub(crate) unrealized_pl: f64,
    pub(crate) unrealized_plpc: f64,
    pub(crate) side: String,
}

static BROKER_SESSIONS: LazyLock<RwLock<BTreeMap<String, BrokerSession>>> =
    LazyLock::new(|| RwLock::new(BTreeMap::new()));

/// Whether a named broker currently has a connected session.
///
/// The live-execution gate consults this on every decision: an arming that
/// names a broker means nothing if that broker's session has since dropped.
pub(crate) fn has_broker_session(broker: &str) -> bool {
    if broker.is_empty() {
        return false;
    }
    BROKER_SESSIONS
        .read()
        .map(|sessions| sessions.contains_key(broker))
        .unwrap_or(false)
}

#[derive(Clone)]
enum BrokerSession {
    Alpaca {
        api_key: String,
        secret: String,
        base_url: String,
        paper: bool,
    },
    Coinbase {
        api_key: String,
        secret: String,
        passphrase: String,
    },
    Kraken {
        api_key: String,
        secret: String,
    },
}

#[tauri::command]
pub(crate) async fn connect_broker(
    broker: String,
    credentials: BTreeMap<String, String>,
    paper: Option<bool>,
) -> Result<String, String> {
    let paper = paper.unwrap_or(true);
    let session = match broker.as_str() {
        "alpaca" => {
            let api_key = credentials
                .get("apiKey")
                .ok_or("Alpaca requires apiKey")?
                .clone();
            let secret = credentials
                .get("secret")
                .ok_or("Alpaca requires secret")?
                .clone();
            let base_url = if paper {
                "https://paper-api.alpaca.markets".to_string()
            } else {
                "https://api.alpaca.markets".to_string()
            };
            BrokerSession::Alpaca {
                api_key,
                secret,
                base_url,
                paper,
            }
        },
        "coinbase" => {
            let api_key = credentials
                .get("apiKey")
                .ok_or("Coinbase requires apiKey")?
                .clone();
            let secret = credentials
                .get("secret")
                .ok_or("Coinbase requires secret")?
                .clone();
            let passphrase = credentials
                .get("passphrase")
                .ok_or("Coinbase requires passphrase")?
                .clone();
            BrokerSession::Coinbase {
                api_key,
                secret,
                passphrase,
            }
        },
        "kraken" => {
            let api_key = credentials
                .get("apiKey")
                .ok_or("Kraken requires apiKey")?
                .clone();
            let secret = credentials
                .get("secret")
                .ok_or("Kraken requires secret")?
                .clone();
            BrokerSession::Kraken { api_key, secret }
        },
        _ => {
            return Err(format!(
                "Unsupported broker: {broker}. Supported: alpaca, coinbase, kraken"
            ))
        },
    };

    BROKER_SESSIONS
        .write()
        .map_err(|_| "state unavailable")?
        .insert(broker.clone(), session);
    Ok(format!(
        "Connected to {broker} ({})",
        if paper { "paper" } else { "LIVE" }
    ))
}

#[tauri::command]
pub(crate) async fn get_broker_account(broker: String) -> Result<BrokerAccount, String> {
    let (api_key, secret, base_url, paper) = {
        let sessions = BROKER_SESSIONS.read().map_err(|_| "state unavailable")?;
        match sessions.get(&broker) {
            Some(BrokerSession::Alpaca {
                api_key,
                secret,
                base_url,
                paper,
            }) => (api_key.clone(), secret.clone(), base_url.clone(), *paper),
            _ => return Err(format!("Account query not yet implemented for {broker}")),
        }
    };

    // Alpaca account query
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base_url}/v2/account"))
        .header("APCA-API-KEY-ID", &api_key)
        .header("APCA-API-SECRET-KEY", &secret)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Alpaca returned HTTP {}", resp.status()));
    }

    let val: serde_json::Value = resp.json().await.map_err(|e| format!("parse error: {e}"))?;

    // get positions
    let pos_resp = client
        .get(format!("{base_url}/v2/positions"))
        .header("APCA-API-KEY-ID", &api_key)
        .header("APCA-API-SECRET-KEY", &secret)
        .send()
        .await
        .map_err(|e| format!("positions request failed: {e}"))?;

    let positions: Vec<Position> = if pos_resp.status().is_success() {
        let pos_val: serde_json::Value = pos_resp.json().await.unwrap_or_default();
        pos_val
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|p| Position {
                        symbol: p["symbol"].as_str().unwrap_or("").into(),
                        qty: p["qty"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
                        avg_entry: p["avg_entry_price"]
                            .as_str()
                            .unwrap_or("0")
                            .parse()
                            .unwrap_or(0.0),
                        current_price: p["current_price"]
                            .as_str()
                            .unwrap_or("0")
                            .parse()
                            .unwrap_or(0.0),
                        market_value: p["market_value"]
                            .as_str()
                            .unwrap_or("0")
                            .parse()
                            .unwrap_or(0.0),
                        unrealized_pl: p["unrealized_pl"]
                            .as_str()
                            .unwrap_or("0")
                            .parse()
                            .unwrap_or(0.0),
                        unrealized_plpc: p["unrealized_plpc"]
                            .as_str()
                            .unwrap_or("0")
                            .parse()
                            .unwrap_or(0.0),
                        side: p["side"].as_str().unwrap_or("long").into(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        vec![]
    };

    Ok(BrokerAccount {
        broker: "alpaca".into(),
        account_id: val["id"].as_str().unwrap_or("").into(),
        buying_power: val["buying_power"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0.0),
        cash: val["cash"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
        portfolio_value: val["portfolio_value"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0.0),
        positions,
        mode: if paper { "paper".into() } else { "live".into() },
    })
}

#[tauri::command]
pub(crate) async fn submit_order(
    broker: String,
    symbol: String,
    side: String,
    qty: f64,
    order_type: Option<String>,
    limit_price: Option<f64>,
) -> Result<ExecutionOrder, String> {
    let (api_key, secret, base_url) = {
        let sessions = BROKER_SESSIONS.read().map_err(|_| "state unavailable")?;
        match sessions.get(&broker) {
            Some(BrokerSession::Alpaca {
                api_key,
                secret,
                base_url,
                ..
            }) => (api_key.clone(), secret.clone(), base_url.clone()),
            _ => return Err(format!("Order submission not yet implemented for {broker}")),
        }
    };

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "symbol": symbol,
        "qty": qty.to_string(),
        "side": side,
        "type": order_type.as_deref().unwrap_or("market"),
        "time_in_force": "day",
        "limit_price": limit_price.map(|p| p.to_string()),
    });

    let resp = client
        .post(format!("{base_url}/v2/orders"))
        .header("APCA-API-KEY-ID", &api_key)
        .header("APCA-API-SECRET-KEY", &secret)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("order failed: {e}"))?;

    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        return Err(format!("Alpaca rejected order: {err}"));
    }

    let val: serde_json::Value = resp.json().await.map_err(|e| format!("parse error: {e}"))?;

    Ok(ExecutionOrder {
        id: val["id"].as_str().unwrap_or("").into(),
        symbol: val["symbol"].as_str().unwrap_or(&symbol).into(),
        side: val["side"].as_str().unwrap_or(&side).into(),
        qty: val["qty"].as_str().unwrap_or("0").parse().unwrap_or(qty),
        order_type: val["type"].as_str().unwrap_or("market").into(),
        limit_price: val["limit_price"].as_str().and_then(|p| p.parse().ok()),
        status: val["status"].as_str().unwrap_or("submitted").into(),
        filled_qty: val["filled_qty"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0.0),
        filled_avg_price: val["filled_avg_price"]
            .as_str()
            .and_then(|p| p.parse().ok()),
        submitted_at: val["submitted_at"].as_str().unwrap_or("").into(),
        filled_at: val["filled_at"].as_str().map(String::from),
        broker: "alpaca".into(),
    })
}

#[tauri::command]
pub(crate) async fn list_orders(
    broker: String,
    status: Option<String>,
) -> Result<Vec<ExecutionOrder>, String> {
    let (api_key, secret, base_url) = {
        let sessions = BROKER_SESSIONS.read().map_err(|_| "state unavailable")?;
        match sessions.get(&broker) {
            Some(BrokerSession::Alpaca {
                api_key,
                secret,
                base_url,
                ..
            }) => (api_key.clone(), secret.clone(), base_url.clone()),
            _ => return Err(format!("Order listing not yet implemented for {broker}")),
        }
    };

    let client = reqwest::Client::new();
    let status_filter = status.unwrap_or_else(|| "all".into());
    let resp = client
        .get(format!(
            "{base_url}/v2/orders?status={status_filter}&limit=50"
        ))
        .header("APCA-API-KEY-ID", &api_key)
        .header("APCA-API-SECRET-KEY", &secret)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Alpaca returned HTTP {}", resp.status()));
    }

    let val: serde_json::Value = resp.json().await.map_err(|e| format!("parse error: {e}"))?;

    Ok(val
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|o| ExecutionOrder {
                    id: o["id"].as_str().unwrap_or("").into(),
                    symbol: o["symbol"].as_str().unwrap_or("").into(),
                    side: o["side"].as_str().unwrap_or("").into(),
                    qty: o["qty"].as_str().unwrap_or("0").parse().unwrap_or(0.0),
                    order_type: o["type"].as_str().unwrap_or("").into(),
                    limit_price: o["limit_price"].as_str().and_then(|p| p.parse().ok()),
                    status: o["status"].as_str().unwrap_or("").into(),
                    filled_qty: o["filled_qty"]
                        .as_str()
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0.0),
                    filled_avg_price: o["filled_avg_price"].as_str().and_then(|p| p.parse().ok()),
                    submitted_at: o["submitted_at"].as_str().unwrap_or("").into(),
                    filled_at: o["filled_at"].as_str().map(String::from),
                    broker: "alpaca".into(),
                })
                .collect()
        })
        .unwrap_or_default())
}
