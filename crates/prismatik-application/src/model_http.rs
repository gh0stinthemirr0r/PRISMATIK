//! Bounded HTTP inference adapters for documented cloud and local model APIs.

use crate::ModelProvider;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

/// How a credential is presented on the wire.
///
/// Distinct from `model_plane::ModelAuth`, which records *where a credential is
/// stored* for governance. This records *how it is sent*, which is a transport
/// concern: the same secret is a header, a bearer token or a query parameter
/// depending on both the provider and how it was obtained.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModelCredentialKind {
    /// Provider-issued API key. The documented path for all three providers.
    #[default]
    ApiKey,
    /// OAuth 2.0 access token, presented as a bearer token.
    OAuth,
    /// A console or web session credential, presented as a bearer token.
    ///
    /// Consumer web sessions generally address a different API surface than
    /// the developer endpoints used here, so this is accepted but not claimed
    /// to be equivalent — see the provider catalog for per-provider status.
    SessionToken,
}

type ResponseParser = fn(Value) -> Result<ModelHttpResponse, String>;

/// Secret-bearing inference request. Debug output always redacts credentials.
#[derive(Clone)]
pub struct ModelHttpRequest {
    /// Provider family.
    pub provider: ModelProvider,
    /// Provider model identifier.
    pub model: String,
    /// Provider API key, if required.
    pub api_key: Option<String>,
    /// How `api_key` should be presented to the provider.
    ///
    /// The same string is a different kind of credential depending on how it
    /// was obtained, and each provider wants it in a different place. An
    /// Anthropic API key goes in `x-api-key`; an Anthropic OAuth token goes in
    /// `Authorization: Bearer`. Sending one as the other fails with an opaque
    /// 401, so the method travels with the credential rather than being
    /// guessed from its shape.
    pub auth: ModelCredentialKind,
    /// Base URL for a loopback OpenAI-compatible server.
    pub local_endpoint: Option<String>,
    /// Application-owned instruction, separate from evidence data.
    pub instruction: String,
    /// Untrusted evidence serialized as data.
    pub evidence: String,
    /// Maximum output tokens.
    pub max_output_tokens: u32,
}

impl std::fmt::Debug for ModelHttpRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ModelHttpRequest")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("local_endpoint", &self.local_endpoint)
            .field("max_output_tokens", &self.max_output_tokens)
            .finish_non_exhaustive()
    }
}

/// Normalized successful inference response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelHttpResponse {
    /// Provider-returned text.
    pub text: String,
    /// Input token count when supplied.
    pub input_tokens: Option<u64>,
    /// Output token count when supplied.
    pub output_tokens: Option<u64>,
    /// Provider request id when supplied.
    pub provider_request_id: Option<String>,
}

