# Wave 5 — Controlled Execution

**Maps to v1.0 phases:** P7 (Controlled Execution)
**Business outcome:** full retail product. Live order submission through the deterministic execution boundary.
**Duration:** ~46 days.
**Date:** 2026-07-26

---

## Objective

Live order submission through the deterministic execution boundary. The Wave 4 paper broker becomes a real broker adapter. The Wave 4 reconciliation loop becomes load-bearing. The Wave 4 audit-as-write-path portfolio becomes the live portfolio.

**This wave does not open on schedule pressure. It opens when Waves 0 through 4 have passed their gates.** Live trading is the point where a defect stops being a feature gap and starts being a way to lose real money.

## Entry Criteria

Wave 4 exit gate.

## Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P7-QM-01 | QM | `BrokerGateway` trait, `SubmissionResult` including the `Unknown` variant | 4 | M |
| P7-QM-02 | QM | Idempotency key generation, collision detection, replay safety | 4 | M |
| P7-QM-03 | QM | Reconciliation: delta computation, quarantine, resolution, ledger append | 7 | L |
| P7-QM-04 | QM | First broker adapter (Alpaca), full lifecycle including cancel and replace | 8 | L |
| P7-QM-05 | QM | Broker error taxonomy: `BrokerErrorCode` with permanence bit, `Connecting` state, classify `MARKET_CLOSED` before `AUTH` (from OpenAlice, clean-room) | 4 | M |
| P7-QM-06 | QM | Fill stream handling, partial fills, out-of-order events | 5 | M |
| P7-SS-01 | SS | Step-up authorization, live execution enablement, privilege audit | 5 | M |
| P7-SS-02 | SS | Emergency stop: cancel-all, flatten, halt automation, with a tested runbook | 4 | M |
| P7-EX-01 | EX | Order ticket, intent review, risk-decision display, approval flow | 6 | M |
| P7-OD-01 | OD | Execution runbooks, incident procedures, reconciliation playbook | 3 | M |

## The Three Outcomes of Submission (P7-QM-01)

`BrokerGateway::submit` can return three outcomes, not two. The third is the one that separates a working execution system from a dangerous one.

```rust
pub enum SubmissionResult {
    Accepted { broker_order_id: BrokerOrderId, accepted_at: OffsetDateTime },
    Rejected { reason: BrokerRejection },
    /// The request may or may not have reached the broker. The system
    /// MUST NOT retry blindly. It MUST reconcile, and until reconciliation
    /// completes the intent is quarantined: no further intents for the
    /// same instrument are admitted.
    Unknown { idempotency_key: IdempotencyKey, last_known_state: LocalOrderState },
}
```

**The naive response to `Unknown` — retrying — is how duplicate orders happen.** The correct path: quarantine the instrument, call `BrokerGateway::reconcile`, compare the returned delta against local belief, append the resolution to the audit ledger, and only then release the quarantine. If reconciliation itself fails, the instrument stays quarantined and the user is notified. Failing loudly and stopping is correct here; guessing is not.

## The Broker Error Taxonomy (P7-QM-05)

From OpenAlice's UTA protocol (AGPL, clean-room reimplementation per ADR-024). The design encodes production-incident knowledge that took OpenAlice a year of incidents to get right:

```rust
pub enum BrokerErrorCode {
    Config,        // permanent — disables the account
    Auth,          // permanent — disables the account
    Network,       // transient — auto-recover
    Exchange,      // transient — venue rejected, do not retry blindly
    MarketClosed,  // transient — expected, not a failure
    Connecting,    // "data pending, retry shortly" — NOT a failure
    Unknown,
}
// permanent = code == Config || Auth
```

Two non-obvious rules make this work:

1. **The `Connecting` state is the non-obvious one.** It means *the account is mid-connect or mid-recovery*. A read during `Connecting` returns immediately without blocking, **without counting as a health failure**, and without disabling the account. Systems that lack this state either block their whole read path on a slow venue handshake or spuriously degrade account health during normal reconnects.

2. **Classify `MarketClosed` before `Auth`.** Venues return 403 for both. Getting the order wrong means a routine after-hours read permanently disables a healthy account.

## The Live Gate (Two Independent Acts)

Per the existing README, reaching live execution requires BOTH:

- The server was started with `PRISMATIK_LIVE_TRADING=I_ACCEPT_FULL_RESPONSIBILITY_FOR_LIVE_TRADING`
- The session request repeats that exact phrase in `live_confirm`

Two independent, deliberate acts: one by the operator at deploy time, one per session. This is the product's liability posture as much as its safety posture. Wave 5 adds step-up authorization on top, with both events recorded in the audit ledger.

## Definition of Done

| # | Criterion | Verified by |
|---|---|---|
| 1 | A submission timeout produces `Unknown`, quarantines the instrument, and does not retry | Committed negative test |
| 2 | Reconciliation resolves a divergence and appends the resolution to the ledger | Test |
| 3 | A failed reconciliation leaves the instrument quarantined and notifies the user | Committed negative test |
| 4 | The same idempotency key submitted twice produces exactly one broker order, property tested | Property test |
| 5 | Live execution requires explicit enablement plus step-up authorization, both audited | Audit-log inspection |
| 6 | Emergency stop halts all automation and cancels working orders within a stated time bound | Timed test |
| 7 | Every risk denial is recorded with the portfolio snapshot at evaluation time | Audit-log inspection |
| 8 | Chaos test: broker disconnection mid-submission is handled without duplicate orders | Chaos test in CI |

## The `Unknown` Path and Why It Carries 7 Days

`P7-QM-03` is the item that separates a working execution system from a dangerous one, and it is the one most likely to be under-tested because it is hard to provoke. The response: build a broker simulator that produces timeouts, duplicate acknowledgements, out-of-order fills, and post-submission disconnections on demand, and make it part of CI. This is why `P7-QM-03` carries seven days.

The broker simulator built here is also the Wave 4 paper broker's test harness — it pays back across both waves.

## Risks

| Risk | Response |
|---|---|
| The `Unknown` path is under-tested because it is hard to provoke | Build the broker simulator described above. Make it part of CI. This is `P7-QM-03` and it is why that item carries seven days. |
| Broker API change mid-wave (Alpaca options GTC, Coinbase Sept 2026 INTX→Deribit migration) | Pin adapter versions; monitor provider changelogs as a continuous-track activity. Coinbase international derivatives migrate Sept 9, 2026 to a Deribit-powered gateway — plan for this if international perps are in scope. |
| OAuth-only auth path for some brokers | Alpaca MCP V2 supports OAuth (best OAuth-first execution path); IBKR MCP is official; Coinbase Developer Platform MCP is OAuth-based. API-key fallback for brokers without OAuth. |

## Commercial Metrics

| Metric | Wave 5 target |
|---|---|
| Paying customers | 500+ cumulative |
| Revenue | Funds team growth |
| Retention (30-day) | >70% |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26*
