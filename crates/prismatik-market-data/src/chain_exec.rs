//! Ordered provider-chain execution for crypto markets.

use crate::types::CryptoMarketQuote;
use async_trait::async_trait;
use prismatik_domain::ProviderId;

/// Provider fetch failure understood by the chain executor.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FetchMarketsError {
    /// The provider returned no usable rows; try the next provider.
    #[error("provider returned no market rows")]
    Empty,
    /// A non-failover provider error.
    #[error("provider fetch failed: {0}")]
    Provider(String),
}

/// Minimal crypto-market fetch port used by the chain executor.
#[async_trait]
pub trait FetchMarkets: Send + Sync + std::fmt::Debug {
    /// Provider registry identifier.
    fn provider_id(&self) -> ProviderId;

    /// Fetch normalized crypto market rows.
    async fn fetch_markets(&self) -> Result<Vec<CryptoMarketQuote>, FetchMarketsError>;
}

/// Successful provider-chain result.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderChainResult {
    /// Provider that served the result.
    pub provider: ProviderId,
    /// Normalized rows.
    pub markets: Vec<CryptoMarketQuote>,
}

/// Executes providers in configured order.
#[derive(Debug)]
pub struct ProviderChainExecutor {
    providers: Vec<Box<dyn FetchMarkets>>,
}

impl ProviderChainExecutor {
    /// Construct an executor in primary-to-fallback order.
    pub fn new(providers: Vec<Box<dyn FetchMarkets>>) -> Self {
        Self { providers }
    }

    /// Fetch markets, failing over only when a provider reports `Empty`.
    pub async fn fetch_markets(&self) -> Result<ProviderChainResult, FetchMarketsError> {
        for provider in &self.providers {
            match provider.fetch_markets().await {
                Ok(markets) => {
                    return Ok(ProviderChainResult {
                        provider: provider.provider_id(),
                        markets,
                    });
                },
                Err(FetchMarketsError::Empty) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(FetchMarketsError::Empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[derive(Debug)]
    struct StubProvider {
        id: ProviderId,
        responses: Mutex<VecDeque<Result<Vec<CryptoMarketQuote>, FetchMarketsError>>>,
    }

    #[async_trait]
    impl FetchMarkets for StubProvider {
        fn provider_id(&self) -> ProviderId {
            self.id
        }

        async fn fetch_markets(&self) -> Result<Vec<CryptoMarketQuote>, FetchMarketsError> {
            self.responses.lock().unwrap().pop_front().unwrap()
        }
    }

    #[tokio::test]
    async fn fails_over_to_second_provider_on_empty() {
        let first = StubProvider {
            id: ProviderId::COINGECKO,
            responses: Mutex::new(VecDeque::from([Err(FetchMarketsError::Empty)])),
        };
        let second = StubProvider {
            id: ProviderId::CCXT,
            responses: Mutex::new(VecDeque::from([Ok(Vec::new())])),
        };
        let executor = ProviderChainExecutor::new(vec![Box::new(first), Box::new(second)]);

        let result = executor.fetch_markets().await.unwrap();

        assert_eq!(result.provider, ProviderId::CCXT);
    }
}
