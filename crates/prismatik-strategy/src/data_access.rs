//! Data-access logical plan stub and point-in-time rewrite (`P4-QM-04` / `P4-QM-05`).
//!
//! # Normative contract (DataFusion deferred)
//!
//! Real compilation to Apache DataFusion `LogicalPlan` and an optimizer
//! rewrite rule are **deferred** — linking DataFusion is intentionally out of
//! scope for this floor (heavy dep; fail-closed stub only). This module is the
//! **normative contract**:
//!
//! 1. Strategy data access is expressed as a Prismatik logical-plan AST
//!    ([`LogicalPlanStub`]) with [`Scan`](LogicalPlanStub::Scan) /
//!    [`Filter`](LogicalPlanStub::Filter) / [`Project`](LogicalPlanStub::Project)
//!    (plus `Limit`, `Join`, and [`AsOfFilter`](LogicalPlanStub::AsOfFilter)).
//! 2. Every leaf [`LogicalPlanStub::Scan`] carries an optional
//!    [`as_of_bound`](LogicalPlanStub::Scan::as_of_bound) (UTC micros).
//! 3. [`enforce_pit`] **must** run before any execution path: it binds every
//!    `Scan`, recursively rewrites joins/filters/projects, **collapses** nested
//!    [`AsOfFilter`](LogicalPlanStub::AsOfFilter) nodes, and **inserts** a
//!    single root as-of wrapper so no unbounded or future-leaking plan remains.
//!
//! When DataFusion is wired later, the same PIT invariant applies via a real
//! plan-rewrite rule; this stub remains the behavioral reference for tests.
//!
//! # PIT invariants (fail-closed)
//!
//! | Invariant | Rule |
//! |---|---|
//! | Bound every leaf | After [`enforce_pit`], every `Scan.as_of_bound == Some(as_of)`. |
//! | Cover every path | Every `Scan` must sit under an `AsOfFilter` ancestor (partial coverage on one join side is **unsafe**). |
//! | Single enforced bound | Nested / stale `AsOfFilter` wrappers are stripped; root wrapper uses the enforced `as_of_micros`. |
//! | Override wins | Pre-bound scans and prior as-of wrappers are overwritten by the enforced bound. |
//! | Join without as-of | Equi-join of scans with no covering `AsOfFilter` is **not** PIT-safe (look-ahead / label-leak negative). |
//! | No DataFusion link | This crate does not depend on DataFusion; the stub fails closed without it. |

use std::sync::Arc;

/// Lightweight Prismatik logical-plan AST for strategy data access
/// (DataFusion stand-in; no DataFusion link).
///
/// Core relational nodes: scan, filter, project. Extensions: limit, equi-join,
/// and the PIT [`AsOfFilter`](LogicalPlanStub::AsOfFilter) inserted by
/// [`enforce_pit`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicalPlanStub {
    /// Table / feature-view scan.
    Scan {
        /// Logical source name (table, view, or feature id).
        source: String,
        /// Point-in-time upper bound in UTC microseconds since Unix epoch.
        /// `None` means **unbounded** (look-ahead risk) — forbidden after
        /// [`enforce_pit`].
        as_of_bound: Option<u64>,
    },
    /// Row filter over an input plan.
    Filter {
        /// Predicate expression (opaque string at this floor).
        predicate: String,
        /// Child plan.
        input: Arc<LogicalPlanStub>,
    },
    /// Column projection over an input plan.
    Project {
        /// Projected column names.
        columns: Vec<String>,
        /// Child plan.
        input: Arc<LogicalPlanStub>,
    },
    /// Row limit over an input plan.
    Limit {
        /// Maximum rows to return.
        n: u64,
        /// Child plan.
        input: Arc<LogicalPlanStub>,
    },
    /// Equi-join of two subplans.
    ///
    /// A join whose children (or the join itself) lack a covering as-of bound
    /// is a classic look-ahead / future-label leak — see
    /// [`LogicalPlanStub::is_pit_safe`].
    Join {
        /// Left input.
        left: Arc<LogicalPlanStub>,
        /// Right input.
        right: Arc<LogicalPlanStub>,
        /// Join key expression (opaque string at this floor).
        on: String,
    },
    /// Explicit point-in-time temporal filter inserted by [`enforce_pit`].
    AsOfFilter {
        /// As-of upper bound in UTC microseconds since Unix epoch.
        as_of_micros: u64,
        /// Child plan (already rewritten recursively).
        input: Arc<LogicalPlanStub>,
    },
}

