//! Forecast cache keyed by asset, modality, and context hash (`P5-QM-10` floor).

use crate::registry::SeriesModality;
use crate::runtime::TsfmForecastResponse;
use prismatik_determinism::{ContentHash, DetMap};
use serde::{Deserialize, Serialize};

/// Cache key: `(asset, modality, context_hash)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ForecastCacheKey {
    /// Asset identifier.
    pub asset: String,
    /// Series modality.
    pub modality: SeriesModality,
    /// BLAKE3 of the context window bytes.
    pub context_hash: ContentHash,
}

impl ForecastCacheKey {
    /// Build a key from asset, modality, and raw context floats.
    ///
    /// Context is hashed as little-endian `f64` bytes for stability.
    #[must_use]
    pub fn from_context(
        asset: impl Into<String>,
        modality: SeriesModality,
        context: &[f64],
    ) -> Self {
        let mut bytes = Vec::with_capacity(context.len() * 8);
        for v in context {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        Self {
            asset: asset.into(),
            modality,
            context_hash: ContentHash::from_bytes(&bytes),
        }
    }
}

/// In-memory forecast cache.
#[derive(Clone, Debug, Default)]
pub struct ForecastCache {
    entries: DetMap<ForecastCacheKey, TsfmForecastResponse>,
}

impl ForecastCache {
    /// Create an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of cached forecasts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert or replace a forecast for `key`.
    pub fn insert(&mut self, key: ForecastCacheKey, response: TsfmForecastResponse) {
        self.entries.insert(key, response);
    }

    /// Look up a cached forecast.
    pub fn get(&self, key: &ForecastCacheKey) -> Option<&TsfmForecastResponse> {
        self.entries.get(key)
    }

    /// Remove and return a cached forecast.
    pub fn remove(&mut self, key: &ForecastCacheKey) -> Option<TsfmForecastResponse> {
        self.entries.remove(key)
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::OutOfDomainVerdict;

    #[test]
    fn keyed_by_asset_modality_and_context_hash() {
        let mut cache = ForecastCache::new();
        let ctx = [1.0_f64, 2.0, 3.0];
        let key = ForecastCacheKey::from_context("AAPL", SeriesModality::Ohlcv, &ctx);
        let other_asset = ForecastCacheKey::from_context("MSFT", SeriesModality::Ohlcv, &ctx);
        let other_mod = ForecastCacheKey::from_context("AAPL", SeriesModality::Univariate, &ctx);
        let other_ctx =
            ForecastCacheKey::from_context("AAPL", SeriesModality::Ohlcv, &[1.0, 2.0, 3.1]);

        assert_ne!(key, other_asset);
        assert_ne!(key, other_mod);
        assert_ne!(key, other_ctx);

        let response = TsfmForecastResponse {
            point: vec![4.0],
            quantiles: Vec::new(),
            ood: OutOfDomainVerdict::Unevaluated,
        };
        cache.insert(key.clone(), response.clone());
        assert_eq!(cache.get(&key), Some(&response));
        assert!(cache.get(&other_asset).is_none());
        assert_eq!(cache.len(), 1);
    }
}
