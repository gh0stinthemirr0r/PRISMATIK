//! Outbound HTTP port for provider adapters.
//!
//! Domain crates never depend on `reqwest`. Live clients live in the
//! application/shell layer and implement this trait.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// HTTP method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    /// GET
    Get,
    /// POST
    Post,
}

/// Transport request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpRequest {
    /// Method.
    pub method: HttpMethod,
    /// Absolute or host-relative path (e.g. `/coins/markets`).
    pub path: String,
    /// Query string parameters.
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    /// Headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Request body, for methods that carry one.
    ///
    /// Optional and defaulted so that cassettes recorded before POST was
    /// supported still deserialize: a GET has no body and its absence must not
    /// invalidate a stored interaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// Transport response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpResponse {
    /// Status code.
    pub status: u16,
    /// Response headers (lowercased keys recommended).
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// UTF-8 body.
    pub body: String,
}

/// Transport errors.
#[derive(Debug, Error)]
pub enum TransportError {
    /// No matching cassette / offline miss.
    #[error("transport offline: no cassette for {method:?} {path}")]
    Offline {
        /// Method.
        method: HttpMethod,
        /// Path.
        path: String,
    },
    /// Network or I/O failure.
    #[error("transport I/O: {0}")]
    Io(String),
    /// Invalid response encoding.
    #[error("transport decode: {0}")]
    Decode(String),
}

/// HTTP port implemented by live clients and cassette replay.
#[async_trait]
pub trait HttpTransport: Send + Sync {
    /// Execute a request.
    async fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError>;
}