impl LogicalPlanStub {
    /// Construct an unbounded scan (`as_of_bound = None`).
    pub fn scan(source: impl Into<String>) -> Self {
        Self::Scan {
            source: source.into(),
            as_of_bound: None,
        }
    }

    /// Construct a scan already bound to `as_of_micros`.
    pub fn scan_as_of(source: impl Into<String>, as_of_micros: u64) -> Self {
        Self::Scan {
            source: source.into(),
            as_of_bound: Some(as_of_micros),
        }
    }

    /// Wrap `self` in a [`Filter`](LogicalPlanStub::Filter).
    pub fn filter(self, predicate: impl Into<String>) -> Self {
        Self::Filter {
            predicate: predicate.into(),
            input: Arc::new(self),
        }
    }

    /// Wrap `self` in a [`Project`](LogicalPlanStub::Project).
    pub fn project(self, columns: Vec<String>) -> Self {
        Self::Project {
            columns,
            input: Arc::new(self),
        }
    }

    /// Wrap `self` in a [`Limit`](LogicalPlanStub::Limit).
    pub fn limit(self, n: u64) -> Self {
        Self::Limit {
            n,
            input: Arc::new(self),
        }
    }

    /// Equi-join `self` (left) with `right` on `on`.
    pub fn join(self, right: Self, on: impl Into<String>) -> Self {
        Self::Join {
            left: Arc::new(self),
            right: Arc::new(right),
            on: on.into(),
        }
    }

    /// Wrap `self` in an [`AsOfFilter`](LogicalPlanStub::AsOfFilter).
    pub fn as_of_filter(self, as_of_micros: u64) -> Self {
        Self::AsOfFilter {
            as_of_micros,
            input: Arc::new(self),
        }
    }

    /// Collect every scan's `as_of_bound` in pre-order (for tests / audits).
    pub fn scan_as_of_bounds(&self) -> Vec<Option<u64>> {
        let mut out = Vec::new();
        self.collect_as_of_bounds(&mut out);
        out
    }

    fn collect_as_of_bounds(&self, out: &mut Vec<Option<u64>>) {
        match self {
            Self::Scan { as_of_bound, .. } => out.push(*as_of_bound),
            Self::Filter { input, .. }
            | Self::Project { input, .. }
            | Self::Limit { input, .. }
            | Self::AsOfFilter { input, .. } => input.collect_as_of_bounds(out),
            Self::Join { left, right, .. } => {
                left.collect_as_of_bounds(out);
                right.collect_as_of_bounds(out);
            },
        }
    }

    /// Count of [`AsOfFilter`](LogicalPlanStub::AsOfFilter) nodes in the tree.
    pub fn as_of_filter_count(&self) -> usize {
        match self {
            Self::AsOfFilter { input, .. } => 1 + input.as_of_filter_count(),
            Self::Scan { .. } => 0,
            Self::Filter { input, .. }
            | Self::Project { input, .. }
            | Self::Limit { input, .. } => input.as_of_filter_count(),
            Self::Join { left, right, .. } => {
                left.as_of_filter_count() + right.as_of_filter_count()
            },
        }
    }

    /// `true` if every scan has a non-`None` `as_of_bound`.
    pub fn all_scans_bounded(&self) -> bool {
        self.scan_as_of_bounds().iter().all(|b| b.is_some())
    }

    /// `true` if this node or any descendant is an [`AsOfFilter`].
    pub fn contains_as_of_filter(&self) -> bool {
        self.as_of_filter_count() > 0
    }

    /// `true` if every leaf [`Scan`](LogicalPlanStub::Scan) has an
    /// [`AsOfFilter`](LogicalPlanStub::AsOfFilter) ancestor.
    ///
    /// Partial coverage (e.g. as-of only on the left of a join) returns
    /// `false` — fail-closed against look-ahead through the uncovered side.
    pub fn every_scan_under_as_of_filter(&self) -> bool {
        self.every_scan_under_as_of_filter_rec(false)
    }

