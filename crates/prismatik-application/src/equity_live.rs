//! Wave 2 live equity OHLCV wiring (application layer).
//!
//! Adapters stay cassette-friendly in `prismatik-market-data`; tokens and live
//! HTTP roots live here. Without env credentials, live mode refuses rather than
//! leaking empty-key requests.

use crate::http_live::ReqwestTransport;
use async_trait::async_trait;
use prismatik_market_data::{
    adapters::{AlpacaAdapter, FinnhubAdapter},
    HttpRequest, HttpResponse, HttpTransport, TransportError,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;

/// Default Alpaca market-data HTTP root (IEX feed path family).
pub const ALPACA_DATA_BASE_URL: &str = "https://data.alpaca.markets";
/// Default Finnhub API root.
pub const FINNHUB_API_BASE_URL: &str = "https://finnhub.io/api/v1";

/// Env var: Alpaca API key id.
pub const ENV_ALPACA_KEY_ID: &str = "ALPACA_API_KEY_ID";
/// Env var: Alpaca API secret.
pub const ENV_ALPACA_SECRET: &str = "ALPACA_API_SECRET_KEY";
/// Env var: Finnhub API token.
pub const ENV_FINNHUB_TOKEN: &str = "FINNHUB_API_TOKEN";

/// Live equity credential / wiring errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EquityLiveError {
    /// Required credential env var missing or empty.
    #[error("missing live equity credential: {0}")]
    MissingCredential(&'static str),
    /// Base URL could not be parsed.
    #[error("invalid live equity base URL: {0}")]
    InvalidBaseUrl(String),
}

/// Credentials loaded from the process environment (never logged).
#[derive(Clone, Debug)]
pub struct EquityLiveCredentials {
    /// Alpaca key id + secret, when both present.
    pub alpaca: Option<AlpacaCredentials>,
    /// Finnhub token, when present.
    pub finnhub_token: Option<String>,
}

/// Alpaca header pair.
#[derive(Clone)]
pub struct AlpacaCredentials {
    /// `APCA-API-KEY-ID`.
    pub key_id: String,
    /// `APCA-API-SECRET-KEY`.
    pub secret_key: String,
}

impl std::fmt::Debug for AlpacaCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlpacaCredentials")
            .field("key_id", &"<redacted>")
            .field("secret_key", &"<redacted>")
            .finish()
    }
}

impl EquityLiveCredentials {
    /// Read credentials from env. Missing vars become `None` (not an error).
    pub fn from_env() -> Self {
        Self::from_map(|key| std::env::var(key).ok())
    }

    /// Testable loader: `lookup` returns env-like values.
    pub fn from_map<F>(mut lookup: F) -> Self
    where
        F: FnMut(&str) -> Option<String>,
    {
        let alpaca = match (
            nonempty(lookup(ENV_ALPACA_KEY_ID)),
            nonempty(lookup(ENV_ALPACA_SECRET)),
        ) {
            (Some(key_id), Some(secret_key)) => Some(AlpacaCredentials { key_id, secret_key }),
            _ => None,
        };
        let finnhub_token = nonempty(lookup(ENV_FINNHUB_TOKEN));
        Self {
            alpaca,
            finnhub_token,
        }
    }

    /// True when at least one live equity provider can be built.
    pub fn any_provider(&self) -> bool {
        self.alpaca.is_some() || self.finnhub_token.is_some()
    }
}

fn nonempty(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let t = s.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    })
}

/// Transport that injects static headers on every request.
pub struct HeaderInjectingTransport {
    inner: Arc<dyn HttpTransport>,
    headers: BTreeMap<String, String>,
}

