//! Live and chaos implementations of the market-data HTTP port.

use async_trait::async_trait;
use prismatik_market_data::{HttpMethod, HttpRequest, HttpResponse, HttpTransport, TransportError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Data acquisition mode selected by the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataMode {
    /// Deterministic cassette replay.
    Cassette,
    /// Real network requests.
    Live,
    /// Forced-offline chaos testing.
    Chaos,
}

/// Reqwest-backed GET transport.
#[derive(Clone, Debug)]
pub struct ReqwestTransport {
    client: reqwest::Client,
    base_url: reqwest::Url,
}

impl ReqwestTransport {
    /// Create a transport rooted at a provider base URL.
    pub fn new(base_url: &str) -> Result<Self, TransportError> {
        let base_url =
            reqwest::Url::parse(base_url).map_err(|error| TransportError::Io(error.to_string()))?;
        Ok(Self {
            client: reqwest::Client::new(),
            base_url,
        })
    }
}

#[async_trait]
impl HttpTransport for ReqwestTransport {
    async fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError> {
        if request.method != HttpMethod::Get {
            return Err(TransportError::Io(
                "Wave 1 live transport supports GET only".into(),
            ));
        }
        let url = if let Ok(url) = reqwest::Url::parse(&request.path) {
            url
        } else {
            self.base_url
                .join(&request.path)
                .map_err(|error| TransportError::Io(error.to_string()))?
        };
        let mut builder = self.client.get(url).query(&request.query);
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        let response = builder
            .send()
            .await
            .map_err(|error| TransportError::Io(error.to_string()))?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_ascii_lowercase(),
                    value.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let body = response
            .text()
            .await
            .map_err(|error| TransportError::Decode(error.to_string()))?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

/// Chaos transport that deterministically simulates an offline provider.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlackholeTransport;

#[async_trait]
impl HttpTransport for BlackholeTransport {
    async fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError> {
        Err(TransportError::Offline {
            method: request.method,
            path: request.path.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn blackhole_is_always_offline() {
        let request = HttpRequest {
            method: HttpMethod::Get,
            path: "/ping".into(),
            query: BTreeMap::new(),
            headers: BTreeMap::new(),
        };
        assert!(matches!(
            BlackholeTransport.execute(&request).await,
            Err(TransportError::Offline { .. })
        ));
    }
}
