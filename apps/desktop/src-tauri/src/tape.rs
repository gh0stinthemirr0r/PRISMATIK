//! Tape — the sequence estimator.
//!
//! Tape reads price action the way a language model reads text: OHLCV bars are
//! tokenized by hierarchical vector quantization and continued by a
//! decoder-only transformer, then detokenized back into predicted bars. It is
//! the only estimator on the desk that forecasts the *shape* of the next bars
//! rather than a summary statistic.
//!
//! Inference runs in the Python sidecar on port 8766
//! (`services/kronos-sidecar/`). The weights are the open-source Kronos
//! foundation model (shiyu-coder/Kronos, pinned as a submodule) in three
//! sizes: mini (4.1M params, 2048 context), small (24.7M, 512), base
//! (102.3M, 512). PRISMATIK supplies the surface, the scoring and the
//! forecast plumbing; it did not train the model, and the sizes above are the
//! upstream project's published figures rather than anything measured here.
//!
//! **Tape earns its standing like everything else.** A prediction is not a
//! result until it has been converted into a directional claim, filed against
//! the `prismatik.tape` cohort, resolved at its horizon, and scored against
//! climatology. Until then it is a picture, not evidence.
use serde::{Deserialize, Serialize};

/// Kronos model size
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum TapeModel {
    Mini,  // 4.1M, fast, context 2048
    Small, // 24.7M, balanced, context 512
    Base,  // 102.3M, best accuracy, context 512
}

impl TapeModel {
    fn huggingface_id(&self) -> &str {
        match self {
            Self::Mini => "NeoQuasar/Kronos-mini-base",
            Self::Small => "NeoQuasar/Kronos-small-base",
            Self::Base => "NeoQuasar/Kronos-base-base",
        }
    }

    fn display_name(&self) -> &str {
        match self {
            Self::Mini => "Kronos-mini (4.1M)",
            Self::Small => "Kronos-small (24.7M)",
            Self::Base => "Kronos-base (102.3M)",
        }
    }
}

/// A single predicted K-line bar
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TapeBar {
    pub(crate) timestamp: String,
    pub(crate) open: f64,
    pub(crate) high: f64,
    pub(crate) low: f64,
    pub(crate) close: f64,
    pub(crate) volume: f64,
}

/// Kronos prediction result
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TapePrediction {
    pub(crate) symbol: String,
    pub(crate) model: String,
    pub(crate) context_length: usize,
    pub(crate) prediction_length: usize,
    pub(crate) sample_count: usize,
    pub(crate) predicted_bars: Vec<TapeBar>,
    pub(crate) confidence_bands: Vec<TapeConfidenceBand>,
    pub(crate) predicted_direction: String,
    pub(crate) predicted_return: f64,
    pub(crate) predicted_volatility: f64,
    pub(crate) generated_at: String,
    pub(crate) inference_time_ms: u64,
    /// Directional probabilities from the sampler's own terminal paths, or
    /// `None` when the run produced no sampling distribution.
    pub(crate) probabilities: Option<TapeProbabilities>,
    /// True when the sidecar could not load the model and answered with its
    /// mean-reversion sketch instead.
    pub(crate) is_fallback: bool,
}

/// Directional probabilities measured from the sampled paths.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TapeProbabilities {
    pub(crate) probability_up_ppm: u32,
    pub(crate) probability_down_ppm: u32,
    pub(crate) probability_flat_ppm: u32,
    pub(crate) terminal_paths: usize,
    pub(crate) median_move_bps: f64,
}

