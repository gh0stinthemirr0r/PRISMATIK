//! Broker adapters: deterministic paper simulator and typed Alpaca stub.

mod alpaca;
mod paper;

pub use alpaca::AlpacaBroker;
pub use paper::{PaperBroker, PaperHooks};
