//! # prismatik-prismatik-risk
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — risk policy and pre-trade contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use approved::{RiskApproved, RiskApprovedOrderIntent};
pub use catalog::PreTradeCatalog;
pub use checks::{CheckContext, CheckId, CheckResult, PreTradeCheck, Severity};
pub use policy::{DefaultRiskPolicy, RiskError, RiskPolicy};

/// Risk-approval contracts.
pub mod approved {
    use crate::checks::CheckContext;
    use crate::policy::RiskError;

    /// Compile-time marker that an order intent passed risk checks.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RiskApprovedOrderIntent {
        /// Canonical asset id.
        pub asset_id: String,
        /// Side (`buy` or `sell`).
        pub side: String,
        /// Quantity as decimal string.
        pub quantity: String,
    }

    /// Marker for approved decision.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct RiskApproved;

    impl RiskApprovedOrderIntent {
        /// Construct an approved order intent from a validated context.
        pub(crate) fn approve(ctx: &CheckContext) -> Result<Self, RiskError> {
            if ctx.asset_id.is_empty() {
                return Err(RiskError::new("empty asset_id"));
            }
            Ok(Self {
                asset_id: ctx.asset_id.clone(),
                side: ctx.side.clone(),
                quantity: ctx.quantity.clone(),
            })
        }
    }
}

/// Pre-trade check contracts.
pub mod checks {
    /// Stable check identifier.
    pub type CheckId = &'static str;

    /// Check severity class.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Severity {
        /// Hard deny.
        HardDeny,
        /// Soft warning.
        SoftWarn,
    }

    /// Check input context.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CheckContext {
        /// Canonical asset id.
        pub asset_id: String,
        /// Side (`buy` or `sell`).
        pub side: String,
        /// Quantity as decimal string.
        pub quantity: String,
    }

    /// Check decision output.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CheckResult {
        /// Identifier of the check.
        pub id: CheckId,
        /// Severity declared by the check.
        pub severity: Severity,
        /// Whether this check passed.
        pub passed: bool,
        /// Optional diagnostic message.
        pub message: Option<String>,
    }

    /// Pre-trade check contract.
    pub trait PreTradeCheck: Send + Sync {
        /// Stable check id.
        fn id(&self) -> CheckId;
        /// Check severity.
        fn severity(&self) -> Severity;
        /// Evaluate against context.
        fn evaluate(&self, ctx: &CheckContext) -> CheckResult;
    }
}

/// Pre-trade catalog contracts.
pub mod catalog {
    /// Pre-trade catalog constants and evaluation order.
    #[derive(Debug, Default, Clone, Copy)]
    pub struct PreTradeCatalog;

    impl PreTradeCatalog {
        /// Normative fixed check order.
        pub const ORDER: [&'static str; 4] = [
            "quantity_positive",
            "asset_tradeable",
            "market_open",
            "portfolio_limit",
        ];

        /// Return deterministic evaluation index for a check identifier.
        pub fn order_index(check_id: &str) -> Option<usize> {
            Self::ORDER.iter().position(|entry| *entry == check_id)
        }
    }
}

/// Risk policy contracts.
pub mod policy {
    use crate::approved::RiskApprovedOrderIntent;
    use crate::catalog::PreTradeCatalog;
    use crate::checks::{CheckContext, CheckResult, PreTradeCheck, Severity};