fn parse_probabilities(value: &serde_json::Value) -> Option<TapeProbabilities> {
    let node = value.get("probabilities")?;
    if node.is_null() {
        return None;
    }
    let ppm = |key: &str| -> Option<u32> {
        let p = node[key].as_f64()?;
        p.is_finite()
            .then(|| (p.clamp(0.0, 1.0) * 1_000_000.0).round() as u32)
    };
    Some(TapeProbabilities {
        probability_up_ppm: ppm("probability_up")?,
        probability_down_ppm: ppm("probability_down")?,
        probability_flat_ppm: ppm("probability_flat")?,
        terminal_paths: node["terminal_paths"].as_u64().unwrap_or(0) as usize,
        median_move_bps: node["median_move_bps"].as_f64().unwrap_or(0.0),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TapeConfidenceBand {
    pub(crate) step: usize,
    pub(crate) close_lower: f64,
    pub(crate) close_upper: f64,
    pub(crate) close_median: f64,
    pub(crate) high_lower: f64,
    pub(crate) high_upper: f64,
    pub(crate) low_lower: f64,
    pub(crate) low_upper: f64,
}

/// Batch prediction for multiple symbols
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TapeBatchResult {
    pub(crate) predictions: Vec<TapePrediction>,
    pub(crate) total_symbols: usize,
    pub(crate) successful: usize,
    pub(crate) failed: usize,
    pub(crate) total_time_ms: u64,
}

/// Run Kronos prediction for a single symbol.
/// Calls the Python sidecar which loads the model and runs inference.
#[tauri::command]
pub(crate) async fn tape_predict(
    symbol: String,
    prices: Vec<f64>,
    timestamps: Vec<String>,
    pred_len: Option<usize>,
    model: Option<String>,
    sample_count: Option<usize>,
) -> Result<TapePrediction, String> {
    let pred_len = pred_len.unwrap_or(5);
    let model_name = model.unwrap_or_else(|| "base".into());
    let sample_count = sample_count.unwrap_or(10);

    if prices.len() < 60 {
        return Err("Kronos needs at least 60 data points (K-line context window)".into());
    }

    let start = std::time::Instant::now();

    // try Python sidecar first
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "symbol": symbol,
        "prices": prices,
        "timestamps": timestamps,
        "pred_len": pred_len,
        "model": model_name,
        "sample_count": sample_count,
    });

    let resp = client
        .post("http://localhost:8766/predict")
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let val: serde_json::Value = r.json().await.map_err(|e| format!("parse error: {e}"))?;
            let inference_ms = start.elapsed().as_millis() as u64;

            let predicted_bars: Vec<TapeBar> = val["predicted_bars"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|b| TapeBar {
                            timestamp: b["timestamp"].as_str().unwrap_or("").into(),
                            open: b["open"].as_f64().unwrap_or(0.0),
                            high: b["high"].as_f64().unwrap_or(0.0),
                            low: b["low"].as_f64().unwrap_or(0.0),
                            close: b["close"].as_f64().unwrap_or(0.0),
                            volume: b["volume"].as_f64().unwrap_or(0.0),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let confidence_bands: Vec<TapeConfidenceBand> = val["confidence_bands"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .enumerate()
                        .map(|(i, b)| TapeConfidenceBand {
                            step: i + 1,
                            close_lower: b["close_lower"].as_f64().unwrap_or(0.0),
                            close_upper: b["close_upper"].as_f64().unwrap_or(0.0),
                            close_median: b["close_median"].as_f64().unwrap_or(0.0),
                            high_lower: b["high_lower"].as_f64().unwrap_or(0.0),
                            high_upper: b["high_upper"].as_f64().unwrap_or(0.0),
                            low_lower: b["low_lower"].as_f64().unwrap_or(0.0),
                            low_upper: b["low_upper"].as_f64().unwrap_or(0.0),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let last_price = *prices.last().unwrap_or(&0.0);
            let pred_close = predicted_bars.last().map(|b| b.close).unwrap_or(last_price);
            let predicted_return = if last_price > 0.0 {
                (pred_close - last_price) / last_price
            } else {
                0.0
            };

            // compute predicted volatility from predicted bars
            let returns: Vec<f64> = predicted_bars
                .windows(2)
                .map(|w| {
                    if w[0].close > 0.0 {
                        (w[1].close - w[0].close) / w[0].close
                    } else {
                        0.0
                    }
                })
                .collect();
            let mean_ret = if returns.is_empty() {
                0.0
            } else {
                returns.iter().sum::<f64>() / returns.len() as f64
            };
            let volatility = if returns.len() < 2 {
                0.0
            } else {
                (returns.iter().map(|r| (r - mean_ret).powi(2)).sum::<f64>()
                    / (returns.len() - 1) as f64)
                    .sqrt()
            };

            Ok(TapePrediction {
                probabilities: parse_probabilities(&val),
                is_fallback: val["is_fallback"].as_bool().unwrap_or(false),
                symbol,
                model: format!("Kronos-{}", model_name),
                context_length: prices.len(),
                prediction_length: pred_len,
                sample_count,
                predicted_bars,
                confidence_bands,
                predicted_direction: if predicted_return > 0.01 {
                    "bullish"
                } else if predicted_return < -0.01 {
                    "bearish"
                } else {
                    "neutral"
                }
                .into(),
                predicted_return,
                predicted_volatility: volatility,
                generated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                inference_time_ms: inference_ms,
            })
        },
        Ok(response) => Err(format!(
            "the Tape sidecar answered {} — start it with `python services/kronos-sidecar/server.py`",
            response.status()
        )),
        Err(error) => Err(format!(
            "the Tape sidecar on port 8766 is not reachable ({error}). Start it with \
             `python services/kronos-sidecar/server.py`."
        )),
    }
}

/// Run Kronos batch prediction for multiple symbols.
#[tauri::command]
pub(crate) async fn tape_batch_predict(
    symbols: Vec<String>,
    data: Vec<(Vec<f64>, Vec<String>)>, // (prices, timestamps) per symbol
    pred_len: Option<usize>,
    model: Option<String>,
) -> Result<TapeBatchResult, String> {
    if symbols.len() != data.len() {
        return Err("symbols and data must have same length".into());
    }

    let start = std::time::Instant::now();
    let mut predictions = Vec::new();
    let mut successful = 0;
    let mut failed = 0;

    for (symbol, (prices, timestamps)) in symbols.iter().zip(data.iter()) {
        match tape_predict(
            symbol.clone(),
            prices.clone(),
            timestamps.clone(),
            pred_len,
            model.clone(),
            Some(5),
        )
        .await
        {
            Ok(pred) => {
                successful += 1;
                predictions.push(pred);
            },
            Err(_) => {
                failed += 1;
            },
        }
    }

    Ok(TapeBatchResult {
        predictions,
        total_symbols: symbols.len(),
        successful,
        failed,
        total_time_ms: start.elapsed().as_millis() as u64,
    })
}

/// Get available Kronos models and their status.
#[tauri::command]
pub(crate) fn tape_models() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![
        serde_json::json!({
            "id": "mini",
            "name": "Kronos-mini",
            "params": "4.1M",
            "context": 2048,
            "speed": "fastest",
            "accuracy": "good",
            "use_case": "Real-time inference, high-frequency signals"
        }),
        serde_json::json!({
            "id": "small",
            "name": "Kronos-small",
            "params": "24.7M",
            "context": 512,
            "speed": "fast",
            "accuracy": "better",
            "use_case": "Balanced speed/accuracy for daily analysis"
        }),
        serde_json::json!({
            "id": "base",
            "name": "Kronos-base",
            "params": "102.3M",
            "context": 512,
            "speed": "moderate",
            "accuracy": "best",
            "use_case": "Highest accuracy for research and strategy"
        }),
    ])
}

/// Run Kronos prediction using the existing historical data engine.
/// Fetches OHLCV from Yahoo Finance, then runs Kronos on it.
#[tauri::command]
pub(crate) async fn tape_predict_from_market(
    symbol: String,
    pred_len: Option<usize>,
    model: Option<String>,
) -> Result<TapePrediction, String> {
    // fetch historical data
    let hist = crate::historical_data::get_historical_ohlcv(
        symbol.clone(),
        Some("1d".into()),
        Some("1y".into()),
    )
    .await?;

    if hist.bars.len() < 60 {
        return Err(format!(
            "Not enough data for Kronos: only {} bars (need 60+)",
            hist.bars.len()
        ));
    }

    let prices: Vec<f64> = hist.bars.iter().map(|b| b.close).collect();
    let timestamps: Vec<String> = hist.bars.iter().map(|b| b.timestamp.clone()).collect();

    tape_predict(symbol, prices, timestamps, pred_len, model, Some(10)).await
}

/// Cohort key for a Tape run.
///
/// The model size is part of the key because Kronos-mini and Kronos-base are
/// different estimators; pooling their records would let a strong one carry a
/// weak one. The `v1` is this integration's contract version — if the way a
/// prediction becomes a directional claim changes, it bumps and a fresh
/// cohort starts rather than the change being absorbed into an existing
/// record.
fn cohort_model(model: &str) -> String {
    format!(
        "tape.{}.v1",
        model.trim().to_ascii_lowercase().replace(' ', "-")
    )
}

/// Run Tape on a tracked instrument and file the result as a scored forecast.
///
/// Tape is only worth having if it can be shown to beat the base rate, and
/// that means its claims have to be filed before the outcome is known and
/// resolved against the same climatology as every other estimator. This is
/// the command that closes that loop.
///
/// It refuses in two cases, both deliberate:
///
/// - the sidecar fell back to its mean-reversion sketch, which is not a model
///   and must never enter a foundation-model cohort;
/// - the run produced no sampling distribution, so there is no probability to
///   file — and a direction without a probability cannot be Brier-scored.
#[tauri::command]
pub(crate) async fn tape_file_forecast(
    app: tauri::AppHandle,
    symbol: String,
    horizon_days: usize,
) -> Result<String, String> {
    let tracked = crate::tracking::read_tracked(&app)?;
    let row = tracked
        .iter()
        .find(|row| row.symbol.eq_ignore_ascii_case(&symbol))
        .ok_or_else(|| format!("{symbol} is not tracked, so there is nothing to forecast"))?;

    let (bars, _) = crate::analytics::fetch_daily_bars(row.kind, &row.provider_id).await?;
    if bars.len() < 60 {
        return Err(format!(
            "{symbol} has {} daily bars; Tape needs at least 60 for its context window",
            bars.len()
        ));
    }
    let classification =
        prismatik_regime::classify(&bars, &prismatik_regime::RegimeParams::daily());

    let prices: Vec<f64> = bars.iter().map(|bar| bar.c).collect();
    let timestamps: Vec<String> = bars.iter().map(|bar| bar.t.to_string()).collect();
    let prediction = tape_predict(
        symbol.clone(),
        prices,
        timestamps,
        Some(horizon_days),
        None,
        None,
    )
    .await?;

    if prediction.is_fallback {
        return Err(
            "the sidecar answered with its statistical fallback, not the model. That is a \
             mean-reversion sketch, and filing it under a Tape cohort would corrupt the record."
                .into(),
        );
    }
    let probabilities = prediction
        .probabilities
        .as_ref()
        .ok_or("this run produced no sampling distribution, so there is no probability to score")?;

    // The direction Tape claims is whichever outcome its sampled paths
    // favoured — taken from the same counts that produce the probability, so
    // the two can never disagree.
    let (direction, probability_ppm) = if probabilities.probability_up_ppm
        >= probabilities.probability_down_ppm
        && probabilities.probability_up_ppm >= probabilities.probability_flat_ppm
    {
        (
            prismatik_regime::ForecastDirection::Up,
            probabilities.probability_up_ppm,
        )
    } else if probabilities.probability_down_ppm >= probabilities.probability_flat_ppm {
        (
            prismatik_regime::ForecastDirection::Down,
            probabilities.probability_down_ppm,
        )
    } else {
        (
            prismatik_regime::ForecastDirection::Flat,
            probabilities.probability_flat_ppm,
        )
    };

    let climatology =
        prismatik_regime::climatology_for(&bars, &classification, horizon_days, direction)
            .ok_or("no full forward window exists, so there is no base rate to score against")?;
    let edge_ppm = i64::from(probability_ppm) - i64::from(climatology.probability_ppm);

    let quote_price = *bars.last().map(|bar| &bar.c).ok_or("no closing price")?;
    // The baseline is the last bar's close and the time that bar closed, not
    // "now": resolution measures from the price the claim was actually made
    // against, and a live quote would drift from the series Tape was given.
    let observed_at = time::OffsetDateTime::from_unix_timestamp(
        bars.last().map(|bar| bar.t).unwrap_or(0) / 1_000,
    )
    .map_err(|_| "the final bar carries an unusable timestamp".to_owned())?
    .format(&time::format_description::well_known::Rfc3339)
    .map_err(|error| error.to_string())?;

    let summary = format!(
        "Tape ({}) continued {symbol} over {horizon_days}d across {} sampled paths: {:.1}% up, \
         {:.1}% down, {:.1}% flat, median {:+.0} bps. Base rate for the stated direction is \
         {:.1}% over {} windows, so the edge is {:+.1} points.",
        prediction.model,
        probabilities.terminal_paths,
        f64::from(probabilities.probability_up_ppm) / 10_000.0,
        f64::from(probabilities.probability_down_ppm) / 10_000.0,
        f64::from(probabilities.probability_flat_ppm) / 10_000.0,
        probabilities.median_move_bps,
        f64::from(climatology.probability_ppm) / 10_000.0,
        climatology.sample_size,
        edge_ppm as f64 / 10_000.0,
    );

    let filed = crate::forecast_candidates::file_estimator_candidate(
        crate::forecast_candidates::EstimatorClaim {
            provider_id: crate::forecast_candidates::TAPE_PROVIDER,
            model: &cohort_model(&prediction.model),
            target: &symbol,
            direction,
            probability_ppm,
            climatology_ppm: climatology.probability_ppm,
            summary,
            evidence_ids: vec![format!("ohlcv:{symbol}:{}bars", bars.len())],
            drivers: vec![
                format!(
                    "{} sampled continuation paths",
                    probabilities.terminal_paths
                ),
                format!(
                    "median predicted move {:+.0} bps",
                    probabilities.median_move_bps
                ),
                format!("context window {} bars", prediction.context_length),
            ],
            risks: vec![
                "The model was pre-trained on other instruments and is applied zero-shot"
                    .to_owned(),
                "Sampled paths are correlated through a shared context window".to_owned(),
            ],
        },
        horizon_days,
        quote_price,
        &observed_at,
    )?;

    Ok(if filed {
        format!(
            "Filed a {horizon_days}d {} claim for {symbol} at {:.1}% against a {:.1}% base rate.",
            match direction {
                prismatik_regime::ForecastDirection::Up => "up",
                prismatik_regime::ForecastDirection::Down => "down",
                prismatik_regime::ForecastDirection::Flat => "flat",
            },
            f64::from(probability_ppm) / 10_000.0,
            f64::from(climatology.probability_ppm) / 10_000.0,
        )
    } else {
        format!("A {horizon_days}d claim for {symbol} is already open and unresolved.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cohort_key_carries_the_model_size() {
        // Kronos-mini and Kronos-base are different estimators. Pooling their
        // records would let a strong one carry a weak one.
        assert_eq!(cohort_model("Kronos-base"), "tape.kronos-base.v1");
        assert_eq!(cohort_model("Kronos-mini"), "tape.kronos-mini.v1");
        assert_ne!(cohort_model("Kronos-base"), cohort_model("Kronos-mini"));
    }

    #[test]
    fn the_fallback_never_shares_a_cohort_with_the_model() {
        assert_ne!(
            cohort_model("statistical_fallback"),
            cohort_model("Kronos-base"),
        );
    }

    #[test]
    fn probabilities_are_absent_rather_than_zero_when_the_sampler_gave_none() {
        // A null probabilities block means "no sampling distribution", which
        // must not be read as "zero percent" — the filing path refuses on
        // None and would happily file a confident 0% claim otherwise.
        let null = serde_json::json!({"probabilities": null});
        assert!(parse_probabilities(&null).is_none());
        let missing = serde_json::json!({});
        assert!(parse_probabilities(&missing).is_none());
    }

    #[test]
    fn sampled_probabilities_convert_to_ppm() {
        let value = serde_json::json!({
            "probabilities": {
                "probability_up": 0.62,
                "probability_down": 0.30,
                "probability_flat": 0.08,
                "terminal_paths": 50,
                "median_move_bps": 41.5,
            }
        });
        let parsed = parse_probabilities(&value).expect("probabilities");
        assert_eq!(parsed.probability_up_ppm, 620_000);
        assert_eq!(parsed.probability_down_ppm, 300_000);
        assert_eq!(parsed.probability_flat_ppm, 80_000);
        assert_eq!(parsed.terminal_paths, 50);
    }
}