    fn every_scan_under_as_of_filter_rec(&self, under_as_of: bool) -> bool {
        match self {
            Self::AsOfFilter { input, .. } => input.every_scan_under_as_of_filter_rec(true),
            Self::Scan { .. } => under_as_of,
            Self::Filter { input, .. }
            | Self::Project { input, .. }
            | Self::Limit { input, .. } => input.every_scan_under_as_of_filter_rec(under_as_of),
            Self::Join { left, right, .. } => {
                left.every_scan_under_as_of_filter_rec(under_as_of)
                    && right.every_scan_under_as_of_filter_rec(under_as_of)
            },
        }
    }

    /// PIT safety (fail-closed): every scan bounded **and** every scan path
    /// covered by an [`AsOfFilter`] ancestor.
    ///
    /// A future join of unbounded scans without an as-of filter fails this
    /// check. So does a join where only one side sits under an as-of wrapper
    /// (look-ahead / label-leak risk on the uncovered side).
    pub fn is_pit_safe(&self) -> bool {
        self.all_scans_bounded() && self.every_scan_under_as_of_filter()
    }
}

/// Point-in-time enforcement rewrite (`P4-QM-05` floor).
///
/// Recursively walks `plan`, sets `as_of_bound = Some(as_of_micros)` on every
/// [`LogicalPlanStub::Scan`], **strips** nested
/// [`AsOfFilter`](LogicalPlanStub::AsOfFilter) nodes (stale / partial wrappers),
/// and **inserts** a single root as-of wrapper so the rewritten plan is
/// [`is_pit_safe`](LogicalPlanStub::is_pit_safe).
///
/// This is the normative stand-in for a future DataFusion optimizer rule.
pub fn enforce_pit(plan: LogicalPlanStub, as_of_micros: u64) -> LogicalPlanStub {
    // `rewrite_children` always strips AsOfFilter nodes, so the root wrapper
    // is the sole as-of coverage after rewrite.
    let rewritten = rewrite_children(plan, as_of_micros);
    debug_assert!(
        !matches!(rewritten, LogicalPlanStub::AsOfFilter { .. }),
        "rewrite_children must strip AsOfFilter; root re-inserts the enforced bound"
    );
    LogicalPlanStub::AsOfFilter {
        as_of_micros,
        input: Arc::new(rewritten),
    }
}

