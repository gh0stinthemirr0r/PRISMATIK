//! Provider adapters.
//!
//! Spec: `DOCS/spec/PROVIDER_ADAPTERS.md`.

pub mod alpaca;
pub mod cftc;
pub mod coingecko;
pub mod crypto_venues;
pub mod finnhub;
pub mod fred;
pub mod prediction_markets;
pub mod remaining_providers;
pub mod sec_edgar;
pub mod tradingview_screener;
// NOTE: `unusual_whales.rs` exists on disk but is intentionally NOT declared
// here. It depends on `prismatik-options`, which itself depends on this crate
// — a layering cycle. It will compile once the shared option-print types are
// moved down to `prismatik-domain`. Until then it stays uncompiled rather than
// breaking the workspace. Do not delete it (briefing §1: the orphaned module
// is the spec).

pub use alpaca::{AlpacaAdapter, AlpacaCredentials, AlpacaError};
pub use cftc::{CftcAdapter, CftcError};
pub use coingecko::{demo_cassette_json, CoinGeckoAdapter, CoinGeckoAuth, CoinGeckoError};
pub use crypto_venues::{BinanceAdapter, CoinbaseAdapter, KrakenAdapter, VenueError};
pub use finnhub::{FinnhubAdapter, FinnhubError};
pub use fred::{FredAdapter, FredError};
pub use prediction_markets::{KalshiAdapter, PolymarketAdapter, PredictionVenueError};
pub use remaining_providers::{
    AlphaVantageAdapter, GdeltAdapter, GdeltArticle, IbkrAuthStatus, IbkrGatewayAdapter,
    PolygonAdapter, ProviderError,
};
pub use sec_edgar::{SecEdgarAdapter, SecEdgarError};
pub use tradingview_screener::{
    ScreenerError, ScreenerHit, ScreenerMarket, ScreenerPage, ScreenerSort, TradingViewScreener,
};
