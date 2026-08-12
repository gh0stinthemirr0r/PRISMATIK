/**
 * Time-Series Foundation Model Integration — Chronos (Amazon) and TimesFM (Google).
 * Zero-shot forecasting with no training required.
 * Runs as isolated Python sidecar processes.
 */
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TSForecast {
    pub(crate) symbol: String,
    pub(crate) model: String,
    pub(crate) horizon: usize,
    pub(crate) point_forecast: Vec<f64>,
    pub(crate) quantile_forecasts: Vec<QuantileForecast>,
    pub(crate) confidence_intervals: Vec<ConfidenceInterval>,
    pub(crate) generated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuantileForecast {
    pub(crate) quantile: f64,
    pub(crate) values: Vec<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfidenceInterval {
    pub(crate) step: usize,
    pub(crate) lower: f64,
    pub(crate) upper: f64,
    pub(crate) point: f64,
}

/// Generate zero-shot forecast using Chronos or TimesFM.
/// Calls the Python sidecar which loads the model and runs inference.
#[tauri::command]
pub(crate) async fn tsfm_forecast(
    symbol: String,
    prices: Vec<f64>,
    horizon: Option<usize>,
    model: Option<String>,
) -> Result<TSForecast, String> {
    let horizon = horizon.unwrap_or(7);
    let model = model.unwrap_or_else(|| "chronos".into());

    if prices.len() < 30 {
        return Err("Need at least 30 data points for forecasting".into());
    }

    // call Python sidecar
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "symbol": symbol,
        "prices": prices,
        "horizon": horizon,
        "model": model,
    });

    let resp = client
        .post("http://localhost:8765/forecast")
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let val: serde_json::Value = r.json().await.map_err(|e| format!("parse error: {e}"))?;
            Ok(TSForecast {
                symbol,
                model: val["model"].as_str().unwrap_or(&model).into(),
                horizon,
                point_forecast: val["point_forecast"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default(),
                quantile_forecasts: val["quantiles"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|q| QuantileForecast {
                                quantile: q["quantile"].as_f64().unwrap_or(0.5),
                                values: q["values"]
                                    .as_array()
                                    .map(|v| v.iter().filter_map(|x| x.as_f64()).collect())
                                    .unwrap_or_default(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                confidence_intervals: val["intervals"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .enumerate()
                            .map(|(i, v)| ConfidenceInterval {
                                step: i + 1,
                                lower: v["lower"].as_f64().unwrap_or(0.0),
                                upper: v["upper"].as_f64().unwrap_or(0.0),
                                point: v["point"].as_f64().unwrap_or(0.0),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                generated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            })
        },
        _ => {
            // fallback: simple statistical forecast
            let last = *prices.last().unwrap_or(&0.0);
            let mean: f64 = prices.iter().sum::<f64>() / prices.len() as f64;
            let std: f64 = (prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>()
                / prices.len() as f64)
                .sqrt();

            let point_forecast: Vec<f64> = (0..horizon)
                .map(|i| last + (mean - last) * (1.0 - (-0.1 * i as f64).exp()))
                .collect();

            let confidence_intervals: Vec<ConfidenceInterval> = point_forecast
                .iter()
                .enumerate()
                .map(|(i, &p)| {
                    let widen = std * (i as f64 + 1.0).sqrt() * 0.1;
                    ConfidenceInterval {
                        step: i + 1,
                        lower: p - widen * 1.96,
                        upper: p + widen * 1.96,
                        point: p,
                    }
                })
                .collect();

            Ok(TSForecast {
                symbol,
                model: "statistical_fallback".into(),
                horizon,
                point_forecast: point_forecast.clone(),
                quantile_forecasts: vec![
                    QuantileForecast {
                        quantile: 0.1,
                        values: confidence_intervals.iter().map(|c| c.lower).collect(),
                    },
                    QuantileForecast {
                        quantile: 0.5,
                        values: point_forecast,
                    },
                    QuantileForecast {
                        quantile: 0.9,
                        values: confidence_intervals.iter().map(|c| c.upper).collect(),
                    },
                ],
                confidence_intervals,
                generated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            })
        },
    }
}