fn rewrite_children(plan: LogicalPlanStub, as_of_micros: u64) -> LogicalPlanStub {
    match plan {
        LogicalPlanStub::Scan { source, .. } => LogicalPlanStub::Scan {
            source,
            as_of_bound: Some(as_of_micros),
        },
        LogicalPlanStub::Filter { predicate, input } => LogicalPlanStub::Filter {
            predicate,
            input: Arc::new(rewrite_children(Arc::unwrap_or_clone(input), as_of_micros)),
        },
        LogicalPlanStub::Project { columns, input } => LogicalPlanStub::Project {
            columns,
            input: Arc::new(rewrite_children(Arc::unwrap_or_clone(input), as_of_micros)),
        },
        LogicalPlanStub::Limit { n, input } => LogicalPlanStub::Limit {
            n,
            input: Arc::new(rewrite_children(Arc::unwrap_or_clone(input), as_of_micros)),
        },
        LogicalPlanStub::Join { left, right, on } => LogicalPlanStub::Join {
            left: Arc::new(rewrite_children(Arc::unwrap_or_clone(left), as_of_micros)),
            right: Arc::new(rewrite_children(Arc::unwrap_or_clone(right), as_of_micros)),
            on,
        },
        LogicalPlanStub::AsOfFilter { input, .. } => {
            // Collapse nested / stale wrappers; outer `enforce_pit` re-inserts
            // a single root wrapper with the enforced bound.
            rewrite_children(Arc::unwrap_or_clone(input), as_of_micros)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unbounded_scan_has_none_as_of_before_rewrite() {
        let plan = LogicalPlanStub::scan("features.equity_bars");
        match &plan {
            LogicalPlanStub::Scan { as_of_bound, .. } => assert!(as_of_bound.is_none()),
            other => panic!("expected Scan, got {other:?}"),
        }
        assert!(!plan.all_scans_bounded());
        assert!(!plan.every_scan_under_as_of_filter());
        assert!(!plan.is_pit_safe());
    }

    #[test]
    fn enforce_pit_binds_every_scan_and_inserts_as_of_filter() {
        // Negative: without rewrite, as_of stays None (look-ahead risk).
        let unbound = LogicalPlanStub::scan("features.equity_bars")
            .project(vec!["close".into()])
            .limit(10);
        assert_eq!(unbound.scan_as_of_bounds(), vec![None]);
        assert!(!unbound.all_scans_bounded());
        assert!(!unbound.contains_as_of_filter());

        let as_of = 1_700_000_000_000_000_u64;
        let bound = enforce_pit(unbound, as_of);
        assert_eq!(bound.scan_as_of_bounds(), vec![Some(as_of)]);
        assert!(bound.all_scans_bounded());
        assert_eq!(bound.as_of_filter_count(), 1);
        assert!(bound.every_scan_under_as_of_filter());
        assert!(bound.is_pit_safe());
        match &bound {
            LogicalPlanStub::AsOfFilter {
                as_of_micros,
                input,
            } => {
                assert_eq!(*as_of_micros, as_of);
                match input.as_ref() {
                    LogicalPlanStub::Limit { .. } => {},
                    other => panic!("expected Limit under AsOfFilter, got {other:?}"),
                }
            },
            other => panic!("expected AsOfFilter root, got {other:?}"),
        }
    }

    #[test]
    fn enforce_pit_rewrites_nested_filter_scan() {
        let as_of = 1_650_000_000_000_000_u64;
        let plan = LogicalPlanStub::scan("market.quotes").filter("bid > 0");
        assert_eq!(plan.scan_as_of_bounds(), vec![None]);

        let rewritten = enforce_pit(plan, as_of);
        match &rewritten {
            LogicalPlanStub::AsOfFilter {
                as_of_micros,
                input,
            } => {
                assert_eq!(*as_of_micros, as_of);
                match input.as_ref() {
                    LogicalPlanStub::Filter { predicate, input } => {
                        assert_eq!(predicate, "bid > 0");
                        match input.as_ref() {
                            LogicalPlanStub::Scan {
                                source,
                                as_of_bound,
                            } => {
                                assert_eq!(source, "market.quotes");
                                assert_eq!(*as_of_bound, Some(as_of));
                            },
                            other => panic!("expected Scan under Filter, got {other:?}"),
                        }
                    },
                    other => panic!("expected Filter under AsOfFilter, got {other:?}"),
                }
            },
            other => panic!("expected AsOfFilter, got {other:?}"),
        }
        assert!(rewritten.is_pit_safe());
    }

    #[test]
    fn enforce_pit_overrides_existing_as_of() {
        let stale = LogicalPlanStub::scan_as_of("features.x", 100);
        let rewritten = enforce_pit(stale, 999);
        assert_eq!(rewritten.scan_as_of_bounds(), vec![Some(999)]);
        match rewritten {
            LogicalPlanStub::AsOfFilter { as_of_micros, .. } => assert_eq!(as_of_micros, 999),
            other => panic!("expected AsOfFilter, got {other:?}"),
        }
    }

    #[test]
    fn enforce_pit_overrides_existing_as_of_filter_wrapper() {
        let stale_wrapped = LogicalPlanStub::scan_as_of("features.x", 100).as_of_filter(100);
        assert_eq!(stale_wrapped.as_of_filter_count(), 1);
        assert!(stale_wrapped.is_pit_safe());

        let rewritten = enforce_pit(stale_wrapped, 999);
        assert_eq!(rewritten.scan_as_of_bounds(), vec![Some(999)]);
        assert_eq!(
            rewritten.as_of_filter_count(),
            1,
            "nested wrappers collapse to one"
        );
        match &rewritten {
            LogicalPlanStub::AsOfFilter {
                as_of_micros,
                input,
            } => {
                assert_eq!(*as_of_micros, 999);
                match input.as_ref() {
                    LogicalPlanStub::Scan {
                        as_of_bound,
                        source,
                    } => {
                        assert_eq!(source, "features.x");
                        assert_eq!(*as_of_bound, Some(999));
                    },
                    other => panic!("expected Scan under root AsOfFilter, got {other:?}"),
                }
            },
            other => panic!("expected AsOfFilter root, got {other:?}"),
        }
        assert!(rewritten.is_pit_safe());
    }

    #[test]
    fn enforce_pit_collapses_deeply_nested_as_of_filters() {
        let as_of = 7_777_u64;
        // Triple-nested stale wrappers around a Project → Filter → Scan tree.
        let plan = LogicalPlanStub::scan("deep.source")
            .filter("ok")
            .project(vec!["c".into()])
            .as_of_filter(1)
            .as_of_filter(2)
            .as_of_filter(3);
        assert_eq!(plan.as_of_filter_count(), 3);
        assert_eq!(plan.scan_as_of_bounds(), vec![None]);

        let rewritten = enforce_pit(plan, as_of);
        assert_eq!(rewritten.as_of_filter_count(), 1);
        assert_eq!(rewritten.scan_as_of_bounds(), vec![Some(as_of)]);
        assert!(rewritten.every_scan_under_as_of_filter());
        assert!(rewritten.is_pit_safe());

        match &rewritten {
            LogicalPlanStub::AsOfFilter {
                as_of_micros,
                input,
            } => {
                assert_eq!(*as_of_micros, as_of);
                match input.as_ref() {
                    LogicalPlanStub::Project { columns, input } => {
                        assert_eq!(columns, &vec!["c".to_string()]);
                        match input.as_ref() {
                            LogicalPlanStub::Filter { predicate, input } => {
                                assert_eq!(predicate, "ok");
                                match input.as_ref() {
                                    LogicalPlanStub::Scan { as_of_bound, .. } => {
                                        assert_eq!(*as_of_bound, Some(as_of));
                                    },
                                    other => panic!("expected Scan, got {other:?}"),
                                }
                            },
                            other => panic!("expected Filter, got {other:?}"),
                        }
                    },
                    other => panic!("expected Project under AsOfFilter, got {other:?}"),
                }
            },
            other => panic!("expected AsOfFilter root, got {other:?}"),
        }
    }

    #[test]
    fn enforce_pit_strips_mid_tree_as_of_under_join() {
        let as_of = 5_555_u64;
        // AsOf only on the left child — partial coverage before rewrite.
        let left = LogicalPlanStub::scan("features.left")
            .filter("x > 0")
            .as_of_filter(111);
        let right = LogicalPlanStub::scan("labels.right");
        let plan = left.join(right, "id");

        assert_eq!(plan.as_of_filter_count(), 1);
        assert!(!plan.every_scan_under_as_of_filter());
        assert!(!plan.is_pit_safe());

        let rewritten = enforce_pit(plan, as_of);
        assert_eq!(rewritten.as_of_filter_count(), 1);
        assert_eq!(
            rewritten.scan_as_of_bounds(),
            vec![Some(as_of), Some(as_of)]
        );
        assert!(rewritten.every_scan_under_as_of_filter());
        assert!(rewritten.is_pit_safe());

        match &rewritten {
            LogicalPlanStub::AsOfFilter { input, .. } => match input.as_ref() {
                LogicalPlanStub::Join { left, right, on } => {
                    assert_eq!(on, "id");
                    // Mid-tree AsOfFilter on left must be gone.
                    assert_eq!(left.as_of_filter_count(), 0);
                    assert_eq!(right.as_of_filter_count(), 0);
                    assert_eq!(left.scan_as_of_bounds(), vec![Some(as_of)]);
                    assert_eq!(right.scan_as_of_bounds(), vec![Some(as_of)]);
                },
                other => panic!("expected Join under AsOfFilter, got {other:?}"),
            },
            other => panic!("expected AsOfFilter root, got {other:?}"),
        }
    }

    #[test]
    fn enforce_pit_deep_nesting_project_filter_limit() {
        let as_of = 42_u64;
        let plan = LogicalPlanStub::scan("t")
            .filter("a = 1")
            .project(vec!["a".into(), "b".into()])
            .limit(5);
        let rewritten = enforce_pit(plan, as_of);
        assert!(rewritten.is_pit_safe());
        assert_eq!(rewritten.scan_as_of_bounds(), vec![Some(as_of)]);
        assert_eq!(rewritten.as_of_filter_count(), 1);

        // Structure: AsOfFilter → Limit → Project → Filter → Scan(bound).
        match &rewritten {
            LogicalPlanStub::AsOfFilter {
                as_of_micros,
                input,
            } => {
                assert_eq!(*as_of_micros, as_of);
                match input.as_ref() {
                    LogicalPlanStub::Limit { n, input } => {
                        assert_eq!(*n, 5);
                        match input.as_ref() {
                            LogicalPlanStub::Project { columns, input } => {
                                assert_eq!(columns, &vec!["a".to_string(), "b".to_string()]);
                                match input.as_ref() {
                                    LogicalPlanStub::Filter { predicate, input } => {
                                        assert_eq!(predicate, "a = 1");
                                        match input.as_ref() {
                                            LogicalPlanStub::Scan {
                                                source,
                                                as_of_bound,
                                            } => {
                                                assert_eq!(source, "t");
                                                assert_eq!(*as_of_bound, Some(as_of));
                                            },
                                            other => panic!("expected Scan, got {other:?}"),
                                        }
                                    },
                                    other => panic!("expected Filter, got {other:?}"),
                                }
                            },
                            other => panic!("expected Project, got {other:?}"),
                        }
                    },
                    other => panic!("expected Limit, got {other:?}"),
                }
            },
            other => panic!("expected AsOfFilter root, got {other:?}"),
        }
    }

    #[test]
    fn partial_as_of_on_one_join_side_is_unsafe_negative() {
        // Both scans pre-bound, but AsOfFilter covers only the left arm —
        // fail-closed: uncovered right path is look-ahead risk.
        let left = LogicalPlanStub::scan_as_of("features.a", 50).as_of_filter(50);
        let right = LogicalPlanStub::scan_as_of("labels.b", 50);
        let plan = left.join(right, "instrument_id");

        assert!(plan.all_scans_bounded());
        assert!(plan.contains_as_of_filter());
        assert!(
            !plan.every_scan_under_as_of_filter(),
            "right scan must not count as covered"
        );
        assert!(
            !plan.is_pit_safe(),
            "partial as-of coverage on a join must fail PIT safety"
        );
    }

    #[test]
    fn future_join_without_as_of_is_unsafe_negative() {
        // Negative: joining "future" features to labels without an as-of filter
        // is a classic look-ahead leak — not PIT-safe until enforce_pit.
        let future_join = LogicalPlanStub::scan("features.future_returns")
            .join(LogicalPlanStub::scan("labels.outcome"), "instrument_id");

        assert_eq!(future_join.scan_as_of_bounds(), vec![None, None]);
        assert!(!future_join.all_scans_bounded());
        assert!(!future_join.contains_as_of_filter());
        assert!(!future_join.every_scan_under_as_of_filter());
        assert!(
            !future_join.is_pit_safe(),
            "future join without as-of must fail PIT safety"
        );

        let as_of = 1_710_000_000_000_000_u64;
        let rewritten = enforce_pit(future_join, as_of);
        assert_eq!(
            rewritten.scan_as_of_bounds(),
            vec![Some(as_of), Some(as_of)]
        );
        assert_eq!(rewritten.as_of_filter_count(), 1);
        assert!(rewritten.every_scan_under_as_of_filter());
        assert!(rewritten.is_pit_safe());

        match &rewritten {
            LogicalPlanStub::AsOfFilter {
                as_of_micros,
                input,
            } => {
                assert_eq!(*as_of_micros, as_of);
                match input.as_ref() {
                    LogicalPlanStub::Join { left, right, on } => {
                        assert_eq!(on, "instrument_id");
                        assert_eq!(left.scan_as_of_bounds(), vec![Some(as_of)]);
                        assert_eq!(right.scan_as_of_bounds(), vec![Some(as_of)]);
                    },
                    other => panic!("expected Join under AsOfFilter, got {other:?}"),
                }
            },
            other => panic!("expected AsOfFilter root, got {other:?}"),
        }
    }

    #[test]
    fn prebound_join_without_as_of_filter_remains_unsafe() {
        // Scans may already carry bounds, but missing covering AsOfFilter is
        // still a fail-closed negative until enforce_pit.
        let plan = LogicalPlanStub::scan_as_of("features.a", 10)
            .join(LogicalPlanStub::scan_as_of("labels.b", 10), "k");
        assert!(plan.all_scans_bounded());
        assert!(!plan.contains_as_of_filter());
        assert!(!plan.is_pit_safe());

        let rewritten = enforce_pit(plan, 99);
        assert!(rewritten.is_pit_safe());
        assert_eq!(rewritten.scan_as_of_bounds(), vec![Some(99), Some(99)]);
    }
}
