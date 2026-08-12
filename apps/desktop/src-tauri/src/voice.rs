use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TtsResult {
    pub(crate) audio_base64: String,
    pub(crate) duration_ms: u64,
    pub(crate) format: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SttResult {
    pub(crate) text: String,
    pub(crate) language: String,
    pub(crate) confidence: f64,
}

/// Text-to-speech using the voice service (edge-tts, free, high quality).
#[tauri::command]
pub(crate) async fn tts_speak(
    text: String,
    voice: Option<String>,
    speed: Option<f64>,
) -> Result<TtsResult, String> {
    if text.is_empty() {
        return Err("text cannot be empty".into());
    }
    if text.len() > 10_000 {
        return Err("text too long (max 10,000 characters)".into());
    }

    let voice = voice.unwrap_or_else(|| "en-US-AriaNeural".into());
    let rate = speed
        .map(|s| format!("{}%", ((s - 1.0) * 100.0) as i32))
        .unwrap_or_else(|| "+0%".into());

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "text": text,
        "voice": voice,
        "rate": rate,
    });

    let resp = client
        .post("http://127.0.0.1:8767/tts")
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("voice service error: {e}"))?;

    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        return Err(format!("TTS failed: {err}"));
    }

    let val: serde_json::Value = resp.json().await.map_err(|e| format!("parse error: {e}"))?;

    Ok(TtsResult {
        audio_base64: val["audio_base64"].as_str().unwrap_or("").into(),
        duration_ms: (text.len() as u64 * 50),
        format: val["format"].as_str().unwrap_or("mp3").into(),
    })
}

/// Speech-to-text using the voice service (faster-whisper).
#[tauri::command]
pub(crate) async fn stt_transcribe(
    audio_base64: String,
    language: Option<String>,
) -> Result<SttResult, String> {
    if audio_base64.is_empty() {
        return Err("no audio data provided".into());
    }

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "audio": audio_base64,
        "language": language,
    });

    let resp = client
        .post("http://127.0.0.1:8767/stt")
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("voice service error: {e}"))?;

    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        return Err(format!("STT failed: {err}"));
    }

    let val: serde_json::Value = resp.json().await.map_err(|e| format!("parse error: {e}"))?;

    Ok(SttResult {
        text: val["text"].as_str().unwrap_or("").into(),
        language: val["language"].as_str().unwrap_or("en").into(),
        confidence: val["confidence"].as_f64().unwrap_or(0.8),
    })
}
