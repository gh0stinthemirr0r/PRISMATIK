use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, RwLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PredictionRecord {
    pub(crate) id: String,
    pub(crate) entity: String,
    pub(crate) direction: String,
    pub(crate) confidence: f64,
    pub(crate) interval_low: f64,
    pub(crate) interval_high: f64,
    pub(crate) horizon: String,
    pub(crate) regime: String,
    pub(crate) evidence: Vec<String>,
    pub(crate) falsifiers: Vec<String>,
    pub(crate) model: String,
    pub(crate) provider: String,
    pub(crate) status: String,
    pub(crate) created_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PredictionState {
    pub(crate) predictions: Vec<PredictionRecord>,
    pub(crate) track_record: TrackRecord,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrackRecord {
    pub(crate) total: usize,
    pub(crate) correct: usize,
    pub(crate) accuracy: f64,
    pub(crate) avg_confidence: f64,
    pub(crate) calibration_error: f64,
}

static PREDICTIONS: LazyLock<RwLock<Vec<PredictionRecord>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

use crate::model_integrations::session;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[tauri::command]
pub(crate) async fn generate_live_prediction(
    entity: String,
    question: String,
    horizon: Option<String>,
    provider_id: Option<String>,
    model: Option<String>,
) -> Result<PredictionRecord, String> {
    let pid = provider_id.unwrap_or_else(|| "openai".into());
    let model_name = model.unwrap_or_else(|| "gpt-4o".into());
    let horizon = horizon.unwrap_or_else(|| "7d".into());

    let active = session(&pid).ok_or("no active model session")?;

    let prompt = format!(
        "Analyze {entity} and produce a calibrated prediction.\n\
         Question: {question}\n\
         Horizon: {horizon}\n\n\
         Respond in JSON:\n\
         {{\n\
           \"direction\": \"bullish\"|\"bearish\"|\"neutral\",\n\
           \"confidence\": 0.0-1.0,\n\
           \"interval_low\": <lower return bound>,\n\
           \"interval_high\": <upper return bound>,\n\
           \"evidence\": [\"...\"],\n\
           \"falsifiers\": [\"...\"],\n\
           \"regime\": \"...\"\n\
         }}"
    );

    let (_, credential, credential_kind) = active.parts();
    let provider_str = active.provider_id();

    let messages = vec![
        serde_json::json!({"role": "system", "content": "You are a calibrated financial prediction engine. Always respond with valid JSON."}),
        serde_json::json!({"role": "user", "content": prompt}),
    ];

    let result = crate::chat::call_chat(
        provider_str,
        Some(credential),
        credential_kind,
        &model_name,
        &messages,
    )
    .await;
    let (text, _tokens) = result?;

    // parse response
    let json_str = text
        .find('{')
        .and_then(|start| {
            text[start..]
                .find('}')
                .map(|end| &text[start..=start + end])
        })
        .ok_or("model did not return valid JSON")?;

    let parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("parse error: {e}"))?;

    let record = PredictionRecord {
        id: format!("pred_{}_{}", now_millis(), &rand_id()),
        entity,
        direction: parsed["direction"].as_str().unwrap_or("neutral").into(),
        confidence: parsed["confidence"].as_f64().unwrap_or(0.5).clamp(0.0, 1.0),
        interval_low: parsed["interval_low"].as_f64().unwrap_or(-0.05),
        interval_high: parsed["interval_high"].as_f64().unwrap_or(0.05),
        horizon,
        regime: parsed["regime"].as_str().unwrap_or("unknown").into(),
        evidence: parsed["evidence"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        falsifiers: parsed["falsifiers"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        model: model_name,
        provider: pid,
        status: "pending".into(),
        created_at: now_millis(),
    };

    PREDICTIONS
        .write()
        .map_err(|_| "state unavailable")?
        .push(record.clone());
    Ok(record)
}

#[tauri::command]
pub(crate) fn get_predictions() -> Result<PredictionState, String> {
    let preds = PREDICTIONS.read().map_err(|_| "state unavailable")?.clone();
    let total_resolved = preds
        .iter()
        .filter(|p| p.status == "confirmed" || p.status == "invalidated")
        .count();
    let correct = preds.iter().filter(|p| p.status == "confirmed").count();
    let accuracy = if total_resolved == 0 {
        0.0
    } else {
        correct as f64 / total_resolved as f64
    };
    let avg_confidence = if total_resolved == 0 {
        0.0
    } else {
        preds
            .iter()
            .filter(|p| p.status == "confirmed" || p.status == "invalidated")
            .map(|p| p.confidence)
            .sum::<f64>()
            / total_resolved as f64
    };

    Ok(PredictionState {
        predictions: preds,
        track_record: TrackRecord {
            total: total_resolved,
            correct,
            accuracy,
            avg_confidence,
            calibration_error: (accuracy - avg_confidence).abs(),
        },
    })
}

#[tauri::command]
pub(crate) fn resolve_prediction(id: String, outcome: String) -> Result<PredictionRecord, String> {
    let mut preds = PREDICTIONS.write().map_err(|_| "state unavailable")?;
    let pred = preds
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("prediction not found")?;
    pred.status = match outcome.as_str() {
        "correct" | "confirmed" => "confirmed",
        "incorrect" | "invalidated" => "invalidated",
        "expired" => "expired",
        _ => return Err("invalid outcome: use correct/incorrect/expired".into()),
    }
    .into();
    Ok(pred.clone())
}

#[tauri::command]
pub(crate) fn clear_predictions() -> Result<(), String> {
    PREDICTIONS
        .write()
        .map_err(|_| "state unavailable")?
        .clear();
    Ok(())
}

fn rand_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    now_millis().hash(&mut h);
    format!("{:x}", h.finish())
}
