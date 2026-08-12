//! Crowd — the population estimator.
//!
//! Crowd is the desk's third and least conventional forecaster. Where the
//! regime model reasons from history and Tape reasons from sequence, Crowd
//! reasons from *people*: it seeds a population of LLM agents, each with a
//! distinct disposition, with the same documents a human desk would read, lets
//! them talk to one another, and reads the prediction out of what the
//! population converges on.
//!
//! That makes its dispersion as interesting as its answer. A population that
//! splits down the middle on the same evidence is telling you something a
//! point estimate cannot, which is why `agreement` is carried alongside the
//! direction rather than collapsed into it.
//!
//! The simulation runs in the Flask service on port 5001
//! (`services/mirofish-compat/server.py`) — our own implementation of the
//! MiroFish protocol, without the camel-oasis dependency.
//!
//! Like Tape, Crowd is scored: its conclusions are filed against the
//! `prismatik.crowd` cohort and resolved against climatology.
use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, RwLock};

/// Crowd server configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CrowdConfig {
    pub(crate) base_url: String,
    pub(crate) connected: bool,
    pub(crate) version: Option<String>,
}

/// A simulation request sent to Crowd
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

/// Simulation status from Crowd
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

/// Prediction report from Crowd simulation
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CrowdPrediction {
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

static MIROFISH_CONFIG: LazyLock<RwLock<CrowdConfig>> = LazyLock::new(|| {
    RwLock::new(CrowdConfig {
        base_url: "http://localhost:5001".into(),
        connected: false,
        version: None,
    })
});

static SIMULATION_HISTORY: LazyLock<RwLock<Vec<CrowdPrediction>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// Configure Crowd connection
#[tauri::command]
pub(crate) async fn crowd_connect(base_url: Option<String>) -> Result<String, String> {
    let url = base_url.unwrap_or_else(|| "http://localhost:5001".into());
    let client = reqwest::Client::new();

    // probe Crowd health
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
            Ok(format!("Connected to Crowd at {url}"))
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
                    Ok(format!("Connected to Crowd at {url}"))
                },
                _ => Err(format!(
                    "Cannot reach Crowd at {url}. Start it with: cd mirofish && python run.py"
                )),
            }
        },
    }
}

/// Get Crowd connection status
#[tauri::command]
pub(crate) fn crowd_status() -> Result<CrowdConfig, String> {
    Ok(MIROFISH_CONFIG
        .read()
        .map_err(|_| "state unavailable")?
        .clone())
}

