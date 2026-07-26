# Wave 4 — Research-to-Decision Loop

**Maps to v1.0 phases:** P6 (Portfolio, Journal, and Paper Trading)
**Business outcome:** the complete research-to-decision loop. Portfolio accounting on the audit ledger, the learning loop, and a paper broker that behaves like a real one.
**Duration:** ~48 days.
**Date:** 2026-07-26

---

## Objective

Make the audit ledger the portfolio write path (with state as a projection rebuilt by replay), wire the full sixteen-check pre-trade catalogue with fixed ordering, ship a paper broker with realistic fill/latency/rejection behavior, and build the continuous-reconciliation loop that will become load-bearing in Wave 5.

By the end of Wave 4, a user can run paper-trading sessions against realistic broker behavior with every order journaled before and after submission, every risk denial recorded with the portfolio snapshot at evaluation time, and the journal linking each thesis to the evidence chain that produced it.

## Entry Criteria

Wave 3 exit gate.

## Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P6-DK-01 | DK | Audit ledger becomes the portfolio write path; projections as read models (**evaluate TigerBeetle Rust client, shipped April 2026**) | 6 | L |
| P6-DK-02 | DK | Projection rebuild from ledger replay, startup reconciliation | 4 | M |
| P6-DK-03 | DK | Divergence detection between projection and ledger, alert on mismatch | 3 | M |
| P6-QM-01 | QM | Portfolio model: positions, lots, cost basis (`avg_cost_source` provenance), realized and unrealized PnL | 6 | M |
| P6-QM-02 | QM | Risk measures: exposure, concentration, correlation, scenario analysis | 6 | M |
| P6-QM-03 | QM | `RiskPolicy` and the full 16-check pre-trade catalogue with fixed ordering | 6 | M |
| P6-QM-04 | QM | Paper broker with realistic fill, latency, and rejection behavior | 5 | M |
| P6-QM-05 | QM | **`prismatik-reconciliation`: continuous sync loop, divergence classification, auto-heal, halt on irreconcilable** (from nofx, clean-room) | 6 | L |
| P6-EX-01 | EX | Portfolio workspace: exposures, risk panel, scenario runner | 6 | M |
| P6-EX-02 | EX | Journal: entry capture, thesis linkage, outcome tagging, review workflow (**3-layer memory loop from prism-insight, clean-room**) | 5 | M |
| P6-QM-06 | QM | Post-trade learning: realized versus expected, thesis validation reporting | 4 | M |
| P6-QM-07 | QM | TimesFM onboarded to the registry | 2 | M |

## The Audit-as-Write-Path Inversion (P6-DK-01)

This is the barter-rs-derived inversion (v1.0 §14.2) and the highest-risk architectural commitment in the plan. Conventional design: the application mutates state, then writes an audit record describing what it did. The audit record can be forgotten, can disagree with state, and is trusted only because everyone agrees to trust it.

**PRISMATIK design: the audit entry *is* the write.** State is a projection.

```
                      ┌──────────────────────┐
  command  ──────────▶│  Policy + Risk gate  │
                      └───────────┬──────────┘
                                  │ (allowed | denied, BOTH are appended)
                      ┌───────────▼──────────┐
                      │   AUDIT LEDGER       │  ◀── the only write path
                      │   append-only Merkle │
                      └───────────┬──────────┘
                                  │ replay
              ┌───────────────────┼───────────────────┐
              ▼                   ▼                   ▼
      PortfolioState        OrderState          UI Replica
       (projection)        (projection)        (projection)
```

Three consequences follow, all good. (1) A denied action is recorded with the same fidelity as an allowed one — most audit systems record what happened; this one records what was *attempted*, which is where the security signal lives. (2) Any projection can be rebuilt from the ledger — a corrupted portfolio table is a recoverable inconvenience rather than data loss. (3) A projection that disagrees with the ledger is *detectable*, because the ledger is authoritative and replay is cheap.

**2026 technology update:** TigerBeetle shipped a Rust client in April 2026, making it directly usable from PRISMATIK's Rust core for this pattern. TigerBeetle is purpose-built financial accounting OLTP, Jepsen-passing, designed for "the next 30 years of OLTP." It is a stronger substrate than a hand-rolled Merkle ledger and is the recommended evaluation target for `P6-DK-01`.