impl std::fmt::Debug for HeaderInjectingTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeaderInjectingTransport")
            .field("header_names", &self.headers.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl HeaderInjectingTransport {
    /// Wrap `inner` and merge `headers` (caller wins on key collision).
    pub fn new(inner: Arc<dyn HttpTransport>, headers: BTreeMap<String, String>) -> Self {
        Self { inner, headers }
    }
}

#[async_trait]
impl HttpTransport for HeaderInjectingTransport {
    async fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError> {
        let mut headers = request.headers.clone();
        for (k, v) in &self.headers {
            headers.insert(k.clone(), v.clone());
        }
        let forwarded = HttpRequest {
            method: request.method,
            path: request.path.clone(),
            query: request.query.clone(),
            headers,
        };
        self.inner.execute(&forwarded).await
    }
}

/// Live equity adapter set (Wave 2 chain: Alpaca → Finnhub).
#[derive(Debug)]
pub struct LiveEquityAdapters {
    /// Alpaca adapter when credentials were present.
    pub alpaca: Option<AlpacaAdapter>,
    /// Finnhub adapter when token was present.
    pub finnhub: Option<FinnhubAdapter>,
}

impl LiveEquityAdapters {
    /// Build live adapters from credentials. Errors only on bad base URLs.
    ///
    /// Returns [`EquityLiveError::MissingCredential`] when **no** provider can be built.
    pub fn try_build(creds: &EquityLiveCredentials) -> Result<Self, EquityLiveError> {
        Self::try_build_with_bases(creds, ALPACA_DATA_BASE_URL, FINNHUB_API_BASE_URL)
    }

    /// Same as [`Self::try_build`] with injectable roots (tests / proxies).
    pub fn try_build_with_bases(
        creds: &EquityLiveCredentials,
        alpaca_base: &str,
        finnhub_base: &str,
    ) -> Result<Self, EquityLiveError> {
        if !creds.any_provider() {
            return Err(EquityLiveError::MissingCredential(ENV_FINNHUB_TOKEN));
        }

        let alpaca = if let Some(ref a) = creds.alpaca {
            let transport = ReqwestTransport::new(alpaca_base)
                .map_err(|e| EquityLiveError::InvalidBaseUrl(format!("{alpaca_base}: {e}")))?;
            let mut headers = BTreeMap::new();
            headers.insert("APCA-API-KEY-ID".into(), a.key_id.clone());
            headers.insert("APCA-API-SECRET-KEY".into(), a.secret_key.clone());
            let authed = HeaderInjectingTransport::new(Arc::new(transport), headers);
            Some(AlpacaAdapter::new(Arc::new(authed)))
        } else {
            None
        };

        let finnhub = if let Some(ref token) = creds.finnhub_token {
            let transport = ReqwestTransport::new(finnhub_base)
                .map_err(|e| EquityLiveError::InvalidBaseUrl(format!("{finnhub_base}: {e}")))?;
            Some(FinnhubAdapter::new(Arc::new(transport), token.clone()))
        } else {
            None
        };

        Ok(Self { alpaca, finnhub })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_market_data::{HttpMethod, HttpRequest};
    use std::sync::Mutex;

    #[derive(Default)]
    struct CaptureTransport {
        last: Mutex<Option<HttpRequest>>,
    }

    #[async_trait]
    impl HttpTransport for CaptureTransport {
        async fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, TransportError> {
            *self.last.lock().unwrap() = Some(HttpRequest {
                method: request.method,
                path: request.path.clone(),
                query: request.query.clone(),
                headers: request.headers.clone(),
                body: None,
            });
            Ok(HttpResponse {
                status: 200,
                headers: BTreeMap::new(),
                body: "{}".into(),
            })
        }
    }

    #[test]
    fn credentials_require_both_alpaca_halves() {
        let only_key = EquityLiveCredentials::from_map(|k| match k {
            ENV_ALPACA_KEY_ID => Some("key".into()),
            _ => None,
        });
        assert!(only_key.alpaca.is_none());
        assert!(!only_key.any_provider());
    }

    #[test]
    fn finnhub_alone_is_enough_for_live_floor() {
        let creds = EquityLiveCredentials::from_map(|k| match k {
            ENV_FINNHUB_TOKEN => Some("tok".into()),
            _ => None,
        });
        assert!(creds.any_provider());
        let adapters = LiveEquityAdapters::try_build(&creds).expect("finnhub live");
        assert!(adapters.alpaca.is_none());
        assert!(adapters.finnhub.is_some());
    }

    #[test]
    fn missing_all_credentials_refuses() {
        let creds = EquityLiveCredentials::from_map(|_| None);
        let err = LiveEquityAdapters::try_build(&creds).unwrap_err();
        assert!(matches!(err, EquityLiveError::MissingCredential(_)));
    }

    #[tokio::test]
    async fn header_inject_adds_alpaca_auth() {
        let capture = Arc::new(CaptureTransport::default());
        let mut headers = BTreeMap::new();
        headers.insert("APCA-API-KEY-ID".into(), "id".into());
        headers.insert("APCA-API-SECRET-KEY".into(), "sec".into());
        let transport = HeaderInjectingTransport::new(capture.clone(), headers);
        let _ = transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "/v2/stocks/AAPL/bars".into(),
                query: BTreeMap::new(),
                headers: BTreeMap::new(),
                body: None,
            })
            .await
            .unwrap();
        let seen = capture.last.lock().unwrap().clone().unwrap();
        assert_eq!(
            seen.headers.get("APCA-API-KEY-ID").map(String::as_str),
            Some("id")
        );
        assert_eq!(
            seen.headers.get("APCA-API-SECRET-KEY").map(String::as_str),
            Some("sec")
        );
    }
}
