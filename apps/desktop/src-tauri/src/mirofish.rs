/**
 * MiroFish — Multi-agent social simulation for market prediction.
 *
 * MiroFish spawns thousands of LLM agents with distinct personalities,
 * injects them with market data/news as seed documents, runs a social
 * simulation, and generates prediction reports from emergent behavior.
 *
 * This module provides native integration with MiroFish's Flask REST API.
 *
 * Architecture:
 *   PRISMATIK → seed documents (news, market data, filings)
 *            → MiroFish GraphRAG (entity extraction, knowledge graph)
 *            → Agent swarm simulation (thousands of LLM agents)
 *            → Emergent prediction report
 *            → PRISMATIK prediction pipeline
 */
use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, RwLock};

/// MiroFish server configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MiroFishConfig {
    pub(crate) base_url: String,
    pub(crate) connected: bool,
    pub(crate) version: Option<String>,
}

/// A simulation request sent to MiroFish
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SimulationRequest {
    pub(crate) scenario_id: String,
    pub(crate) seed_documents: Vec<SeedDocument>,
    pub(crate) prediction_query: String,
    pub(crate) agent_count: usize,
    pub(crate) simulation_rounds: usize,
    pub(crate) market_context: MarketContext,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SeedDocument {
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) source: String,
    pub(crate) published_at: Option<String>,
    pub(crate) entities: Vec<String>,
    pub(crate) sentiment: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MarketContext {
    pub(crate) symbols: Vec<MarketSymbol>,
    pub(crate) regime: String,
    pub(crate) recent_events: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MarketSymbol {
    pub(crate) symbol: String,
    pub(crate) price: f64,
    pub(crate) change_pct: f64,
    pub(crate) volume: f64,
}

/// Simulation status from MiroFish
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SimulationStatus {
    pub(crate) scenario_id: String,
    pub(crate) state: String, // building_graph, simulating, generating_report, complete, error
    pub(crate) progress: f64, // 0-1
    pub(crate) agents_active: usize,
    pub(crate) rounds_completed: usize,
    pub(crate) message: String,
}

/// Prediction report from MiroFish simulation
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MiroFishPrediction {
    pub(crate) scenario_id: String,
    pub(crate) generated_at: String,
    pub(crate) summary: String,
    pub(crate) consensus_direction: String, // bullish, bearish, neutral, divergent
    pub(crate) consensus_strength: f64,     // 0-1, how much agents agree
    pub(crate) agent_sentiments: Vec<AgentSentiment>,
    pub(crate) key_arguments: Vec<KeyArgument>,
    pub(crate) emergent_themes: Vec<String>,
    pub(crate) risk_factors: Vec<String>,
    pub(crate) confidence: f64,
    pub(crate) divergence_map: Vec<DivergencePoint>,
    pub(crate) full_report: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentSentiment {
    pub(crate) agent_persona: String,
    pub(crate) direction: String,
    pub(crate) conviction: f64,
    pub(crate) reasoning: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyArgument {
    pub(crate) argument: String,
    pub(crate) supporting_agents: usize,
    pub(crate) opposing_agents: usize,
    pub(crate) strength: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DivergencePoint {
    pub(crate) topic: String,
    pub(crate) bull_pct: f64,
    pub(crate) bear_pct: f64,
    pub(crate) neutral_pct: f64,
}

static MIROFISH_CONFIG: LazyLock<RwLock<MiroFishConfig>> = LazyLock::new(|| {
    RwLock::new(MiroFishConfig {
        base_url: "http://localhost:5001".into(),
        connected: false,
        version: None,
    })
});

static SIMULATION_HISTORY: LazyLock<RwLock<Vec<MiroFishPrediction>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// Configure MiroFish connection
#[tauri::command]
pub(crate) async fn mirofish_connect(base_url: Option<String>) -> Result<String, String> {
    let url = base_url.unwrap_or_else(|| "http://localhost:5001".into());
    let client = reqwest::Client::new();

    // probe MiroFish health
    let resp = client
        .get(format!("{url}/api/graph/health"))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let mut config = MIROFISH_CONFIG.write().map_err(|_| "state unavailable")?;
            config.base_url = url.clone();
            config.connected = true;
            config.version = Some("connected".into());
            Ok(format!("Connected to MiroFish at {url}"))
        },
        _ => {
            // try alternate health endpoint
            let resp2 = client
                .get(format!("{url}/"))
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await;
            match resp2 {
                Ok(r) if r.status().is_success() => {
                    let mut config = MIROFISH_CONFIG.write().map_err(|_| "state unavailable")?;
                    config.base_url = url.clone();
                    config.connected = true;
                    Ok(format!("Connected to MiroFish at {url}"))
                },
                _ => Err(format!(
                    "Cannot reach MiroFish at {url}. Start it with: cd mirofish && python run.py"
                )),
            }
        },
    }
}

/// Get MiroFish connection status
#[tauri::command]
pub(crate) fn mirofish_status() -> Result<MiroFishConfig, String> {
    Ok(MIROFISH_CONFIG
        .read()
        .map_err(|_| "state unavailable")?
        .clone())
}

/// Run a MiroFish simulation with market data as seed documents.
///
/// This is the core integration: PRISMATIK feeds market data, news, and
/// analysis into MiroFish as seed documents. MiroFish spawns thousands
/// of LLM agents with different trading personas, runs the simulation,
/// and returns an emergent prediction.
#[tauri::command]
pub(crate) async fn mirofish_run_simulation(
    symbols: Vec<String>,
    seed_headlines: Vec<String>,
    prediction_query: String,
    agent_count: Option<usize>,
    rounds: Option<usize>,
) -> Result<SimulationStatus, String> {
    let base_url = {
        let config = MIROFISH_CONFIG.read().map_err(|_| "state unavailable")?;
        if !config.connected {
            return Err("MiroFish not connected. Run mirofish_connect first.".into());
        }
        config.base_url.clone()
    };

    let agent_count = agent_count.unwrap_or(1000);
    let rounds = rounds.unwrap_or(10);

    // build seed documents from headlines
    let seed_docs: Vec<SeedDocument> = seed_headlines
        .iter()
        .enumerate()
        .map(|(i, headline)| SeedDocument {
            title: format!("Market Signal {}", i + 1),
            content: headline.clone(),
            source: "PRISMATIK".into(),
            published_at: Some(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            entities: symbols.clone(),
            sentiment: 0.0,
        })
        .collect();

    // build market context
    let market_context = MarketContext {
        symbols: symbols
            .iter()
            .map(|s| MarketSymbol {
                symbol: s.clone(),
                price: 0.0,
                change_pct: 0.0,
                volume: 0.0,
            })
            .collect(),
        regime: "unknown".into(),
        recent_events: seed_headlines.clone(),
    };

    let scenario_id = format!("prismatik_{}", chrono::Utc::now().timestamp());

    // send to MiroFish
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "scenario_id": scenario_id,
        "seed_documents": seed_docs,
        "prediction_query": prediction_query,
        "agent_count": agent_count,
        "simulation_rounds": rounds,
        "market_context": market_context,
    });

    let resp = client
        .post(format!("{base_url}/api/simulation/start"))
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("MiroFish request failed: {e}"))?;

    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        return Err(format!("MiroFish rejected simulation: {err}"));
    }

    Ok(SimulationStatus {
        scenario_id,
        state: "simulating".into(),
        progress: 0.0,
        agents_active: agent_count,
        rounds_completed: 0,
        message: "Simulation started".into(),
    })
}

