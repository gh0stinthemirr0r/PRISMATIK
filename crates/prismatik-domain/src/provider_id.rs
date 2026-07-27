//! Provider identifier registry.

use serde::{Deserialize, Serialize};

/// Stable provider identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderId(pub u16);

impl ProviderId {
    /// CoinGecko provider id.
    pub const COINGECKO: ProviderId = ProviderId(1);
    /// Alpaca provider id.
    pub const ALPACA: ProviderId = ProviderId(2);
    /// SEC EDGAR provider id.
    pub const SEC_EDGAR: ProviderId = ProviderId(3);
    /// Unusual Whales provider id.
    pub const UNUSUAL_WHALES: ProviderId = ProviderId(4);
    /// FRED provider id.
    pub const FRED: ProviderId = ProviderId(5);
    /// CFTC provider id.
    pub const CFTC: ProviderId = ProviderId(6);
    /// Polygon provider id.
    pub const POLYGON: ProviderId = ProviderId(7);
    /// CCXT provider id.
    pub const CCXT: ProviderId = ProviderId(8);
    /// Tiingo provider id.
    pub const TIINGO: ProviderId = ProviderId(9);
}