    /// Risk policy error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RiskError {
        /// Error message.
        pub message: String,
        /// Optional failed hard-deny check id.
        pub failed_check_id: Option<String>,
    }

    impl RiskError {
        /// Create a new risk error.
        pub fn new(message: impl Into<String>) -> Self {
            Self {
                message: message.into(),
                failed_check_id: None,
            }
        }

        /// Create risk error pinned to a specific failed check id.
        pub fn with_check(message: impl Into<String>, failed_check_id: impl Into<String>) -> Self {
            Self {
                message: message.into(),
                failed_check_id: Some(failed_check_id.into()),
            }
        }
    }

    /// Risk policy executor.
    pub trait RiskPolicy: Send + Sync {
        /// Evaluate checks and return approved intent if hard-deny checks pass.
        fn evaluate(
            &self,
            ctx: &CheckContext,
            checks: &[&dyn PreTradeCheck],
        ) -> Result<(RiskApprovedOrderIntent, Vec<CheckResult>), RiskError>;
    }

    /// Default risk policy implementation with fixed-order semantics.
    #[derive(Debug, Default, Clone, Copy)]
    pub struct DefaultRiskPolicy;

    impl RiskPolicy for DefaultRiskPolicy {
        fn evaluate(
            &self,
            ctx: &CheckContext,
            checks: &[&dyn PreTradeCheck],
        ) -> Result<(RiskApprovedOrderIntent, Vec<CheckResult>), RiskError> {
            let mut hard_checks: Vec<&dyn PreTradeCheck> = checks
                .iter()
                .copied()
                .filter(|check| matches!(check.severity(), Severity::HardDeny))
                .collect();
            let mut soft_checks: Vec<&dyn PreTradeCheck> = checks
                .iter()
                .copied()
                .filter(|check| matches!(check.severity(), Severity::SoftWarn))
                .collect();

            sort_checks_for_determinism(&mut hard_checks);
            sort_checks_for_determinism(&mut soft_checks);

            let mut results = Vec::with_capacity(checks.len());
            let mut failed_hard: Option<CheckResult> = None;

            for check in hard_checks {
                let result = check.evaluate(ctx);
                if !result.passed && failed_hard.is_none() {
                    failed_hard = Some(result.clone());
                }
                results.push(result);
            }

            if let Some(failed) = failed_hard {
                return Err(RiskError::with_check(
                    format!("hard deny check failed: {}", failed.id),
                    failed.id,
                ));
            }

            for check in soft_checks {
                results.push(check.evaluate(ctx));
            }
            Ok((RiskApprovedOrderIntent::approve(ctx)?, results))
        }
    }

    fn sort_checks_for_determinism(checks: &mut Vec<&dyn PreTradeCheck>) {
        checks.sort_by(|left, right| {
            let l_idx = PreTradeCatalog::order_index(left.id()).unwrap_or(usize::MAX);
            let r_idx = PreTradeCatalog::order_index(right.id()).unwrap_or(usize::MAX);
            l_idx.cmp(&r_idx).then_with(|| left.id().cmp(right.id()))
        });
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::checks::{CheckContext, CheckResult, PreTradeCheck, Severity};
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct TestCheck {
            id: &'static str,
            severity: Severity,
            pass: bool,
            calls: Arc<Mutex<Vec<&'static str>>>,
        }

        impl PreTradeCheck for TestCheck {
            fn id(&self) -> &'static str {
                self.id
            }

            fn severity(&self) -> Severity {
                self.severity
            }

            fn evaluate(&self, _ctx: &CheckContext) -> CheckResult {
                self.calls.lock().expect("calls lock").push(self.id);
                CheckResult {
                    id: self.id,
                    severity: self.severity,
                    passed: self.pass,
                    message: None,
                }
            }
        }

        #[test]
        fn evaluates_hard_checks_before_soft_and_in_catalog_order() {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let checks: Vec<TestCheck> = vec![
                TestCheck {
                    id: "portfolio_limit",
                    severity: Severity::SoftWarn,
                    pass: true,
                    calls: Arc::clone(&calls),
                },
                TestCheck {
                    id: "market_open",
                    severity: Severity::HardDeny,
                    pass: true,
                    calls: Arc::clone(&calls),
                },
                TestCheck {
                    id: "quantity_positive",
                    severity: Severity::HardDeny,
                    pass: true,
                    calls: Arc::clone(&calls),
                },
            ];
            let check_refs: Vec<&dyn PreTradeCheck> = checks
                .iter()
                .map(|check| check as &dyn PreTradeCheck)
                .collect();
            let policy = DefaultRiskPolicy;
            let ctx = CheckContext {
                asset_id: "A1".to_owned(),
                side: "buy".to_owned(),
                quantity: "1".to_owned(),
            };

            let (_approved, results) = policy
                .evaluate(&ctx, &check_refs)
                .expect("evaluation should pass");
            let order = calls.lock().expect("calls lock").clone();
            assert_eq!(
                order,
                vec!["quantity_positive", "market_open", "portfolio_limit"]
            );
            assert_eq!(results.len(), 3);
        }

        #[test]
        fn returns_failed_hard_check_details() {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let checks: Vec<TestCheck> = vec![
                TestCheck {
                    id: "quantity_positive",
                    severity: Severity::HardDeny,
                    pass: false,
                    calls: Arc::clone(&calls),
                },
                TestCheck {
                    id: "portfolio_limit",
                    severity: Severity::SoftWarn,
                    pass: true,
                    calls: Arc::clone(&calls),
                },
            ];
            let check_refs: Vec<&dyn PreTradeCheck> = checks
                .iter()
                .map(|check| check as &dyn PreTradeCheck)
                .collect();
            let policy = DefaultRiskPolicy;
            let ctx = CheckContext {
                asset_id: "A1".to_owned(),
                side: "buy".to_owned(),
                quantity: "1".to_owned(),
            };

            let error = policy
                .evaluate(&ctx, &check_refs)
                .expect_err("hard deny should fail");
            assert_eq!(error.failed_check_id.as_deref(), Some("quantity_positive"));
            assert!(error.message.contains("hard deny check failed"));
            let order = calls.lock().expect("calls lock").clone();
            assert_eq!(order, vec!["quantity_positive"]);
        }
    }
}

pub mod measures;