/// Check simulation status
#[tauri::command]
pub(crate) async fn mirofish_check_status(scenario_id: String) -> Result<SimulationStatus, String> {
    let base_url = {
        let config = MIROFISH_CONFIG.read().map_err(|_| "state unavailable")?;
        if !config.connected {
            return Err("MiroFish not connected".into());
        }
        config.base_url.clone()
    };

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base_url}/api/simulation/status/{scenario_id}"))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("MiroFish returned HTTP {}", resp.status()));
    }

    let status: SimulationStatus = resp.json().await.map_err(|e| format!("parse error: {e}"))?;
    Ok(status)
}

/// Get simulation report/prediction
#[tauri::command]
pub(crate) async fn mirofish_get_prediction(
    scenario_id: String,
) -> Result<MiroFishPrediction, String> {
    let base_url = {
        let config = MIROFISH_CONFIG.read().map_err(|_| "state unavailable")?;
        if !config.connected {
            return Err("MiroFish not connected".into());
        }
        config.base_url.clone()
    };

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base_url}/api/report/{scenario_id}"))
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("MiroFish returned HTTP {}", resp.status()));
    }

    let val: serde_json::Value = resp.json().await.map_err(|e| format!("parse error: {e}"))?;

    // parse MiroFish report into our prediction format
    let prediction = MiroFishPrediction {
        scenario_id: scenario_id.clone(),
        generated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        summary: val["summary"]
            .as_str()
            .unwrap_or("No summary available")
            .into(),
        consensus_direction: val["consensus_direction"]
            .as_str()
            .unwrap_or("neutral")
            .into(),
        consensus_strength: val["consensus_strength"].as_f64().unwrap_or(0.5),
        agent_sentiments: val["agent_sentiments"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|a| AgentSentiment {
                        agent_persona: a["persona"].as_str().unwrap_or("unknown").into(),
                        direction: a["direction"].as_str().unwrap_or("neutral").into(),
                        conviction: a["conviction"].as_f64().unwrap_or(0.5),
                        reasoning: a["reasoning"].as_str().unwrap_or("").into(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        key_arguments: val["key_arguments"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|a| KeyArgument {
                        argument: a["argument"].as_str().unwrap_or("").into(),
                        supporting_agents: a["supporting"].as_u64().unwrap_or(0) as usize,
                        opposing_agents: a["opposing"].as_u64().unwrap_or(0) as usize,
                        strength: a["strength"].as_f64().unwrap_or(0.5),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        emergent_themes: val["themes"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        risk_factors: val["risks"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| r.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        confidence: val["confidence"].as_f64().unwrap_or(0.5),
        divergence_map: val["divergence"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|d| DivergencePoint {
                        topic: d["topic"].as_str().unwrap_or("").into(),
                        bull_pct: d["bull"].as_f64().unwrap_or(0.33),
                        bear_pct: d["bear"].as_f64().unwrap_or(0.33),
                        neutral_pct: d["neutral"].as_f64().unwrap_or(0.34),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        full_report: val["full_report"].as_str().unwrap_or("").into(),
    };

    // store in history
    SIMULATION_HISTORY
        .write()
        .map_err(|_| "state unavailable")?
        .push(prediction.clone());

    Ok(prediction)
}

/// Get simulation history
#[tauri::command]
pub(crate) fn mirofish_history() -> Result<Vec<MiroFishPrediction>, String> {
    Ok(SIMULATION_HISTORY
        .read()
        .map_err(|_| "state unavailable")?
        .clone())
}

/// Build seed documents from current market data + news.
/// This is what PRISMATIK feeds into MiroFish as the "world state."
#[tauri::command]
pub(crate) async fn mirofish_build_seeds(
    symbols: Vec<String>,
) -> Result<Vec<SeedDocument>, String> {
    let mut seeds = Vec::new();

    // get current market data
    let terminal = crate::terminal_feed::get_terminal_feed(crate::app_handle()?).await?;
    for quote in &terminal.quotes {
        if symbols.contains(&quote.symbol) {
            seeds.push(SeedDocument {
                title: format!("{} Market Update", quote.symbol),
                content: format!(
                    "{} is trading at ${:.2}, change: {:.2}%, volume: {:.0}. Provider: {}. Observed: {}.",
                    quote.symbol, quote.price,
                    quote.change_pct.unwrap_or(0.0),
                    quote.volume.unwrap_or(0.0),
                    quote.provider, quote.observed_at
                ),
                source: quote.provider.clone(),
                published_at: Some(quote.observed_at.clone()),
                entities: vec![quote.symbol.clone()],
                sentiment: quote.change_pct.unwrap_or(0.0) * 0.1,
            });
        }
    }

    // get recent predictions
    let preds = crate::predictions::get_predictions()?;
    for pred in preds.predictions.iter().take(10) {
        if symbols.contains(&pred.entity) {
            seeds.push(SeedDocument {
                title: format!("PRISMATIK Prediction: {}", pred.entity),
                content: format!(
                    "Model {} predicts {} for {} with {:.0}% confidence. Interval: [{:.1}%, {:.1}%]. Regime: {}.",
                    pred.model, pred.direction, pred.entity,
                    pred.confidence * 100.0,
                    pred.interval_low * 100.0, pred.interval_high * 100.0,
                    pred.regime
                ),
                source: "PRISMATIK Predictions".into(),
                published_at: Some(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()),
                entities: vec![pred.entity.clone()],
                sentiment: match pred.direction.as_str() {
                    "bullish" => pred.confidence,
                    "bearish" => -pred.confidence,
                    _ => 0.0,
                },
            });
        }
    }

    Ok(seeds)
}
