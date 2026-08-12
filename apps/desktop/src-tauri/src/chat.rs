use prismatik_application::ModelCredentialKind;
use std::{
    sync::{LazyLock, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::model_integrations::session;

// ── conversation state ──────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatMessage {
    pub(crate) role: String,
    pub(crate) content: String,
    pub(crate) timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tokens: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatState {
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) active_provider: Option<String>,
    pub(crate) active_model: Option<String>,
}

static CHAT_HISTORY: LazyLock<RwLock<Vec<ChatMessage>>> = LazyLock::new(|| RwLock::new(Vec::new()));

static ACTIVE_PROVIDER: LazyLock<RwLock<Option<String>>> = LazyLock::new(|| RwLock::new(None));

static ACTIVE_MODEL: LazyLock<RwLock<Option<String>>> = LazyLock::new(|| RwLock::new(None));

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── system prompt ───────────────────────────────────────────────────

fn system_prompt() -> String {
    "You are PRISMATIK, an agentic market intelligence platform. You have access to live market data, \
     news feeds, agent council findings, portfolio state, and risk metrics. \
     \
     When the user asks about markets, instruments, or trading: provide analysis grounded in evidence, \
     state confidence levels as intervals not points, note which regime you're conditioning on, and \
     flag what would change your mind. \
     \
     When the user asks you to do something: confirm what you understood, propose the action, and \
     wait for confirmation before any capital-mutating operations. \
     \
     Be concise. Use the terminal's language — precision over prose. \
     If you don't have current data for something, say so rather than guessing."
        .into()
}

// ── chat command ────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatResponse {
    pub(crate) reply: ChatMessage,
    pub(crate) state: ChatState,
}

#[tauri::command]
pub(crate) async fn chat_send(
    message: String,
    provider_id: Option<String>,
    model: Option<String>,
) -> Result<ChatResponse, String> {
    if message.trim().is_empty() {
        return Err("message cannot be empty".into());
    }
    if message.len() > 32_000 {
        return Err("message too long (max 32,000 characters)".into());
    }

    // resolve provider
    let pid = provider_id
        .or_else(|| ACTIVE_PROVIDER.read().ok()?.clone())
        .ok_or("no model provider selected — connect one in Models workspace first")?;

    let active = session(&pid)
        .ok_or_else(|| format!("provider '{pid}' has no active session — connect it first"))?;

    let model_name = model
        .or_else(|| ACTIVE_MODEL.read().ok()?.clone())
        .unwrap_or_else(|| default_model(&pid));

    // push user message
    let user_msg = ChatMessage {
        role: "user".into(),
        content: message,
        timestamp: now_millis(),
        provider: None,
        model: None,
        tokens: None,
    };
    CHAT_HISTORY
        .write()
        .map_err(|_| "chat state unavailable")?
        .push(user_msg.clone());

    // build messages array for the API
    let api_messages: Vec<serde_json::Value> = {
        let history = CHAT_HISTORY.read().map_err(|_| "chat state unavailable")?;
        let mut msgs = vec![serde_json::json!({
            "role": "system",
            "content": system_prompt()
        })];
        for msg in history.iter() {
            msgs.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }
        msgs
    };

    // call the model
    let (_, credential, credential_kind) = active.parts();
    let provider = active.provider_id();

    let result = call_chat(
        provider,
        Some(credential),
        credential_kind,
        &model_name,
        &api_messages,
    )
    .await;

    match result {
        Ok((text, tokens_used)) => {
            let reply = ChatMessage {
                role: "assistant".into(),
                content: text,
                timestamp: now_millis(),
                provider: Some(pid.clone()),
                model: Some(model_name),
                tokens: tokens_used,
            };
            CHAT_HISTORY
                .write()
                .map_err(|_| "chat state unavailable")?
                .push(reply.clone());

            // persist active provider/model
            *ACTIVE_PROVIDER.write().map_err(|_| "state unavailable")? = Some(pid.clone());
            *ACTIVE_MODEL.write().map_err(|_| "state unavailable")? =
                Some(reply.model.clone().unwrap_or_default());

            Ok(ChatResponse {
                reply,
                state: chat_get_state_inner()?,
            })
        },
        Err(e) => {
            // remove the user message on failure
            if let Ok(mut hist) = CHAT_HISTORY.write() {
                hist.pop();
            }
            Err(e)
        },
    }
}

// ── get / clear state ───────────────────────────────────────────────

fn chat_get_state_inner() -> Result<ChatState, String> {
    let messages = CHAT_HISTORY
        .read()
        .map_err(|_| "chat state unavailable")?
        .clone();
    let active_provider = ACTIVE_PROVIDER
        .read()
        .map_err(|_| "state unavailable")?
        .clone();
    let active_model = ACTIVE_MODEL
        .read()
        .map_err(|_| "state unavailable")?
        .clone();
    Ok(ChatState {
        messages,
        active_provider,
        active_model,
    })
}

#[tauri::command]
pub(crate) fn chat_get_state() -> Result<ChatState, String> {
    chat_get_state_inner()
}

#[tauri::command]
pub(crate) fn chat_clear() -> Result<ChatState, String> {
    CHAT_HISTORY
        .write()
        .map_err(|_| "chat state unavailable")?
        .clear();
    chat_get_state_inner()
}

