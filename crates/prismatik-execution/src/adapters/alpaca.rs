//! Typed Alpaca broker stub — **no network** (`P7-QM-04` floor).
//!
//! Live HTTP adapter is deferred. Every gateway method returns
//! [`BrokerError::NotConnected`] so callers cannot accidentally place live
//! orders through this crate floor.

use crate::broker_state::LocalOrderState;
use crate::error::BrokerError;
use crate::gateway::{
    BrokerGateway, BrokerOrderId, CancelResult, OrderModification, ReplaceResult, SubmissionResult,
};
use crate::order::ApprovedOrder;
use crate::reconcile::ReconciliationDelta;

/// Alpaca adapter placeholder. Holds config labels only; never opens sockets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AlpacaBroker {
    /// Logical account label (never a secret).
    pub account_label: String,
    /// Whether the operator intends paper vs live endpoint (unused until wired).
    pub live_intended: bool,
}

impl AlpacaBroker {
    /// Construct a disconnected stub.
    pub fn stub(account_label: impl Into<String>, live_intended: bool) -> Self {
        Self {
            account_label: account_label.into(),
            live_intended,
        }
    }
}

impl BrokerGateway for AlpacaBroker {
    fn broker_id(&self) -> &str {
        "alpaca"
    }

    fn submit(&mut self, _order: &ApprovedOrder) -> Result<SubmissionResult, BrokerError> {
        Err(BrokerError::NotConnected)
    }

    fn cancel(&mut self, _order_id: &BrokerOrderId) -> Result<CancelResult, BrokerError> {
        Err(BrokerError::NotConnected)
    }

    fn replace(
        &mut self,
        _order_id: &BrokerOrderId,
        _modification: &OrderModification,
    ) -> Result<ReplaceResult, BrokerError> {
        Err(BrokerError::NotConnected)
    }

    fn reconcile(
        &self,
        _local_state: &LocalOrderState,
    ) -> Result<ReconciliationDelta, BrokerError> {
        Err(BrokerError::NotConnected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idempotency::IdempotencyKey;

    #[test]
    fn alpaca_stub_never_connects() {
        let mut alpaca = AlpacaBroker::stub("paper-acct", false);
        let order = ApprovedOrder::new("AAPL", 1, IdempotencyKey::generate(b"a"));
        assert_eq!(
            alpaca.submit(&order).unwrap_err(),
            BrokerError::NotConnected
        );
        assert_eq!(
            alpaca.cancel(&BrokerOrderId::new("x")).unwrap_err(),
            BrokerError::NotConnected
        );
    }
}
