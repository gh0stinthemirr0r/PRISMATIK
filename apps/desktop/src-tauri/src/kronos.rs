/**
 * Kronos — Financial K-line Foundation Model Integration
 *
 * Kronos is the first open-source foundation model for candlestick prediction.
 * Pre-trained on 12B+ K-lines from 45 global exchanges.
 * Treats OHLCV as a "language" via hierarchical vector quantization.
 *
 * Architecture:
 *   OHLCV data → K-line Tokenizer (hierarchical VQ) → Tokens
 *   Tokens → Decoder-only Transformer (GPT-style) → Predicted future tokens
 *   Predicted tokens → K-line Detokenizer → Predicted OHLCV
 *
 * Models:
 *   - Kronos-mini:   4.1M params, context 2048 (fast, good for real-time)
 *   - Kronos-small: 24.7M params, context 512 (balanced)
 *   - Kronos-base: 102.3M params, context 512 (highest accuracy)
 *
 * Zero-shot RankIC: +93% over best general TSFM (AAAI 2026)
 *
 * Integration: Python sidecar on port 8766, or direct HuggingFace inference.
 */
use serde::{Deserialize, Serialize};

/// Kronos model size
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum KronosModel {
    Mini,  // 4.1M, fast, context 2048
    Small, // 24.7M, balanced, context 512
    Base,  // 102.3M, best accuracy, context 512
}

impl KronosModel {
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
pub(crate) struct KronosBar {
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
pub(crate) struct KronosPrediction {
    pub(crate) symbol: String,
    pub(crate) model: String,
    pub(crate) context_length: usize,
    pub(crate) prediction_length: usize,
    pub(crate) sample_count: usize,
    pub(crate) predicted_bars: Vec<KronosBar>,
    pub(crate) confidence_bands: Vec<KronosConfidenceBand>,
    pub(crate) predicted_direction: String,
    pub(crate) predicted_return: f64,
    pub(crate) predicted_volatility: f64,
    pub(crate) generated_at: String,
    pub(crate) inference_time_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KronosConfidenceBand {
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
pub(crate) struct KronosBatchResult {
    pub(crate) predictions: Vec<KronosPrediction>,
    pub(crate) total_symbols: usize,
    pub(crate) successful: usize,
    pub(crate) failed: usize,
    pub(crate) total_time_ms: u64,
}

/// Run Kronos prediction for a single symbol.
/// Calls the Python sidecar which loads the model and runs inference.
#[tauri::command]
pub(crate) async fn kronos_predict(
    symbol: String,
    prices: Vec<f64>,
    timestamps: Vec<String>,
    pred_len: Option<usize>,
    model: Option<String>,
    sample_count: Option<usize>,
) -> Result<KronosPrediction, String> {
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

            let predicted_bars: Vec<KronosBar> = val["predicted_bars"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|b| KronosBar {
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

            let confidence_bands: Vec<KronosConfidenceBand> = val["confidence_bands"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .enumerate()
                        .map(|(i, b)| KronosConfidenceBand {
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

            Ok(KronosPrediction {
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
        _ => {
            // fallback: statistical prediction (not Kronos, but gives a result)
            let inference_ms = start.elapsed().as_millis() as u64;
            let last_price = *prices.last().unwrap_or(&0.0);
            let mean: f64 = prices.iter().sum::<f64>() / prices.len() as f64;
            let std: f64 = (prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>()
                / prices.len() as f64)
                .sqrt();

            // simple mean-reversion forecast
            let predicted_bars: Vec<KronosBar> = (0..pred_len)
                .map(|i| {
                    let t = i as f64 + 1.0;
                    let reversion = (mean - last_price) * 0.05 * t;
                    let noise = std * 0.1 * t.sqrt();
                    let close = last_price + reversion;
                    KronosBar {
                        timestamp: format!("+{}", i + 1),
                        open: close - noise * 0.3,
                        high: close + noise,
                        low: close - noise,
                        close,
                        volume: 0.0,
                    }
                })
                .collect();

            let confidence_bands: Vec<KronosConfidenceBand> = predicted_bars
                .iter()
                .enumerate()
                .map(|(i, bar)| {
                    let widen = std * ((i + 1) as f64).sqrt() * 0.15;
                    KronosConfidenceBand {
                        step: i + 1,
                        close_lower: bar.close - widen * 1.96,
                        close_upper: bar.close + widen * 1.96,
                        close_median: bar.close,
                        high_lower: bar.high - widen,
                        high_upper: bar.high + widen,
                        low_lower: bar.low - widen,
                        low_upper: bar.low + widen,
                    }
                })
                .collect();

            let pred_close = predicted_bars.last().map(|b| b.close).unwrap_or(last_price);
            let predicted_return = if last_price > 0.0 {
                (pred_close - last_price) / last_price
            } else {
                0.0
            };

            Ok(KronosPrediction {
                symbol,
                model: "statistical_fallback (Kronos sidecar not running)".into(),
                context_length: prices.len(),
                prediction_length: pred_len,
                sample_count: 1,
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
                predicted_volatility: std / last_price,
                generated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                inference_time_ms: inference_ms,
            })
        },
    }
}

/// Run Kronos batch prediction for multiple symbols.
#[tauri::command]
pub(crate) async fn kronos_batch_predict(
    symbols: Vec<String>,
    data: Vec<(Vec<f64>, Vec<String>)>, // (prices, timestamps) per symbol
    pred_len: Option<usize>,
    model: Option<String>,
) -> Result<KronosBatchResult, String> {
    if symbols.len() != data.len() {
        return Err("symbols and data must have same length".into());
    }

    let start = std::time::Instant::now();
    let mut predictions = Vec::new();
    let mut successful = 0;
    let mut failed = 0;

    for (symbol, (prices, timestamps)) in symbols.iter().zip(data.iter()) {
        match kronos_predict(
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

    Ok(KronosBatchResult {
        predictions,
        total_symbols: symbols.len(),
        successful,
        failed,
        total_time_ms: start.elapsed().as_millis() as u64,
    })
}

/// Get available Kronos models and their status.
#[tauri::command]
pub(crate) fn kronos_models() -> Result<Vec<serde_json::Value>, String> {
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
pub(crate) async fn kronos_predict_from_market(
    symbol: String,
    pred_len: Option<usize>,
    model: Option<String>,
) -> Result<KronosPrediction, String> {
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

    kronos_predict(symbol, prices, timestamps, pred_len, model, Some(10)).await
}
