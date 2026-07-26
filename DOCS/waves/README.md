# PRISMATIK Wave Delivery Plan — Index

**Companion to:** `PRISMATIK_Enterprise_Overview_Vision_and_Wave_Plan.md` (strategic narrative), `PRISMATIK_Unified_Solution_Architecture_v1.0.md` (architecture of record)
**Date:** 2026-07-26

This directory breaks the Wave Delivery Plan (Part IV of the Enterprise Overview) into one file per wave. Each wave file is self-contained: objective, entry criteria, duration, work items with day estimates and confidence bands, definition-of-done (verified by named artifacts, not by assertion), the hypothesis the wave tests (where applicable), and wave-specific risks.

## The Eight Waves

| Wave | File | Maps to v1.0 phases | Business outcome | Days (focused solo) |
|---|---|---|---|:---:|
| **0 — Foundation** | [Wave_0_Foundation.md](Wave_0_Foundation.md) | P0 | The non-negotiable floor: deterministic, evidence-chained, sandboxed shell. Not a product. | 24–72 |
| **1 — MVP (Crypto Intelligence)** | [Wave_1_MVP_Crypto_Intelligence.md](Wave_1_MVP_Crypto_Intelligence.md) | P1 | First sellable product. **Tests whether the wedge is real.** | 56 (→ first ship at ~80) |
| **2 — Multi-Asset Intelligence** | [Wave_2_Multi_Asset_Intelligence.md](Wave_2_Multi_Asset_Intelligence.md) | P2 + P3 | The differentiating product (equity + filings + options). | 116 |
| **3 — Quantitative Platform** | [Wave_3_Quantitative_Platform.md](Wave_3_Quantitative_Platform.md) | P4 + P5 + P5.5 | The defensible research platform. | 153 |
| **4 — Research-to-Decision Loop** | [Wave_4_Research_to_Decision_Loop.md](Wave_4_Research_to_Decision_Loop.md) | P6 | Complete research-to-decision loop. | 48 |
| **5 — Controlled Execution** | [Wave_5_Controlled_Execution.md](Wave_5_Controlled_Execution.md) | P7 | Full retail product (live trading). | 46 |
| **6 — Enterprise & Team** | [Wave_6_Enterprise_Team.md](Wave_6_Enterprise_Team.md) | P8 | Second business. | 84 |
| **7 — Ecosystem** | [Wave_7_Ecosystem.md](Wave_7_Ecosystem.md) | P9 | Third business (marketplace + published contracts). | 41 |

**Total: ~616 engineering-days.** At 2–3 focused days/week (realistic solo capacity), ~5–7 years to Wave 7. The wave framework tests the commercial hypothesis at Wave 1 (~80 days under Option B) rather than at Wave 3 (~335 days under Option A).

## How to Read These Files

| Audience | Read |
|---|---|
| Executive, board, partner | `PRISMATIK_Enterprise_Overview_Vision_and_Wave_Plan.md` Parts I–II only |
| Engineering leadership, lead developers | This README, then Wave 0, then Wave 1 (the MVP), then the wave your team owns |
| Program management | This README, then each wave's "Definition of Done" and "Risks" sections |
| Architects | Each wave's work items + cross-references to v1.0 Architecture sections |

## Cross-Cutting Tracks (every wave)

Four activities run across every wave and are budgeted at a percentage rather than as discrete items, because scheduling them as tasks guarantees they get cut.

| Track | Budget | Content |
|---|---|---|
| Security maintenance | 5% of every wave | Advisory response, dependency updates, pinned bumps, threat-model revision |
| Documentation | 8% of every wave | ADRs, API docs, runbooks, model cards, the workbook |
| Test debt | 7% of every wave | Property-test expansion, DST seed-corpus growth, conformance-vector additions |
| Refactoring | 5% of every wave | Boundary corrections, crate splits, naming consistency |

**25% overhead.** Not padding — a plan that omits it produces the same total with worse quality and a demoralizing final third. The 616-day total already includes this overhead inside per-item estimates.

## Estimate Confidence Bands

| Band | Meaning |
|---|---|
| **H** | High. Well-understood work, similar to past work. ±20%. |
| **M** | Medium. Known approach, unknown friction. ±50%. |
| **L** | Low. Research or integration with unfamiliar external system. 2×–3× possible. |

## The Six Tracks

Work items carry stable IDs of the form `P{wave}-{track}-{n}` (e.g. `P1-DP-04`). These are the primary keys across the issue tracker, commit messages, ADRs, and the project workbook. They do not change when work moves between waves.

| Track | Code | Scope |
|---|---|---|
| Determinism and Kernel | DK | Clock, entropy, artifacts, manifest, audit ledger, identity, calendar |
| Data and Providers | DP | Provider ports, ingestion, storage, lineage, feature store |
| Quant and Models | QM | Strategy IR, backtest, indicators, TSFM, calibration, drift |
| AI Plane | AI | AI router, LM Studio integration, MCP, analyst orchestration |
| Experience | EX | Design system, Svelte components, charts, workspaces, IPC bindings |
| Security and Supply Chain | SS | Keys, capabilities, sandbox, signing, SBOM, gates |
| Operations and Delivery | OD | CI, observability, packaging, updater, docs, runbooks |

## Wave Review Procedure

Every wave closes with a written review, not a feeling. The review is `docs/gates/wave-N.md` containing:

1. Each exit criterion, its verification artifact, pass/fail.
2. Every waiver, with reason, risk accepted, and the wave by which it must be resolved. A waiver without a resolution wave is not a waiver; it is a silent scope cut.
3. Estimate vs actual per work item — the only way the estimates improve.
4. Items deferred to a later wave with original identifiers.
5. New risks discovered, added to the Enterprise Overview Part VI register.
6. A go/no-go decision on the next wave, with reasoning recorded.

**Recalibrate after Wave 0.** Estimates are anchored on judgment, not measured throughput. Wave 0 produces the first real velocity data. Multiply every subsequent estimate by the Wave 0 actual/estimate ratio and update these files.

## Explicitly Not Now

Recording what is deliberately excluded is as valuable as recording what is included. See `PRISMATIK_Enterprise_Overview_Vision_and_Wave_Plan.md` Part VIII for the full list (causal market graph, market digital twin, federated strategy learning, confidential computing, multi-agent research council, Pine Script compatibility, TradingView Advanced Charts, NautilusTrader adoption, Temporal orchestration below Wave 6, additional TSFM families, mobile client, io_uring on networking path, full thread-per-core runtime migration).

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26*