/// Run a Crowd simulation with market data as seed documents.
///
/// This is the core integration: PRISMATIK feeds market data, news, and
/// analysis into Crowd as seed documents. Crowd spawns thousands
/// of LLM agents with different trading personas, runs the simulation,
/// and returns an emergent prediction.
#[tauri::command]
pub(crate) async fn crowd_run_simulation(
    symbols: Vec<String>,
    seed_headlines: Vec<String>,
    prediction_query: String,
    agent_count: Option<usize>,
    rounds: Option<usize>,
) -> Result<SimulationStatus, String> {
    let base_url = {
        let config = MIROFISH_CONFIG.read().map_err(|_| "state unavailable")?;
        if !config.connected {
            return Err("Crowd not connected. Run crowd_connect first.".into());
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

    // send to Crowd
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
        .map_err(|e| format!("Crowd request failed: {e}"))?;

    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        return Err(format!("Crowd rejected simulation: {err}"));
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
pub(crate) async fn crowd_check_status(scenario_id: String) -> Result<SimulationStatus, String> {
    let base_url = {
        let config = MIROFISH_CONFIG.read().map_err(|_| "state unavailable")?;
        if !config.connected {
            return Err("Crowd not connected".into());
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
        return Err(format!("Crowd returned HTTP {}", resp.status()));
    }

    let status: SimulationStatus = resp.json().await.map_err(|e| format!("parse error: {e}"))?;
    Ok(status)
}

/// Get simulation report/prediction
#[tauri::command]
pub(crate) async fn crowd_get_prediction(scenario_id: String) -> Result<CrowdPrediction, String> {
    let base_url = {
        let config = MIROFISH_CONFIG.read().map_err(|_| "state unavailable")?;
        if !config.connected {
            return Err("Crowd not connected".into());
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
        return Err(format!("Crowd returned HTTP {}", resp.status()));
    }

    let val: serde_json::Value = resp.json().await.map_err(|e| format!("parse error: {e}"))?;

    // parse Crowd report into our prediction format
    let prediction = CrowdPrediction {
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
pub(crate) fn crowd_history() -> Result<Vec<CrowdPrediction>, String> {
    Ok(SIMULATION_HISTORY
        .read()
        .map_err(|_| "state unavailable")?
        .clone())
}

/// Build seed documents from current market data + news.
/// This is what PRISMATIK feeds into Crowd as the "world state."
#[tauri::command]
pub(crate) async fn crowd_build_seeds(symbols: Vec<String>) -> Result<Vec<SeedDocument>, String> {
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

/// Below this many agents a population is a focus group, not a population.
///
/// The probability Crowd files is a share of the swarm; with a handful of
/// agents that share moves in jumps too coarse to be a probability at all.
const MIN_POPULATION: usize = 20;

/// How the population actually split, counted from the agents themselves.
///
/// The report carries `consensus_direction` and `consensus_strength`, but
/// those are the simulation's own summary of itself and the parser defaults
/// them to neutral/0.5 when absent. Counting the sentiments is the difference
/// between a measured split and a plausible-looking one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PopulationSplit {
    pub(crate) up: usize,
    pub(crate) down: usize,
    pub(crate) flat: usize,
    pub(crate) total: usize,
}

impl PopulationSplit {
    fn count(sentiments: &[AgentSentiment]) -> Self {
        let mut split = Self {
            up: 0,
            down: 0,
            flat: 0,
            total: 0,
        };
        for sentiment in sentiments {
            match sentiment.direction.trim().to_ascii_lowercase().as_str() {
                "bullish" | "up" | "long" => split.up += 1,
                "bearish" | "down" | "short" => split.down += 1,
                // Anything the population expressed that is not a directional
                // call counts as flat rather than being dropped: silently
                // discarding agents would shrink the denominator and inflate
                // whatever share remained.
                _ => split.flat += 1,
            }
            split.total += 1;
        }
        split
    }

    /// The plurality view and the share of the population holding it.
    fn plurality(self) -> Option<(prismatik_regime::ForecastDirection, u32)> {
        if self.total == 0 {
            return None;
        }
        let share =
            |count: usize| ((count as f64 / self.total as f64) * 1_000_000.0).round() as u32;
        let (direction, count) = if self.up >= self.down && self.up >= self.flat {
            (prismatik_regime::ForecastDirection::Up, self.up)
        } else if self.down >= self.flat {
            (prismatik_regime::ForecastDirection::Down, self.down)
        } else {
            (prismatik_regime::ForecastDirection::Flat, self.flat)
        };
        Some((direction, share(count)))
    }

    /// Normalised entropy of the split, 0 = unanimous, 1 = evenly divided.
    ///
    /// This is the number Crowd contributes that no other estimator can: a
    /// population that splits down the middle on the same evidence is saying
    /// something a point estimate cannot carry.
    pub(crate) fn dispersion(self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        let total = self.total as f64;
        let entropy: f64 = [self.up, self.down, self.flat]
            .into_iter()
            .filter(|count| *count > 0)
            .map(|count| {
                let share = count as f64 / total;
                -share * share.ln()
            })
            .sum();
        // Three outcomes, so the maximum entropy is ln(3).
        (entropy / 3.0_f64.ln()).clamp(0.0, 1.0)
    }
}

fn cohort_model() -> String {
    // Bumped when the way a population becomes a directional claim changes.
    "crowd.population.v1".to_owned()
}

/// File a completed Crowd simulation as a scored forecast.
///
/// The probability is the share of the population holding the plurality view
/// — a number counted from the agents, not read off the report's own summary
/// of itself.
#[tauri::command]
pub(crate) async fn crowd_file_forecast(
    app: tauri::AppHandle,
    scenario_id: String,
    symbol: String,
    horizon_days: usize,
) -> Result<String, String> {
    let prediction = crowd_get_prediction(scenario_id).await?;
    let split = PopulationSplit::count(&prediction.agent_sentiments);
    if split.total < MIN_POPULATION {
        return Err(format!(
            "only {} agents reported a view; Crowd files nothing under {MIN_POPULATION} because a \
             share of that few is too coarse to be a probability",
            split.total
        ));
    }
    let (direction, probability_ppm) = split
        .plurality()
        .ok_or("the population expressed no views")?;

    let tracked = crate::tracking::read_tracked(&app)?;
    let row = tracked
        .iter()
        .find(|row| row.symbol.eq_ignore_ascii_case(&symbol))
        .ok_or_else(|| {
            format!("{symbol} is not tracked, so there is no series to resolve against")
        })?;
    let (bars, _) = crate::analytics::fetch_daily_bars(row.kind, &row.provider_id).await?;
    let classification =
        prismatik_regime::classify(&bars, &prismatik_regime::RegimeParams::daily());
    let climatology =
        prismatik_regime::climatology_for(&bars, &classification, horizon_days, direction)
            .ok_or("no full forward window exists, so there is no base rate to score against")?;

    let baseline_price = *bars.last().map(|bar| &bar.c).ok_or("no closing price")?;
    let observed_at = time::OffsetDateTime::from_unix_timestamp(
        bars.last().map(|bar| bar.t).unwrap_or(0) / 1_000,
    )
    .map_err(|_| "the final bar carries an unusable timestamp".to_owned())?
    .format(&time::format_description::well_known::Rfc3339)
    .map_err(|error| error.to_string())?;

    let dispersion = split.dispersion();
    let edge_ppm = i64::from(probability_ppm) - i64::from(climatology.probability_ppm);
    let summary = format!(
        "A population of {} agents split {} up / {} down / {} flat on {symbol} over \
         {horizon_days}d (dispersion {:.2}, 0 unanimous to 1 evenly divided). Base rate for the \
         stated direction is {:.1}% over {} windows, so the edge is {:+.1} points.",
        split.total,
        split.up,
        split.down,
        split.flat,
        dispersion,
        f64::from(climatology.probability_ppm) / 10_000.0,
        climatology.sample_size,
        edge_ppm as f64 / 10_000.0,
    );

    let filed = crate::forecast_candidates::file_estimator_candidate(
        crate::forecast_candidates::EstimatorClaim {
            provider_id: crate::forecast_candidates::CROWD_PROVIDER,
            model: &cohort_model(),
            target: &symbol,
            direction,
            probability_ppm,
            climatology_ppm: climatology.probability_ppm,
            summary,
            evidence_ids: vec![format!("crowd:{}", prediction.scenario_id)],
            drivers: prediction
                .emergent_themes
                .iter()
                .take(4)
                .cloned()
                .chain(std::iter::once(format!("dispersion {dispersion:.2}")))
                .collect(),
            risks: prediction
                .risk_factors
                .iter()
                .take(3)
                .cloned()
                .chain(std::iter::once(
                    "Agents share a seed corpus, so their views are correlated by construction"
                        .to_owned(),
                ))
                .collect(),
        },
        horizon_days,
        baseline_price,
        &observed_at,
    )?;

    Ok(if filed {
        format!(
            "Filed a {horizon_days}d claim for {symbol} at {:.1}% from {} agents (dispersion \
             {dispersion:.2}).",
            f64::from(probability_ppm) / 10_000.0,
            split.total,
        )
    } else {
        format!("A {horizon_days}d claim for {symbol} is already open and unresolved.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agents(spec: &[(&str, usize)]) -> Vec<AgentSentiment> {
        spec.iter()
            .flat_map(|(direction, count)| {
                (0..*count).map(move |i| AgentSentiment {
                    agent_persona: format!("agent-{i}"),
                    direction: (*direction).to_owned(),
                    conviction: 0.5,
                    reasoning: String::new(),
                })
            })
            .collect()
    }

    #[test]
    fn the_split_is_counted_from_agents_not_read_off_the_report() {
        let split =
            PopulationSplit::count(&agents(&[("bullish", 30), ("bearish", 15), ("neutral", 5)]));
        assert_eq!(split.up, 30);
        assert_eq!(split.down, 15);
        assert_eq!(split.flat, 5);
        assert_eq!(split.total, 50);

        let (direction, probability) = split.plurality().expect("plurality");
        assert_eq!(direction, prismatik_regime::ForecastDirection::Up);
        assert_eq!(probability, 600_000);
    }

    #[test]
    fn synonyms_are_counted_and_unknown_stances_are_not_dropped() {
        // Dropping an agent would shrink the denominator and inflate whatever
        // share remained — a quieter way of overstating confidence.
        let split = PopulationSplit::count(&agents(&[
            ("up", 4),
            ("long", 4),
            ("short", 3),
            ("down", 3),
            ("undecided", 6),
        ]));
        assert_eq!(split.up, 8);
        assert_eq!(split.down, 6);
        assert_eq!(split.flat, 6);
        assert_eq!(split.total, 20);
    }

    #[test]
    fn dispersion_runs_from_unanimous_to_evenly_divided() {
        let unanimous = PopulationSplit::count(&agents(&[("bullish", 40)]));
        assert!(unanimous.dispersion() < 0.001, "{}", unanimous.dispersion());

        let even = PopulationSplit::count(&agents(&[
            ("bullish", 30),
            ("bearish", 30),
            ("neutral", 30),
        ]));
        assert!(even.dispersion() > 0.999, "{}", even.dispersion());

        // A two-way split is more dispersed than unanimity, less than a
        // three-way one.
        let two_way = PopulationSplit::count(&agents(&[("bullish", 25), ("bearish", 25)]));
        assert!(two_way.dispersion() > unanimous.dispersion());
        assert!(two_way.dispersion() < even.dispersion());
    }

    #[test]
    fn an_empty_population_has_no_plurality() {
        let split = PopulationSplit::count(&[]);
        assert!(split.plurality().is_none());
        assert_eq!(split.dispersion(), 0.0);
    }

    #[test]
    fn the_probability_is_a_share_of_the_whole_population() {
        // Including the agents that disagreed. A probability computed over
        // only the winning side would always be 100%.
        let split = PopulationSplit::count(&agents(&[("bullish", 11), ("bearish", 9)]));
        let (_, probability) = split.plurality().expect("plurality");
        assert_eq!(probability, 550_000);
    }
}