#[tauri::command]
pub(crate) fn chat_set_active_provider(
    provider_id: String,
    model: Option<String>,
) -> Result<(), String> {
    *ACTIVE_PROVIDER.write().map_err(|_| "state unavailable")? = Some(provider_id);
    if let Some(m) = model {
        *ACTIVE_MODEL.write().map_err(|_| "state unavailable")? = Some(m);
    }
    Ok(())
}

// ── model routing ───────────────────────────────────────────────────

fn default_model(provider: &str) -> String {
    match provider {
        "openai" => "gpt-4o".into(),
        "anthropic" => "claude-sonnet-4-20250514".into(),
        "google" => "gemini-2.5-flash".into(),
        "xai" => "grok-3".into(),
        "deepseek" => "deepseek-chat".into(),
        "groq" => "llama-3.3-70b-versatile".into(),
        "local" => "local-model".into(),
        _ => "default".into(),
    }
}

// ── raw API calls ───────────────────────────────────────────────────

pub(crate) async fn call_chat(
    provider: &str,
    api_key: Option<String>,
    credential_kind: ModelCredentialKind,
    model: &str,
    messages: &[serde_json::Value],
) -> Result<(String, Option<u64>), String> {
    let client = reqwest::Client::new();

    match provider {
        "openai" => {
            // Every OpenAI credential kind is presented as a bearer token.
            let url = "https://api.openai.com/v1/chat/completions".to_string();
            let body = serde_json::json!({
                "model": model,
                "messages": messages,
                "max_tokens": 4096,
                "temperature": 0.7,
            });
            let mut req = client.post(&url).json(&body);
            if let Some(key) = &api_key {
                req = req.header("authorization", format!("Bearer {key}"));
            }
            let resp = req
                .send()
                .await
                .map_err(|e| format!("request failed: {e}"))?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(format!("HTTP {status}: {text}"));
            }
            let val: serde_json::Value =
                resp.json().await.map_err(|e| format!("parse error: {e}"))?;
            let text = val["choices"][0]["message"]["content"]
                .as_str()
                .ok_or("no content in response")?
                .to_string();
            let tokens = val["usage"]["total_tokens"].as_u64();
            Ok((text, tokens))
        },
        "anthropic" => {
            let key = api_key.ok_or("Anthropic requires API key")?;
            let body = serde_json::json!({
                "model": model,
                "max_tokens": 4096,
                "system": system_prompt(),
                "messages": messages.iter().filter(|m| m["role"] != "system").collect::<Vec<_>>(),
            });
            // An API key is an `x-api-key` header; OAuth and session tokens
            // are bearer credentials. Anthropic 401s on the wrong pairing.
            let authed = match credential_kind {
                ModelCredentialKind::ApiKey => client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", &key),
                ModelCredentialKind::OAuth | ModelCredentialKind::SessionToken => client
                    .post("https://api.anthropic.com/v1/messages")
                    .bearer_auth(&key),
            };
            let resp = authed
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("request failed: {e}"))?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(format!("HTTP {status}: {text}"));
            }
            let val: serde_json::Value =
                resp.json().await.map_err(|e| format!("parse error: {e}"))?;
            let text = val["content"][0]["text"]
                .as_str()
                .ok_or("no content in response")?
                .to_string();
            let input = val["usage"]["input_tokens"].as_u64().unwrap_or(0);
            let output = val["usage"]["output_tokens"].as_u64().unwrap_or(0);
            Ok((text, Some(input + output)))
        },
        "google" => {
            let key = api_key.ok_or("Google requires API key");
            let key = key?;
            // Convert messages to Gemini format
            let contents: Vec<serde_json::Value> = messages
                .iter()
                .filter(|m| m["role"] != "system")
                .map(|m| {
                    let role = if m["role"] == "assistant" {
                        "model"
                    } else {
                        "user"
                    };
                    serde_json::json!({
                        "role": role,
                        "parts": [{ "text": m["content"] }]
                    })
                })
                .collect();
            let body = serde_json::json!({
                "contents": contents,
                "systemInstruction": { "parts": [{ "text": system_prompt() }] },
                "generationConfig": { "maxOutputTokens": 4096, "temperature": 0.7 },
            });
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
            );
            // Google takes an API key as `?key=` and an OAuth token as a
            // bearer header; a token in the query string is rejected.
            let authed = match credential_kind {
                ModelCredentialKind::ApiKey => client.post(&url).query(&[("key", &key)]),
                ModelCredentialKind::OAuth | ModelCredentialKind::SessionToken => {
                    client.post(&url).bearer_auth(&key)
                },
            };
            let resp = authed
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("request failed: {e}"))?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(format!("HTTP {status}: {text}"));
            }
            let val: serde_json::Value =
                resp.json().await.map_err(|e| format!("parse error: {e}"))?;
            let text = val["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .ok_or("no content in response")?
                .to_string();
            let tokens = val["usageMetadata"]["totalTokenCount"].as_u64();
            Ok((text, tokens))
        },
        _ => Err(format!("unsupported provider: {provider}")),
    }
}