/// Execute one bounded inference against an officially documented endpoint.
pub async fn invoke_model_http(request: &ModelHttpRequest) -> Result<ModelHttpResponse, String> {
    if request.model.trim().is_empty()
        || request.instruction.trim().is_empty()
        || request.evidence.trim().is_empty()
        || request.max_output_tokens == 0
    {
        return Err("model, instruction, evidence, and max output tokens are required".into());
    }
    if request.evidence.len() > 512_000 {
        return Err("evidence packet exceeds 512 KB".into());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| error.to_string())?;
    let (builder, parser): (reqwest::RequestBuilder, ResponseParser) = match request.provider {
        ModelProvider::OpenAi => {
            let key = request
                .api_key
                .as_deref()
                .filter(|value| !value.is_empty())
                .ok_or("OpenAI credential is missing")?;
            // OpenAI presents all three credential kinds as a bearer token.
            (
                client
                    .post("https://api.openai.com/v1/responses")
                    .bearer_auth(key)
                    .json(&json!({
                        "model": request.model,
                        "instructions": request.instruction,
                        "input": request.evidence,
                        "max_output_tokens": request.max_output_tokens
                    })),
                parse_openai,
            )
        },
        ModelProvider::Anthropic => {
            let key = request
                .api_key
                .as_deref()
                .filter(|value| !value.is_empty())
                .ok_or("Anthropic credential is missing")?;
            // An API key goes in `x-api-key`; an OAuth or session token is a
            // bearer credential. Anthropic rejects the wrong pairing with a
            // bare 401, so the distinction has to be made here.
            let authed = match request.auth {
                ModelCredentialKind::ApiKey => client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", key),
                ModelCredentialKind::OAuth | ModelCredentialKind::SessionToken => client
                    .post("https://api.anthropic.com/v1/messages")
                    .bearer_auth(key),
            };
            (
                authed
                    .header("anthropic-version", "2023-06-01")
                    .json(&json!({
                        "model": request.model,
                        "system": request.instruction,
                        "messages": [{"role": "user", "content": request.evidence}],
                        "max_tokens": request.max_output_tokens
                    })),
                parse_anthropic,
            )
        },
        ModelProvider::Google => {
            let key = request
                .api_key
                .as_deref()
                .filter(|value| !value.is_empty())
                .ok_or("Google credential is missing")?;
            let model = request.model.trim().trim_start_matches("models/");
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
            );
            // Google takes an API key as a query parameter and an OAuth access
            // token as a bearer header; a token in `?key=` is rejected.
            let authed = match request.auth {
                ModelCredentialKind::ApiKey => client.post(url).query(&[("key", key)]),
                ModelCredentialKind::OAuth | ModelCredentialKind::SessionToken => {
                    client.post(url).bearer_auth(key)
                },
            };
            (
                authed.json(&json!({
                    "systemInstruction": {"parts": [{"text": request.instruction}]},
                    "contents": [{"role": "user", "parts": [{"text": request.evidence}]}],
                    "generationConfig": {"maxOutputTokens": request.max_output_tokens}
                })),
                parse_google,
            )
        },
        ModelProvider::LocalCompatible => {
            let endpoint = request
                .local_endpoint
                .as_deref()
                .ok_or("local endpoint is missing")?
                .trim_end_matches('/');
            if !(endpoint.starts_with("http://127.0.0.1:")
                || endpoint.starts_with("http://localhost:"))
            {
                return Err("local endpoint must be explicit loopback HTTP".into());
            }
            let mut builder = client
                .post(format!("{endpoint}/v1/chat/completions"))
                .json(&json!({
                    "model": request.model,
                    "messages": [
                        {"role": "system", "content": request.instruction},
                        {"role": "user", "content": request.evidence}
                    ],
                    "max_tokens": request.max_output_tokens
                }));
            if let Some(key) = request.api_key.as_deref().filter(|value| !value.is_empty()) {
                builder = builder.bearer_auth(key);
            }
            (builder, parse_compatible)
        },
        _ => return Err("provider inference adapter is not implemented".into()),
    };
    let response = builder.send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    if response
        .content_length()
        .is_some_and(|length| length > 2_000_000)
    {
        return Err("model response exceeds 2 MB".into());
    }
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    if bytes.len() > 2_000_000 {
        return Err("model response exceeds 2 MB".into());
    }
    if !status.is_success() {
        let detail = String::from_utf8_lossy(&bytes);
        return Err(format!(
            "model provider returned HTTP {status}: {}",
            detail.chars().take(500).collect::<String>()
        ));
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid model response: {error}"))?;
    let mut normalized = parser(value)?;
    normalized.provider_request_id = request_id;
    Ok(normalized)
}

fn parse_openai(value: Value) -> Result<ModelHttpResponse, String> {
    let text = value
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|item| {
            item.get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|content| content.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    normalized(
        text,
        value.pointer("/usage/input_tokens").and_then(Value::as_u64),
        value
            .pointer("/usage/output_tokens")
            .and_then(Value::as_u64),
    )
}

fn parse_anthropic(value: Value) -> Result<ModelHttpResponse, String> {
    let text = value
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|content| content.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    normalized(
        text,
        value.pointer("/usage/input_tokens").and_then(Value::as_u64),
        value
            .pointer("/usage/output_tokens")
            .and_then(Value::as_u64),
    )
}

fn parse_google(value: Value) -> Result<ModelHttpResponse, String> {
    let text = value
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    normalized(
        text,
        value
            .pointer("/usageMetadata/promptTokenCount")
            .and_then(Value::as_u64),
        value
            .pointer("/usageMetadata/candidatesTokenCount")
            .and_then(Value::as_u64),
    )
}

fn parse_compatible(value: Value) -> Result<ModelHttpResponse, String> {
    normalized(
        value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        value
            .pointer("/usage/prompt_tokens")
            .and_then(Value::as_u64),
        value
            .pointer("/usage/completion_tokens")
            .and_then(Value::as_u64),
    )
}

fn normalized(
    text: String,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
) -> Result<ModelHttpResponse, String> {
    if text.trim().is_empty() {
        return Err("model response contained no text".into());
    }
    Ok(ModelHttpResponse {
        text,
        input_tokens,
        output_tokens,
        provider_request_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_openai_responses_usage_and_text() {
        let response = parse_openai(json!({
            "output": [{"content": [{"type":"output_text","text":"Cited analysis"}]}],
            "usage": {"input_tokens": 12, "output_tokens": 7}
        }))
        .unwrap();
        assert_eq!(response.text, "Cited analysis");
        assert_eq!(response.input_tokens, Some(12));
        assert_eq!(response.output_tokens, Some(7));
    }

    #[test]
    fn normalizes_anthropic_and_google_shapes() {
        assert_eq!(
            parse_anthropic(
                json!({"content":[{"text":"A"}],"usage":{"input_tokens":2,"output_tokens":1}})
            )
            .unwrap()
            .text,
            "A"
        );
        assert_eq!(parse_google(json!({"candidates":[{"content":{"parts":[{"text":"G"}]}}],"usageMetadata":{"promptTokenCount":3,"candidatesTokenCount":1}})).unwrap().text, "G");
    }

    #[test]
    fn empty_provider_text_fails_closed() {
        assert!(parse_compatible(json!({"choices":[]})).is_err());
    }
}
