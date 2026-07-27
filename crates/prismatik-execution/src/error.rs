//! Broker error taxonomy with permanence bit (`P7-QM-05`).
//!
//! Classification order is load-bearing: `MarketClosed` MUST be matched
//! before `Auth`. Venues return 403 for both; wrong order permanently
//! disables healthy accounts on routine after-hours reads.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Normative broker error codes (v1.2 §3.8 / OpenAlice ADR-024 clean-room).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerErrorCode {
    /// Permanent — disables the account.
    Config,
    /// Permanent — disables the account.
    Auth,
    /// Transient — auto-recover.
    Network,
    /// Transient — venue rejected; do not retry blindly.
    Exchange,
    /// Transient — expected when the market is closed; not a failure.
    MarketClosed,
    /// Data pending / mid-connect — retry shortly; NOT a health failure.
    Connecting,
    /// Unclassified.
    Unknown,
}

impl BrokerErrorCode {
    /// Permanence bit: `Config` and `Auth` disable the account.
    pub fn permanent(self) -> bool {
        matches!(self, Self::Config | Self::Auth)
    }
}

/// Gateway / adapter failure carrying a classified code.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum BrokerError {
    /// Classified broker/venue error.
    #[error("broker error {code:?}: {message}")]
    Classified {
        /// Taxonomy code.
        code: BrokerErrorCode,
        /// Human-readable detail.
        message: String,
    },
    /// Adapter has no live session (typed stub path).
    #[error("broker not connected")]
    NotConnected,
    /// Idempotency or local invariant violation.
    #[error("broker invariant: {0}")]
    Invariant(String),
}

impl BrokerError {
    /// Convenience constructor for a classified error.
    pub fn classified(code: BrokerErrorCode, message: impl Into<String>) -> Self {
        Self::Classified {
            code,
            message: message.into(),
        }
    }

    /// Extract the taxonomy code when present.
    pub fn code(&self) -> Option<BrokerErrorCode> {
        match self {
            Self::Classified { code, .. } => Some(*code),
            Self::NotConnected => Some(BrokerErrorCode::Network),
            Self::Invariant(_) => Some(BrokerErrorCode::Unknown),
        }
    }
}

/// Classify an HTTP-ish broker failure from status + body text.
///
/// **CRITICAL:** `MARKET_CLOSED` / market-closed signals are matched
/// **before** auth heuristics so a 403 after-hours read is not treated as
/// permanent `Auth`.
pub fn classify_broker_error(status: u16, body: &str) -> BrokerErrorCode {
    let lower = body.to_ascii_lowercase();

    // 1. Market closed — BEFORE auth (403 collision).
    if lower.contains("market_closed")
        || lower.contains("market closed")
        || lower.contains("\"code\":\"market_closed\"")
        || lower.contains("market is closed")
    {
        return BrokerErrorCode::MarketClosed;
    }

    // 2. Connecting / data pending — not a health failure.
    if lower.contains("connecting")
        || lower.contains("data pending")
        || lower.contains("retry shortly")
    {
        return BrokerErrorCode::Connecting;
    }

    // 3. Auth (permanent).
    if status == 401
        || lower.contains("unauthorized")
        || lower.contains("invalid api key")
        || lower.contains("authentication")
        || (status == 403
            && (lower.contains("auth")
                || lower.contains("forbidden")
                || lower.contains("permission")))
    {
        return BrokerErrorCode::Auth;
    }

    // Bare 403 without market-closed / auth markers → exchange / unknown.
    if status == 403 {
        return BrokerErrorCode::Exchange;
    }

    // 4. Config (permanent).
    if lower.contains("misconfigured")
        || lower.contains("invalid account")
        || (lower.contains("config") && (status == 400 || status == 422))
    {
        return BrokerErrorCode::Config;
    }

    // 5. Network / transient transport.
    if status == 0
        || status == 408
        || status == 429
        || (500..600).contains(&status)
        || lower.contains("timeout")
        || lower.contains("connection reset")
    {
        return BrokerErrorCode::Network;
    }

    // 6. Exchange rejection.
    if (400..500).contains(&status) || lower.contains("reject") {
        return BrokerErrorCode::Exchange;
    }

    BrokerErrorCode::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permanence_bit() {
        assert!(BrokerErrorCode::Config.permanent());
        assert!(BrokerErrorCode::Auth.permanent());
        assert!(!BrokerErrorCode::Network.permanent());
        assert!(!BrokerErrorCode::MarketClosed.permanent());
        assert!(!BrokerErrorCode::Connecting.permanent());
    }

    #[test]
    fn market_closed_before_auth_on_403() {
        let code = classify_broker_error(
            403,
            r#"{"code":"market_closed","message":"market is closed"}"#,
        );
        assert_eq!(code, BrokerErrorCode::MarketClosed);
        assert!(!code.permanent());
    }

    #[test]
    fn auth_403_when_not_market_closed() {
        let code = classify_broker_error(403, "forbidden: authentication required");
        assert_eq!(code, BrokerErrorCode::Auth);
        assert!(code.permanent());
    }

    #[test]
    fn connecting_is_not_failure_code() {
        let code = classify_broker_error(200, "connecting: data pending, retry shortly");
        assert_eq!(code, BrokerErrorCode::Connecting);
        assert!(!code.permanent());
    }
}
