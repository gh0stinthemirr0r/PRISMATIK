//! Cassette replay transport (Wave 1 contract tests / offline demo).
//!
//! Spec: `DOCS/spec/PROVIDER_ADAPTERS.md` §2.5, §13.
//! Cassettes are JSON (same fields as the YAML examples) so we stay
//! dependency-light in Layer 2.

use crate::http::{HttpMethod, HttpRequest, HttpResponse, HttpTransport, TransportError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One recorded exchange.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CassetteExchange {
    /// Stable name.
    pub name: String,
    /// Recorded request.
    pub request: CassetteRequest,
    /// Recorded response.
    pub response: CassetteResponse,
}

/// Cassette request shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CassetteRequest {
    /// Method.
    pub method: HttpMethod,
    /// Path.
    pub path: String,
    /// Query.
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    /// Headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

/// Cassette response shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CassetteResponse {
    /// Status.
    pub status: u16,
    /// Headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Body.
    pub body: String,
}

/// Replay-only transport. Network is never touched.
#[derive(Debug, Clone)]
pub struct CassetteTransport {
    exchanges: Vec<CassetteExchange>,
}

impl CassetteTransport {
    /// Construct from an in-memory cassette list.
    pub fn new(exchanges: Vec<CassetteExchange>) -> Self {
        Self { exchanges }
    }

    /// Parse a JSON cassette document (`[ CassetteExchange, ... ]`).
    pub fn from_json(json: &str) -> Result<Self, TransportError> {
        let exchanges: Vec<CassetteExchange> =
            serde_json::from_str(json).map_err(|e| TransportError::Decode(e.to_string()))?;
        Ok(Self::new(exchanges))
    }

    fn matches(req: &HttpRequest, recorded: &CassetteRequest) -> bool {
        if req.method != recorded.method || req.path != recorded.path {
            return false;
        }
        // All recorded query keys must match (extra live keys ignored).
        recorded
            .query
            .iter()
            .all(|(k, v)| req.query.get(k).map(|x| x == v).unwrap_or(false))
    }
}

#[async_trait]
impl HttpTransport for CassetteTransport {
    async fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError> {
        self.exchanges
            .iter()
            .find(|ex| Self::matches(request, &ex.request))
            .map(|ex| HttpResponse {
                status: ex.response.status,
                headers: ex.response.headers.clone(),
                body: ex.response.body.clone(),
            })
            .ok_or_else(|| TransportError::Offline {
                method: request.method,
                path: request.path.clone(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn replays_matching_exchange() {
        let json = r#"[
          {
            "name": "markets",
            "request": { "method": "GET", "path": "/coins/markets", "query": { "vs_currency": "usd" }, "headers": {} },
            "response": { "status": 200, "headers": {}, "body": "[{\"id\":\"bitcoin\"}]" }
          }
        ]"#;
        let t = CassetteTransport::from_json(json).unwrap();
        let mut q = BTreeMap::new();
        q.insert("vs_currency".into(), "usd".into());
        q.insert("extra".into(), "1".into());
        let res = t
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/coins/markets".into(),
                query: q,
                headers: BTreeMap::new(),
                body: None,
            })
            .await
            .unwrap();
        assert_eq!(res.status, 200);
        assert!(res.body.contains("bitcoin"));
    }
}
