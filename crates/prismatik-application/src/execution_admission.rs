//! Budget admission immediately around the typed broker submission boundary.

use crate::{AutonomyBudget, BudgetError, BudgetLane};
use prismatik_execution::{BrokerError, BrokerGateway, SubmissionResult};
use prismatik_risk::RiskApprovedOrderIntent;

/// Failure before or during a governed submission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GovernedSubmissionError {
    /// Trading capital or per-trade budget denied the order.
    Budget(BudgetError),
    /// Broker submission failed before a definitive accepted/unknown response.
    Broker(BrokerError),
}

/// Submit only a risk-approved intent after atomically reserving capital.
///
/// Definitive rejections and broker errors release the reservation. Accepted
/// and unknown outcomes retain it; unknown outcomes must reconcile before any
/// retry, matching the broker gateway contract.
pub fn submit_with_budget(
    budget: &AutonomyBudget,
    gateway: &dyn BrokerGateway,
    intent: RiskApprovedOrderIntent,
    gross_notional_micros: u64,
) -> Result<SubmissionResult, GovernedSubmissionError> {
    budget
        .reserve(BudgetLane::Trading, gross_notional_micros)
        .map_err(GovernedSubmissionError::Budget)?;
    match gateway.submit(intent) {
        Ok(result @ SubmissionResult::Accepted { .. })
        | Ok(result @ SubmissionResult::Unknown { .. }) => Ok(result),
        Ok(result @ SubmissionResult::Rejected { .. }) => {
            budget
                .release(BudgetLane::Trading, gross_notional_micros)
                .map_err(GovernedSubmissionError::Budget)?;
            Ok(result)
        },
        Err(error) => {
            budget
                .release(BudgetLane::Trading, gross_notional_micros)
                .map_err(GovernedSubmissionError::Budget)?;
            Err(GovernedSubmissionError::Broker(error))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AutonomyBudgetPolicy;
    use prismatik_execution::{BrokerErrorCode, BrokerRejection};

    struct Gateway {
        result: Result<SubmissionResult, BrokerError>,
    }
    impl BrokerGateway for Gateway {
        fn submit(&self, _: RiskApprovedOrderIntent) -> Result<SubmissionResult, BrokerError> {
            self.result.clone()
        }
    }
    fn budget(limit: u64) -> AutonomyBudget {
        AutonomyBudget::new(AutonomyBudgetPolicy {
            currency: "USD".into(),
            operations_limit_micros: 100,
            trading_limit_micros: limit,
            per_operation_limit_micros: 100,
            per_trade_limit_micros: limit.max(1),
        })
        .unwrap()
    }
    fn intent() -> RiskApprovedOrderIntent {
        RiskApprovedOrderIntent {
            asset_id: "AAPL".into(),
            side: "buy".into(),
            quantity: "1".into(),
        }
    }

    #[test]
    fn zero_trading_budget_never_calls_broker() {
        let gateway = Gateway {
            result: Ok(SubmissionResult::Accepted {
                broker_order_id: "impossible".into(),
                accepted_at: "now".into(),
            }),
        };
        let result = submit_with_budget(&budget(0), &gateway, intent(), 1);
        assert!(matches!(
            result,
            Err(GovernedSubmissionError::Budget(
                BudgetError::Exhausted { .. }
            ))
        ));
    }

    #[test]
    fn definitive_rejection_releases_capital() {
        let budget = budget(100);
        let gateway = Gateway {
            result: Ok(SubmissionResult::Rejected {
                reason: BrokerRejection {
                    code: "LIMIT".into(),
                    message: "no".into(),
                },
            }),
        };
        assert!(matches!(
            submit_with_budget(&budget, &gateway, intent(), 75),
            Ok(SubmissionResult::Rejected { .. })
        ));
        assert_eq!(budget.snapshot().unwrap().trading_used_micros, 0);
    }

    #[test]
    fn unknown_outcome_remains_reserved_for_reconciliation() {
        use prismatik_execution::{IdempotencyKey, LocalOrderState};
        let budget = budget(100);
        let gateway = Gateway {
            result: Ok(SubmissionResult::Unknown {
                idempotency_key: IdempotencyKey("one".into()),
                last_known_state: LocalOrderState::Submitted,
            }),
        };
        assert!(matches!(
            submit_with_budget(&budget, &gateway, intent(), 75),
            Ok(SubmissionResult::Unknown { .. })
        ));
        assert_eq!(budget.snapshot().unwrap().trading_used_micros, 75);
    }

    #[test]
    fn broker_error_releases_capital() {
        let budget = budget(100);
        let gateway = Gateway {
            result: Err(BrokerError {
                code: BrokerErrorCode::Network,
                message: "offline".into(),
            }),
        };
        assert!(matches!(
            submit_with_budget(&budget, &gateway, intent(), 75),
            Err(GovernedSubmissionError::Broker(_))
        ));
        assert_eq!(budget.snapshot().unwrap().trading_used_micros, 0);
    }
}
