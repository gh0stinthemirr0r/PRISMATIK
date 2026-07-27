//! External identifier types for asset resolution.

use crate::MicCode;
use serde::{Deserialize, Serialize};

/// Provider-facing identifiers that resolve to canonical `AssetId` values.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ExternalIdentifier {
    /// OpenFIGI identifier.
    Figi(String),
    /// ISIN identifier.
    Isin(String),
    /// CUSIP identifier.
    Cusip(String),
    /// Venue-scoped ticker symbol.
    TickerAtVenue {
        /// Ticker text.
        ticker: String,
        /// Venue MIC.
        mic: MicCode,
    },
    /// OCC option symbol.
    OccOptionSymbol(String),
    /// CoinGecko identifier.
    CoinGeckoId(String),
    /// On-chain asset address.
    OnChain {
        /// Chain identifier.
        chain_id: u64,
        /// Address text.
        address: String,
    },
    /// CFTC market code.
    CftcMarketCode(String),
    /// SEC CIK.
    SecCik(u64),
}
