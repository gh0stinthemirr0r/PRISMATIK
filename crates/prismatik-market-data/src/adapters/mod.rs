//! Provider adapters.
//!
//! Spec: `DOCS/spec/PROVIDER_ADAPTERS.md`.

pub mod alpaca;
pub mod cftc;
pub mod coingecko;
pub mod finnhub;
pub mod fred;
pub mod sec_edgar;
pub mod unusual_whales;

pub use alpaca::{AlpacaAdapter, AlpacaError};
pub use cftc::{CftcAdapter, CftcError};
pub use coingecko::{demo_cassette_json, CoinGeckoAdapter, CoinGeckoAuth, CoinGeckoError};
pub use finnhub::{FinnhubAdapter, FinnhubError};
pub use fred::{FredAdapter, FredError};
pub use sec_edgar::{SecEdgarAdapter, SecEdgarError};
pub use unusual_whales::{UnusualWhalesAdapter, UnusualWhalesError};