## The Reconciliation Loop (P6-QM-05)

The Wave 4 paper broker is the proving ground for the reconciliation loop that becomes load-bearing in Wave 5 live execution. The design comes from nofx (AGPL, clean-room reimplementation):

- **Per-venue incremental fill sync** from a persisted watermark with multi-method symbol detection (commission detection misses VIP/BNB-discount/0-fee trades, so multiple methods are combined).
- **Wrapped in a continuous loop** with exponential-doubling backoff capped at 5min, resetting to base interval on success.
- **Divergence classification:** snapshot → diff against journal → classify divergence → auto-heal the benign cases → halt only on the genuinely irreconcilable.
- **`RealizedPnL == 0` as the open/close discriminator** — venue-truth signal, not trade direction.

Halt-the-session-on-ambiguity (the v0.1 behavior) remains correct as a *fallback*. The Wave 4 production shape is the continuous sync loop with auto-heal.

## The Journal's 3-Layer Memory Loop (P6-EX-02)

The journal is not a notepad. It is the platform's learning loop, designed after prism-insight's pattern (AGPL, clean-room):

- **Layer 1 (0–7 days):** detailed records.
- **Layer 2 (8–30 days):** summarized — `"{sector} + {trigger} → {action} → {result}"`.
- **Layer 3 (31+ days):** intuitions — `"{condition} = {principle}"`, with hit-rate stats.

Feedback loop: trigger-type win rate (>65% with n≥3 encourages the next buy; <35% suppresses); score adjustment (−3 to +3 based on historical performance); last 3 trades per ticker injected to prevent repeated mistakes; Layer 3 patterns injected as reference context.

## The Sixteen Pre-Trade Checks (P6-QM-03)

Fixed evaluation order: cheapest and most deterministic first, all HardDeny checks before any SoftWarn. Two reasons. First, a deny should not depend on how far evaluation happened to get. Second, a fixed order makes the audit record comparable across runs, which matters when someone asks why an identical intent was denied on Tuesday and allowed on Wednesday.

| Check | Severity |
|---|---|
| `max_position`, `max_premium`, `max_account_loss`, `sector_concentration`, `account_permissions`, `market_status`, `stale_data`, `duplicate_order`, `adjusted_contract`, `calendar_mismatch`, `model_suppressed` | HardDeny |
| `correlation_cluster`, `event_proximity`, `spread_width`, `liquidity_volume`, `open_interest` | SoftWarn |

All checks recorded, including those that passed. The evidence chain for a denial includes the portfolio snapshot at evaluation time, so a denial is explainable months later without reconstructing state.

## Definition of Done

| # | Criterion | Verified by |
|---|---|---|
| 1 | Portfolio state rebuilt from a full ledger replay matches the live projection exactly | Property test |
| 2 | A deliberately corrupted projection is detected at startup and rebuilt | Committed negative test |
| 3 | Cash plus market value plus realized PnL reconciles against the ledger, property tested | Property test |
| 4 | Every pre-trade check evaluates in the declared fixed order, with all results recorded including passes | Audit-log inspection |
| 5 | Paper broker rejections and partial fills are exercised and handled | Test suite |
| 6 | Journal entries link to the evidence chain that produced the thesis | UI review |
| 7 | Wallet-sourced cost basis is visibly marked in the UI wherever a P&L figure derived from it is displayed | UI review |
| 8 | Reconciliation classifies divergence correctly across 20+ seeded cases | Test suite |
| 9 | Irreconcilable divergence halts the session and notifies the user | Chaos test |

## Risks

| Risk | Response |
|---|---|
| `P6-DK-01` is the inversion described in architecture §14.2 and is the highest-risk item in the wave | Build the ledger write path alongside the conventional path first, compare them continuously for a full wave, and remove the conventional path only once divergence has been zero across the wave. Cutting over on day one is the version of this that goes wrong. |
| Reconciliation state machine has more edge cases than the nofx reference suggests | The nofx production pattern is well-documented; start there. Build a broker simulator (becomes load-bearing in Wave 5) that produces timeouts, duplicate acks, out-of-order fills, post-submission disconnects on demand. |

## Commercial Metrics

| Metric | Wave 4 target |
|---|---|
| Paying customers | 350+ cumulative |
| Retention (30-day) | >65% |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26*
