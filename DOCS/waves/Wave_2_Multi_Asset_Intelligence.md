# Wave 2 — Multi-Asset Intelligence

**Maps to v1.0 phases:** P2 (Equity, Filings, Macro, Real Symbology) + P3 (Options Intelligence)
**Business outcome:** the differentiating product. The thing that justifies the architecture.
**Duration:** ~116 days (Wave 2A ~54 days + Wave 2B ~62 days).
**Date:** 2026-07-26

---

## Objective

Extend to US equities with regulatory and macro context, make bitemporal symbology load-bearing (equity history is wrong without it), and ship the differentiating **options intelligence wedge**. By the end of Wave 2, PRISMATIK is the product the architecture was built for.

## Entry Criteria

Wave 1 exit gate. **Under Option B, `P0-DK-06` bitemporal symbology moves here and is mandatory, not deferred further.**

## Why This Wave Matters

Wave 1 tested the wedge with crypto. Wave 2 lands the differentiator: US equities with regulatory context (SEC filings, insider transactions, institutional ownership, CFTC positioning), macro indicators (FRED, OECD, IMF), and the **options intelligence wedge** — unusual-whales-class flow, dealer exposure, volatility surfaces, strategy construction with payoff diagrams. This is what the moat protects.

The wave also makes the **bitemporal symbology** load-bearing. A 2021 backtest that resolves the ticker "FB" using a 2026 mapping table silently produces wrong results, and no seed, hash, or signature detects it. Identifiers must be attributes with validity intervals, never keys. This is the concrete implementation of the temporal knowledge graph that earlier documents filed as "future advancement" — it is not future, it is a Wave 2 requirement because equity backtests are wrong without it.

---

## Wave 2A — Equity, Filings, Macro, Real Symbology (~54 days)

### Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P2-DK-01 | DK | Bitemporal symbology production hardening, OpenFIGI ingestion, identity chain | 6 | M |
| P2-DK-02 | DK | Corporate-action ledger, event types including `OptionAdjustment`, conflict retention | 5 | M |
| P2-DK-03 | DK | Read-time adjustment factor computation from the ledger (never destructive rewrite) | 4 | M |
| P2-DK-04 | DK | Calendar artifacts for NYSE, NASDAQ, ARCA, BATS with QuantLib cross-validation | 4 | M |
| P2-DP-01 | DP | SEC EDGAR adapter: submissions, company facts, filing index, full-text search | 7 | M |
| P2-DP-02 | DP | Filing parser: 13F, Forms 3/4/5, 8-K item extraction, SC 13D and 13G | 8 | L |
| P2-DP-03 | DP | CFTC Commitments of Traders adapter and weekly schedule | 4 | M |
| P2-DP-04 | DP | FRED adapter, series metadata, release calendar, vintage handling | 4 | M |
| P2-DP-05 | DP | Equity OHLCV provider integration behind the `Provider` port | 4 | M |
| P2-DP-06 | DP | Feature store: `FeatureView`, `observation_delay`, offline point-in-time join | 6 | M |
| P2-DP-07 | DP | Point-in-time property test suite | 2 | H |
| P2-EX-01 | EX | Institutional intelligence: 13F views, ownership, insider activity | 6 | M |
| P2-EX-02 | EX | Event and catalyst engine with the earnings and macro calendar | 5 | M |
| P2-EX-03 | EX | Universal instrument workspace generalized across asset classes | 5 | M |
| P2-EX-04 | EX | Perspective integration for streaming analytical grids | 4 | M |

### The Non-Negotiable Design Rule: `observation_delay`

The `FeatureView.observation_delay` field is the single highest-value line in the feature store. It is the difference between a feature store and a look-ahead bug generator. 13F holdings, Commitments of Traders reports, and restated fundamentals all have publication lags measured in days to weeks, and a naive `event_time <= as_of` join treats them as observable at the moment they describe rather than the moment they were published. Encoding the delay in the view definition means every consumer inherits the correct behavior.

Property test: for a random view, entity, and instant, assert that no returned value has `event_time + observation_delay > as_of`. This test has caught the bug in every feature store the author has seen it applied to.

---

## Wave 2B — Options Intelligence (~62 days, the differentiating wedge)

### Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P3-DP-01 | DP | Unusual Whales adapter behind the `Provider` port, entitlement mapping | 6 | M |
| P3-DP-02 | DP | Options chain normalization, OCC symbology, contract identity | 5 | M |
| P3-DP-03 | DP | Flow observation ingestion at print level, ClickHouse for cloud profiles | 6 | L |
| P3-DP-04 | DP | Historical chain and IV surface storage, compression strategy | 5 | M |
| P3-QM-01 | QM | `prismatik-quant-kernel`: pricing, greeks, IV solve, selected RustQuant modules | 7 | M |
| P3-QM-02 | QM | QuantLib conformance sidecar, 1e-8 tolerance gate on pricing and greeks | 5 | L |
| P3-QM-03 | QM | Flow classification: directional, hedging, closing, spread-leg detection | 8 | L |
| P3-QM-04 | QM | Flow clustering and aggregation | 5 | L |
| P3-QM-05 | QM | Trade-quality score with a disclosed and versioned formula | 4 | M |
| P3-EX-01 | EX | Chain explorer with dense grid, greeks columns, liquidity shading | 6 | M |
| P3-EX-02 | EX | Volatility lab: term structure, skew, IV rank, surface via wgpu | 7 | L |
| P3-EX-03 | EX | Strategy constructor with payoff diagram and break-even analysis | 6 | M |
| P3-EX-04 | EX | Dealer exposure views | 4 | M |
| P3-SS-01 | SS | `adjusted_contract` pre-trade check wired to the corporate-action ledger | 2 | H |

### The Wedge

The options intelligence surface is the differentiating product. Three things make it defensible:

1. **Flow classification with explicit confidence and an "unclassified" outcome used freely.** A classifier that always decides is a classifier that is often wrong. The trade-quality score formula is disclosed and versioned alongside every score.
2. **QuantLib conformance at 1e-8 relative tolerance on pricing and greeks.** A 500-case grid cross-validates the Rust `prismatik-quant-kernel` against the QuantLib sidecar oracle. Drift is a CI failure.
3. **The `adjusted_contract` HardDeny pre-trade check.** Adjusted option contracts (from OCC memos after mergers and special dividends) have non-standard deliverables and multipliers. Backtests that treat them as ordinary contracts produce plausible and completely wrong results. The check is wired to the corporate-action ledger and is a hard-deny condition for automation.

---

## Definition of Done

| # | Criterion | Verified by |
|---|---|---|
| 1 | A 2015 to 2026 equity universe resolves every ticker correctly through renames, splits, and mergers | Hand-checked set of 50 known identity events |
| 2 | Corporate actions are applied at read time; the raw series is unmodified | Byte comparison of raw Parquet before and after a split ingestion |
| 3 | No feature value is returned whose `event_time + observation_delay` exceeds `as_of` | Property test, 10k cases |
| 4 | 13F holdings are not observable before their filing date, only their period end | Targeted test on a known filing |
| 5 | Calendar artifacts show zero disagreement with QuantLib across all four venues | Generator report |
| 6 | Adjusted option contracts are flagged in the corporate-action ledger; automation hard-denied | Test against a known OCC memo |
| 7 | Pricing and greeks match QuantLib within 1e-8 relative across a 500-case grid | Conformance report |
| 8 | Flow classification decisions expose their evidence and their confidence | UI review against 20 hand-labelled prints |
| 9 | The IV surface renders at 60 fps with a Canvas fallback that is functional | Benchmark plus fallback test |
| 10 | Trade-quality score formula and version appear alongside every score | UI review |

## Risks

| Risk | Response |
|---|---|
| `P2-DP-02` filing parsing is the largest low-confidence item in the wave | Scope to 13F and Form 4 only for the gate. 8-K item extraction and 13D/G move to a Wave 2.5 candidate list. Partial filing coverage is a feature gap; wrong filing parsing is a correctness failure. |
| SEC rate limits and user agent requirements | Implement the declared user agent and the ten requests per second ceiling in the adapter, governed by the same `BudgetGovernor`. |
| Equity price data licensing | Resolve the provider and licensing question **before** Wave 2 opens, not during. This is a Wave 1 background task. |
| Flow classification is genuinely hard and partially unknowable | The architecture already names this in v0.1 §35.5. Ship classification with explicit confidence and an "unclassified" outcome that is used freely. A classifier that always decides is a classifier that is often wrong. |
| Options data costs are material and recurring | Model the cost into pricing before Wave 2 opens. |
| ClickHouse introduction adds operational surface | Cloud and enterprise profiles only. Desktop stays on Parquet and DuckDB, per architecture §15.2. |

## Commercial Metrics

| Metric | Wave 2 target |
|---|---|
| Paying customers | 50+ cumulative |
| Revenue | Covers data costs |
| Retention (30-day) | >50% |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26*
