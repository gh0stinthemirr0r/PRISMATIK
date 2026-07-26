# PRISMATIK — Enterprise Overview, Vision, and Wave Delivery Plan

**Document:** `PRISMATIK_Enterprise_Overview_Vision_and_Wave_Plan.md`
**Companion to:** `PRISMATIK_Unified_Solution_Architecture_v1.0.md` (architecture of record), the v1.1 trio (reference harvest), `PRISMATIK_v1.2_Reference_Consolidation_and_AI_Stack.md` (reference + AI stack), `PRISMATIK_Phased_Implementation_Plan_v1.0.md` (phased plan this document supersedes for sequencing)
**Organization:** Mythos Systems
**Author:** Aaron Stovall
**Version:** 1.0
**Date:** 2026-07-26
**Status:** Proposed strategic plan of record
**Classification:** Internal engineering and executive reference

---

## 0. How to Read This Document

This is the document for an audience that needs to understand what PRISMATIK is, why it exists, what it will look like when shipped, and the order in which it gets built. It is not the architecture specification — that lives in `PRISMATIK_Unified_Solution_Architecture_v1.0.md` (hereafter "v1.0 Architecture"). It is not the work-item-level plan — that lives in `PRISMATIK_Phased_Implementation_Plan_v1.0.md`. This document sits above both and translates them into a strategic narrative.

**Reading paths:**

| Audience | Read |
|---|---|
| Executive, board, partner, acquirer | Part I (Vision) and Part II (Strategic Position) only |
| Engineering leadership, lead developers | Parts I, III, IV in full; skim V |
| Architects, senior engineers building this | Part III, then v1.0 Architecture in full |
| Product, design, GTM | Part I, Part II, Part IV.3 (Wave 1 MVP) |
| Program management | Part IV in full; Part VI (risk); Part VII (metrics) |

**Relationship to the phased plan.** The v1.0 Phased Implementation Plan describes ten phases (P0–P9) over ~616 engineering-days, with each phase a self-contained exit gate. This document **reframes those same phases as four delivery waves**, each wave a coherent business outcome that ends with a shippable, sellable artifact. The wave framing does not replace the phase framing — it overlays it. The phases say what to build; the waves say why it ships in that order and what the customer gets.

| Wave | Maps to phases | Business outcome | Day target (solo, focused) |
|---|---|---|---|
| **Wave 0 — Foundation** | P0 (Foundation) | The non-negotiable floor: deterministic, evidence-chained, sandboxed shell. Not a product. | ~24–72 |
| **Wave 1 — MVP (Crypto Intelligence)** | P1 (Crypto MVP) | First sellable product. Tests whether the wedge is real. | ~80–128 |
| **Wave 2 — Multi-Asset Intelligence** | P2 (Equity/Filings) + P3 (Options) | The differentiating product. The thing that justifies the architecture. | ~196–244 |
| **Wave 3 — Quantitative Platform** | P4 (Strategy/Backtest) + P5 (TSFM/Calibration) + P5.5 (Analog) | The defensible research platform. | ~335–397 |
| **Wave 4 — Research-to-Decision Loop** | P6 (Portfolio/Journal/Paper) | The complete research-to-decision loop. | ~445 |
| **Wave 5 — Controlled Execution** | P7 (Live trading) | Full retail product. | ~491 |
| **Wave 6 — Enterprise & Team** | P8 (Enterprise) | Second business. | ~575 |
| **Wave 7 — Ecosystem** | P9 (Marketplace) | Third business. | ~616 |

---

# Part I. Vision

## 1. The Product in One Sentence

**PRISMATIK is a self-hosted, multi-asset market intelligence and quantitative decision operating system whose central capability is the ability to prove what it said, why it said it, and what it knew at the moment it said it.**

Everything else — data breadth, chart quality, model sophistication, AI integration, execution speed — is a delivery vehicle for that one capability. Data breadth is commodity and can be bought. Chart rendering is commodity and can be adopted. Forecasting models are commodity and are given away by Amazon, Google, and academic labs. What cannot be bought or downloaded is a platform where a backtest run in July 2026 can be re-executed byte-for-byte in July 2029, on a different machine, with every input traceable to a provider endpoint and a retrieval timestamp.

That capability is the product. Everything else is a delivery vehicle for it.

## 2. The World We Are Building Against

The state of retail and prosumer market-intelligence tooling in 2026 is the backdrop against which PRISMATIK's wedge becomes legible. The category has two poles.

**The opaque AI-product pole.** A wave of LLM-driven trading agents and "AI analysts" has shipped in 2025–2026. Most are adaptive systems where a language model selects tools at execution time. They are powerful and unverifiable. They surface a confidence percentage with no calibration record behind it. They show a forecast with no uncertainty band. They produce a recommendation with no resolvable evidence chain. When they are wrong, they cannot explain why in a way that lets the user learn. They are also, increasingly, in regulatory tension: the SEC's June 2025 Predictive Data Analytics rule (Release 34-97990) requires broker-dealers and RIAs to identify, eliminate, or neutralize conflicts of interest in "covered technology" used in "investor interactions," and the EU AI Act's transparency obligations land August 2, 2026. Products that surface AI-generated recommendations without disclosure, calibration, or audit are squarely in the crosshairs.

**The legacy quant-platform pole.** On the other end are the established research platforms — QuantConnect LEAN, OpenBB, vectorbt, pybroker. These are powerful and have access to data PRISMATIK will not have for years. They are also server-first or browser-first, governed by AGPL or commercial licenses that foreclose embedding, deterministic-but-not-reproducible-across-deploys (because calendars and codebooks ship as live code), and built around Python runtimes that bring ambient nondeterminism into the deterministic path. They are research tools, not decision operating systems. None of them can produce a verifiable research artifact that a third party validates without installing the product.

PRISMATIK occupies the space between these poles. It is more rigorous than the AI-product pole (deterministic, evidence-chained, calibrated, audited) and more accessible than the legacy quant-platform pole (self-hosted desktop first, local-first privacy posture, governed AI integration, not a server farm). The wedge is not "better data" or "better models" — it is **verifiability made structural rather than aspirational**.

## 3. The Three Customer Outcomes

A product is what it does for someone. PRISMATIK does three things for three increasingly-sophisticated customers, and the waves of this plan correspond to when each becomes possible.

### 3.1 The Analyst Outcome (Wave 1+)
*"Show me what is happening, with the evidence behind it, in a tool that respects my privacy."*

The Wave 1 customer is a crypto analyst or trader who wants market intelligence — global macro context, asset profiles, watchlists, charts, scanners — that doesn't lie about where its numbers came from. Every rendered number resolves a complete evidence chain to a raw record with a provider identity and a retrieval timestamp. Stale data is visibly marked. Rate limits are surfaced honestly, with exact countdowns rather than indeterminate spinners. AI-assisted analysis runs locally via LM Studio, with the user's positions and journal never leaving the machine unless the user explicitly escalates a single hard synthesis to a hosted model with consent.

This is a real product. It is not a commodity — the evidence-chaining discipline alone distinguishes it from every retail tool on the market, and the local-first privacy posture distinguishes it from every AI-product competitor that ships the user's data to a hosted API by default.

### 3.2 The Researcher Outcome (Wave 3+)
*"Let me do rigorous quantitative research whose results I can defend and reproduce."*

The Wave 3 customer is a serious quantitative researcher. They want to author strategies (visually, in a DSL, in Rust, or by emitting StrategyIR from a Python research sidecar), backtest them with honest cost modeling, walk-forward them with purged/embargoed splits and multiple-testing disclosure, calibrate forecasts with conformal prediction, run Monte Carlo simulations over regime-plausible paths, and search historical analogs against TSFM embeddings. Every result is captured in a signed reproducibility manifest that a third party can verify with a standalone tool and no PRISMATIK installation.

This is the defensible research platform. The differentiator is not the strategy language or the backtest speed — it is the byte-identical-reproduction guarantee under pinned seeds and pinned artifact sets, and the calibrated-probability guarantee that turns model output from prophecy into honest uncertainty.

### 3.3 The Decision-Operator Outcome (Wave 4+)
*"Let me make decisions with the platform's research behind me, and execute them safely."*

The Wave 4 customer is a decision operator. They want a portfolio that reconciles against the audit ledger, a journal whose entries link to the evidence chain that produced the thesis, a paper broker that behaves like a real one, and eventually (Wave 5) live execution through a deterministic boundary that quarantines any ambiguous order state rather than guessing. Risk is enforced structurally — the OMS accepts only `RiskApproved<OrderIntent>` that no code path outside the risk kernel can construct. Emergency stop is a tested runbook. Reconciliation is a continuous loop, not a startup step.

This is the full retail product. The differentiator is the safety posture: every order is journaled before and after submission, every risk denial is recorded with the portfolio snapshot at evaluation time, and any ambiguous order state halts the session rather than guessing.

## 4. The Seven Invariants (the engineering constitution)

Everything in the architecture exists to make these seven statements true by construction, not by review. An invariant without a CI gate is an aspiration. Each invariant below has its gate named in v1.0 Architecture §27.

| # | Invariant | One-sentence statement |
|---|---|---|
| **I1** | Evidence Precedes Conclusion | No conclusion surface renders without a resolvable evidence chain terminating in raw records with provider identity and retrieval timestamps. |
| **I2** | Probability, Never Prophecy | Model output crosses the IPC boundary only as a distribution, interval, calibrated probability, ranking, or scenario set — never as a scalar point estimate without an accompanying uncertainty band and calibration record. |
| **I3** | Determinism Under Seed | Identical inputs plus identical seed plus identical pinned artifact set produce byte-identical outputs, on any machine, at any future date, until artifacts are explicitly rotated. |
| **I4** | Structural Capability Enforcement | What a strategy, plugin, tool, or model may do is enforced by the runtime through capability tokens, not by convention, documentation, or code review. |
| **I5** | Point-in-Time Correctness | No feature, model input, or backtest observation may include information with an `event_time` later than the simulated clock, including information laundered through a pretrained model's corpus. |
| **I6** | Fail Closed on Staleness | Automation that depends on timely data halts rather than degrades. Staleness is a hard deny, not a warning, anywhere an order intent is in scope. |
| **I7** | Tamper-Evident Audit | Sensitive operations write to an append-only, hash-chained ledger where any retroactive edit is detectable without trusting the storage layer. |

These are not marketing claims. They are testable properties with named verification artifacts. They are also the reason Wave 0 exists: most of them cannot be retrofitted cheaply, and the cost of retrofitting rises multiplicatively with every line of code written against a foundation that doesn't have them.

## 5. The Five-Year Trajectory

Where this product goes over the next five years, in three acts.

**Act I (Years 1–2): Build the analyst and researcher product.** Waves 0 through 3. A self-hosted multi-asset intelligence platform with calibrated forecasting, rigorous backtesting, and verifiable research artifacts. The moat is structural verifiability and local-first AI. The customer is the serious individual analyst and the boutique research shop. Revenue is per-seat desktop licensing plus optional data subscriptions routed through the provider plane.

**Act II (Years 2–3): Add execution and the team product.** Waves 4 through 6. The complete research-to-decision loop with paper and live trading. Multi-user team deployments with RBAC, on-prem enterprise installs, air-gapped operation. The customer expands to small hedge funds, family offices, prop shops, and the research arms of larger institutions. Revenue adds per-team licensing and enterprise terms.

**Act III (Years 4–5): Open the ecosystem.** Wave 7 and beyond. Publish the plugin SDK, chart contracts, indicator trait, and manifest schema under Apache-2.0. Third parties build, sign, and publish plugins. Third parties verify PRISMATIK research bundles without installing PRISMATIK. The manifest schema becomes a citable artifact in academic and regulatory contexts. The moat deepens as the ecosystem develops — competitors can copy features but not the body of reproducible, third-party-verifiable research artifacts the community has produced.

This trajectory is a hypothesis, not a forecast. The wave framework exists so the hypothesis can be tested at every wave boundary rather than at the end.

---

# Part II. Strategic Position

## 6. What PRISMATIK Is Not

Honest positioning requires saying what the product is not.

**Not a high-frequency trading system.** The architecture optimizes for verifiability and reproducibility, not microsecond latency. The hot path is async Rust on Tokio, not FPGA or kernel-bypass. PRISMATIK is for human-paced and strategy-paced decisions, not for market-making or statistical arbitrage at microsecond scale. The hftbacktest microstructure lab (v0.4 §28) is a research tool inside the platform, not a deployment target.

**Not a brokerage.** PRISMATIK executes through the customer's own brokerage accounts (Alpaca, IBKR, Coinbase). It does not custody assets, does not hold customer funds, does not extend credit. MPC custody (Fireblocks, Coinbase Prime) is an institutional Wave 6 integration surface, not a core capability.

**Not a hosted SaaS.** The desktop profile is the primary deployment. The team cloud and enterprise profiles exist, but they are self-hosted (customer's AWS/GCP/on-prem) — PRISMATIK is not running a multi-tenant SaaS. This is a deliberate privacy and liability posture, not a limitation to be optimized away later.

**Not a prediction market.** The platform produces calibrated probability distributions, not scalar predictions. It does not claim to forecast prices. It claims to honestly represent uncertainty. A user who confuses the two will be disappointed, and the disclosure registry exists to prevent that confusion.

**Not open-core with a proprietary cloud.** The trusted core is proprietary. The plugin SDK, chart contracts, indicator trait, and manifest schema are Apache-2.0. There is no "PRISMATIK Cloud" running a hosted version. Customers run it themselves.

## 7. The Defensible Moats

A moat is a structural advantage a competitor cannot copy by spending money. PRISMATIK has four.

### 7.1 The Verifiability Moat
The Determinism Kernel, the Merkle audit ledger, the signed reproducibility manifest, and the dual-signature scheme together produce a property no competitor in the retail/prosumer category offers: a research artifact that a third party can verify without installing PRISMATIK. This is not a feature; it is a body of accumulated, citable, reproducible work. Each passing month adds to it. A competitor copying the feature ships with an empty corpus.

### 7.2 The Local-First AI Moat
The evidence graph contains the user's positions, journal, and thesis notes — the single most privacy-sensitive data in their financial life. Sending that to a hosted API by default is the largest objection an enterprise buyer will raise. PRISMATIK's LM Studio integration with loopback-enforced egress makes the privacy claim structural: the local provider is *incapable* of reaching the network. The hosted escalation tier is a per-call consent surface, not a default. Competitors who built their AI plane against hosted APIs cannot retrofit this without breaking their existing UX.

### 7.3 The Calibration Moat
Conformal prediction with Adaptive Conformal Inference (ACI) as the default for time series, regime-conditioned (Mondrian) calibration, per-regime coverage reporting, and the baseline ladder enforcement — these together produce the property that no forecast reaches the UI without a calibration record, and that a model is never promoted unless it beats lower-complexity baselines on both discrimination and calibration. This is the concrete engineering of Invariant I2. It is the difference between "70% confidence" as a marketing number and "70% interval that contained the realization 72% of the time over the last 500 observations, and 51% in high-volatility regimes" as a factual statement. A competitor copying this ships with no calibration data and no baseline ladder.

### 7.4 The Clean-Room Discipline Moat
This is the most counterintuitive moat and worth stating plainly. The corpus of reference projects (the 21 in `reference/`) is read for design inspiration, not embedded. Every pattern taken is written up as an ADR that names the observed source and asserts clean-room reimplementation. This discipline produces two outcomes: (1) the codebase is provably independent of its AGPL/GPL sources, which is a defensible IP posture in an acquisition or funding diligence; and (2) the engineering team develops a deeper understanding of the design space than competitors who vendored code without understanding it. The moat is the understanding, not the code.

## 8. Competitive Landscape (and what to take from each)

PRISMATIK does not compete head-on with established platforms on their strengths. It competes on verifiability and privacy where they are structurally weak.

| Competitor | Their strength | What we take from them (clean-room or permissive) | Where we do not compete |
|---|---|---|---|
| **QuantConnect LEAN** | C# backtest engine maturity, 300+ hedge funds using it | Validation oracle pattern (v0.4 §28) | Their engine scale; their data breadth |
| **OpenBB** | 32-provider data integration | Provider coverage catalog (reimplemented against chosen providers); the TET Fetcher pattern is reimplementable | Their data platform breadth; AGPL forecloses embedding |
| **vectorbt Pro** | Vectorized Python backtest speed | Speed benchmark to beat | Speed at the expense of determinism |
| **pybroker** | Walk-forward + BCa bootstrap | Methodology reference (López de Prado); Commons Clause forecloses commercial use | Their Numba engine |
| **NautilusTrader** | Rust/PyO3 v2 trading engine | Catalog/storage pattern (Parquet + object_store + DataFusion), 128-bit precision feature flag | Their live-trading engine scope; LGPL implications unresolved |
| **OctoBot** | Mature crypto bot, tentacle plugin system | Plugin discovery metadata pattern | Their Python plugin substrate; LGPL at library level |
| **OpenAlice** | UTA broker abstraction, error taxonomy | The five broker-design facts (clean-room ADRs 024/025/026) | Their TS/Electron stack; AGPL forecloses embedding |
| **nofx** | Live reconciliation, regime classification | The continuous-reconciliation-loop design (clean-room) | Their Go architecture |
| **Kronos** | Financial TSFM (AAAI 2026) | Model artifact + tokenizer (MIT) | Point forecasting crowded; we use tokenizer as embedding + path synthesizer |
| **AI-product competitors** (AI-Trader, ai-trading-claude, ai-market-maker, prism-insight, etc.) | Agentic UX, agent topologies | AnalystScope common/private split (ai-trading-claude, MIT); Risk Guard veto placement (ai-market-maker, clean-room); journal-loop design (prism-insight, clean-room) | Adaptive-agent runtime model — we are procedural-by-construction |

## 9. The Pricing Hypothesis

Pricing is a hypothesis to test, not a decision to commit to. The current hypothesis, to be revisited at each wave boundary:

| Tier | Customer | Hypothesis | Wave |
|---|---|---|---|
| **Personal Desktop** | Individual analyst/trader/researcher | One-time license + optional annual updates; bundled with the user's own data subscriptions routed through the provider plane | 1+ |
| **Pro Desktop** | Power user, multi-asset, calibration, research bundle export | Higher one-time + annual; research bundle verification service included | 3+ |
| **Team Cloud** | Small fund/family office, multi-user, on customer's infra | Per-seat monthly/annual + infrastructure | 6+ |
| **Enterprise On-Prem** | Larger institution, air-gapped, custom integrations | Annual contract + services | 6+ |
| **Plugin Marketplace** | Third-party plugin developers | Revenue share on paid plugins; free plugins free | 7+ |

The Wave 1 MVP exists in significant part to test the Personal Desktop hypothesis. If it fails, the wave framework is designed to pivot to equity/options (Wave 2) as the alternative wedge before committing to the full multi-year build.

---

# Part III. Granular Architecture Overview

This part is the architect's view of the system: what the planes are, what each one owns, how they connect, and where the load-bearing decisions live. It is the strategic translation of v1.0 Architecture §9–§28. For normative type definitions, see v1.0 Architecture directly.

## 10. The Four-Plane Model

PRISMATIK is organized as four planes, each with a distinct trust boundary and a distinct rate of change.

```
┌───────────────────────────────────────────────────────────────────────────────┐
│  EXPERIENCE PLANE                                                             │
│  Svelte 5 / SvelteKit · Tauri 2 (patched ≥2.12) · PRISMATIK Design System     │
│  Lightweight Charts · Perspective · ECharts · wgpu Renderer                   │
│  Typed IPC surface generated by tauri-specta (no hand-written types)          │
│  Trust: UNTRUSTED. Holds no secrets. Renders untrusted content.               │
└──────────────────────────────────┬────────────────────────────────────────────┘
                                   │  generated bindings + capability policy
┌──────────────────────────────────▼────────────────────────────────────────────┐
│  TRUSTED CORE (Rust)                                                          │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────────┐  │
│  │  DETERMINISM KERNEL   (Wave 0, load-bearing, non-negotiable)            │  │
│  │  Clock · Entropy · Calendar · Symbology · Codebook · Manifest · Audit   │  │
│  │  Ledger. Every other subsystem borrows time, entropy, and identity      │  │
│  │  from here. Reproducibility claims are only as strong as the weakest     │  │
│  │  ambient-nondeterminism leak in the process.                            │  │
│  └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                               │
│  Domain:        Strategy IR + Runtime · Risk Policy · Execution Boundary      │
│  Portfolio:     Feature Store · Model Registry · AI Tool Gateway              │
│  Evidence:      Evidence and Lineage Plane · Provider Gateway + Cost Governor │
│  Extension:     Plugin Host (wasmtime, capability-scoped) · MCP Server/Client │
│  Trust: TRUSTED. Owns keys, policy, persistence, and the Determinism Kernel.  │
└────────┬──────────────────┬───────────────────┬───────────────────────────────┘
         │                  │                   │
┌────────▼────────┐ ┌───────▼────────┐ ┌────────▼────────────────────────────┐
│ DATA PLANE      │ │ COMPUTE PLANE  │ │ SIDECAR PLANE (isolated processes)  │
│ SQLite          │ │ Backtest       │ │ CCXT connector gateway              │
│ DuckDB          │ │ Monte Carlo    │ │ QuantLib conformance oracle         │
│ DataFusion      │ │ TSFM runtime   │ │ Qlib research bridge                │
│ Parquet/Arrow   │ │ Indicator core │ │ MAPIE/crepes calibration            │
│ LanceDB         │ │ Calibration    │ │ Kronos TSFM worker                  │
│ (Profile-       │ │ Drift monitor  │ │ LM Studio (local inference)         │
│  scoped: PG,    │ │                │ │ Signed MCP servers                  │
│  ClickHouse,    │ │                │ │ Trust: UNTRUSTED. Output treated    │
│  NATS for cloud)│ │                │ │ as provider data — provenance-      │
│                 │ │                │ │ stamped, never authoritative.       │
└─────────────────┘ └────────────────┘ └─────────────────────────────────────┘
```

The hard rule, stated once and enforced everywhere: **a process that can hold a credential cannot execute third-party code, and a process that executes third-party code cannot hold a credential.** Every box satisfies this. The sidecar plane is a hard process boundary; nothing in it can write to the data plane, hold a broker credential, or emit an authoritative audit event.

## 11. The Determinism Kernel (Wave 0, non-negotiable)

The single largest addition in v1.0 Architecture and the reason Wave 0 exists. It is the substrate on which every reproducibility claim rests.

**The problem it solves.** A Rust application of this size has at least seven doors through which nondeterminism enters: `Instant::now` and `SystemTime::now` anywhere in the process or transitive dependencies; `getrandom`/`getentropy` at the libc level; `HashMap`/`HashSet` iteration order (randomized for DoS resistance); task scheduling order in a multi-threaded tokio runtime; floating-point reduction order under Rayon parallelism; network and disk timing affecting retry/timeout/cache eviction paths; and silently upgraded reference data (calendars, tokenizer codebooks). A seed in a manifest addresses door one partially and nothing else. The Determinism Kernel addresses all seven.

**Core traits:** `Clock` (SystemClock, SimulatedClock, FrozenClock), `Entropy` (splittable by label, order-independent), `DetHasher`/`DetMap`/`DetSet` (deterministic hasher replacing `RandomState`), `DeterminismContext` (the bundle every deterministic subsystem receives), `PinnedArtifactSet` (the complete set of external inputs whose change would alter results — if it's not in here, it cannot influence a run).

**Three enforcement mechanisms, all in CI:**
1. **Workspace lint** (`clippy.toml` `disallowed-methods`/`disallowed-types`) banning `Instant::now`, `SystemTime::now`, `rand::thread_rng`, default `HashMap`/`HashSet` outside the kernel crate, with a capped (<10 entries) reviewed allowlist.
2. **Deterministic simulation testing** via madsim, trace-comparison verification following the S2 storage team's byte-for-byte-diff approach across reruns of the same seed.
3. **Golden manifest corpus** — committed manifests with known outputs, regenerated in the same commit when output legitimately changes, with mandatory changelog entry. This is the mechanism that catches door seven (silently upgraded reference data).

**What ships in Wave 0 floor vs remainder:** the floor ships `Clock`/`Entropy`/`DetMap`/`AssetId` (the multiplicative-retrofit items), the determinism lints and grep gate, the Tauri shell with capability policy, the tauri-specta bindings pipeline, and the CI skeleton. The remainder (Merkle audit ledger, manifest builder, dual signatures, calendar artifacts, full DST suite) is additive rather than invasive and interleaves across Waves 1–2.

## 12. The Evidence and Lineage Plane (Wave 1+)

The concrete engineering of Invariant I1. Every conclusion surface (opportunity card, forecast, score, ranking, alert) implements a `Concludes` trait that requires a non-empty evidence slice, a contradictions slice, a blind-spots slice, and an `as_of` timestamp.

**The two non-obvious rules:**
- **Contradictions are required, not optional.** An opportunity with zero contradictions after a genuine search is reported as "none found," which the UI renders differently from "did not search" — and both are weaker claims than "none exist."
- **Provider identity flows into the evidence graph.** A bar from the fallback provider is not the same evidence as one from the primary. A backtest that silently mixed them is not reproducible, and the reproducibility manifest records the per-request provider actually used.

The `Lineage.invalidation_hash` includes the `PinnedArtifactSet` — meaning calendar, codebook, and model changes invalidate downstream records, which is exactly the class of silent breakage the Determinism Kernel exists to prevent.

## 13. The Audit Ledger as Write Path (Wave 6, but designed for in Wave 0)

This is the barter-rs-derived inversion that makes Invariant I7 cheap instead of expensive, and it is the highest-risk architectural commitment in the entire plan.

**Conventional design:** the application mutates state, then writes an audit record describing what it did. The audit record can be forgotten, can disagree with state, and is trusted only because everyone agrees to trust it.

**PRISMATIK design:** the audit entry *is* the write. State is a projection. The command goes through policy + risk gate → audit ledger append (the only write path) → portfolio/order/UI projections rebuilt by replay. Three consequences: (1) a denied action is recorded with the same fidelity as an allowed one — most audit systems record what happened, this one records what was *attempted*, which is where the security signal lives; (2) any projection can be rebuilt from the ledger — a corrupted portfolio table is a recoverable inconvenience, not data loss; (3) a projection that disagrees with the ledger is *detectable*, because the ledger is authoritative and replay is cheap.

The 2026 technology update strengthens this materially: **TigerBeetle shipped a Rust client in April 2026**, making it directly usable from PRISMATIK's core for the audit-ledger-as-write-path pattern. TigerBeetle is purpose-built financial accounting OLTP, Jepsen-passing, designed for the next 30 years of OLTP — a stronger substrate than generic event sourcing. The Wave 6 implementation should evaluate TigerBeetle vs a hand-rolled Merkle ledger.

## 14. The Provider Plane and Cost Governance (Wave 1)

Every external dependency — market data, AI inference, broker APIs — enters through one plane with one port and one governor.

**The port** (`Provider`) declares capabilities, entitlements, cost-per-call (computed before the call), and health. Entitlements are enforced *before* a request is made, so an unentitled request is a local error rather than a remote 403 that burns quota.

**The governor** (`BudgetGovernor`) is GCRA via the `governor` crate, chosen over a token bucket because GCRA is allocation-free and answers "when will the next permit be available" exactly — which the rate-budget UI needs and a token bucket cannot supply without extra bookkeeping. The `AdmissionDecision` enum has four variants: `Admit`, `Defer { retry_at }` (exact time, UI shows countdown), `BudgetExhausted { resets_at }` (caller degrades gracefully), `NotEntitled` (local failure, no request made).

**The multi-source fusion pattern** (from adata, clean-room): sources never raise; they return empty results. `FailoverTrigger::Empty` is the one people forget — an empty 200 is the common real-world failure and invisible to naive error handling. Provider identity flows into the evidence graph; a bar from the fallback is not the same evidence as one from the primary.

**Provider chains** (`ProviderChain`) declare primary, ordered fallbacks, agreement policy (N-of-M for critical reads), divergence action (Halt/PreferPrimary/FlagAndContinue), and failover triggers. Crypto identity (CoinGecko) has no fallback because identity divergence must halt rather than silently resolve.

## 15. The AI Plane and AI Stack (Wave 1, refined in v1.2)

This is the plane the v1.0/v1.1 corpus left thinnest, and where v1.2's contribution is largest. The full provider stack, capability matrix, and per-tier model selection live in `PRISMATIK_v1.2_Reference_Consolidation_and_AI_Stack.md` §4. Summary:

**The critical OAuth finding.** Anthropic (Feb 2026) and OpenAI both prohibit third-party use of their subscription OAuth tokens. OAuth-first works cleanly for **Google Gemini, OpenRouter, Alpaca MCP, IBKR MCP, Coinbase MCP**. For Claude Opus 5 and GPT-5.5 Pro you fall back to API keys. The cleanest single integration is **OpenRouter OAuth PKCE**, which fans out to every frontier model with per-model routing.

**Three-tier architecture:**
- **OAuth Tier:** OpenRouter (PKCE → per-user key, all models), Google Gemini (ADC or Vertex AI), Alpaca MCP (OAuth execution), IBKR MCP (broker auth), Coinbase MCP (OAuth crypto).
- **API-Key Tier:** Anthropic (Claude Opus 5 / Sonnet 5 / Haiku 4.5), OpenAI (GPT-5.5 / Pro), xAI Grok 4.5, DeepSeek V4, Cohere Command A, Groq (fast inference), Together/Fireworks (open-model hosting).
- **Local Tier (LM Studio):** T0 Phi-4-mini (symbology, extraction), T1 Qwen3-14B Q4_K_M (filing summary, evidence), T2 Qwen3-30B-A3B MoE (multi-evidence synthesis). Loopback egress *enforced*, model digest *required*, JSON-schema structured output *mandatory* at T0/T1.

**The boundary, restated (v1.0 §20.1):** the LLM never holds write authority over anything it can also hallucinate about. Live order submission is not a tool; it is a deterministic workflow the `draft_order` tool feeds into. The OMS accepts only `RiskApproved<OrderIntent>` that no code path outside the risk kernel can construct.

**The AnalystScope correction (from ai-trading-claude, MIT):** shared discovery *worsens* the correlated-error problem. Five agents agreeing after reading the same news summary is one opinion reported five times. The correction is a hard common-facts/private-evidence split — technical agent MUST NOT see the news the sentiment agent sees. Then agreement across analysts is genuine signal.

**Determinism at the inference layer:** temperature 0.0, bounded tool rounds per step, stateless-per-step agents, the LLM call is a *seeded, recorded* effect, and replay returns the recorded response rather than re-invoking the model.

## 16. The Strategy Plane (Wave 3)

Three authoring modes, one IR. Python is not an authoring mode for executable strategies — it is a research mode that *emits* StrategyIR in the sidecar, which then executes on the Rust runtime. The reason is Invariant I3: a Python strategy executing inside a backtest brings an entire interpreter's worth of ambient nondeterminism into the deterministic core. Researchers keep their tooling; the runtime keeps its guarantees.

**The Strategy DSL** is purpose-built, non-Turing-complete, parsed with a hand-written recursive-descent parser over a `logos` lexer, compiling to DataFusion logical plans for data access and StrategyIR rule trees for logic. Non-Turing-complete is the most important word: a strategy language with unbounded loops needs fuel metering, a timeout, and a story about what a half-executed strategy means for portfolio state. A language where every expression terminates by construction needs none of those. Rejecting general-purpose embedded interpreters (rhai, steel, Lua) is a feature.

**Point-in-time enforcement is a DataFusion plan rewrite rule**, not a caller-discipline convention. A strategy author cannot bypass it by writing the query differently.

**Indicator kernel** behind a trait with warmup enforcement and golden vectors. YATA supplies formulas behind `prismatik-indicator-core`; PRISMATIK owns the trait and the golden vectors. **AIAlpha's `BarSampler` (time/tick/volume/dollar) is a Phase 4 addition** — information-driven bars (dollar bars sample on cumulative traded value, producing returns closer to IID) matter more for crypto than equities, and v1.0's indicator kernel has no bar-sampling primitive.

**Walk-forward** gets the López de Prado treatment: **purged** (drop training samples whose label horizon overlaps the test window) and **embargoed** (drop a buffer after the test window). Without these, overlapping labels leak across the split and every out-of-sample number is optimistic. PRISMATIK's existing `server/` walk-forward reports configurations-searched for multiple-testing bias — AFML adds purging and embargo.

## 17. The Model Plane and Calibration (Wave 3)

Every model artifact — linear baseline, gradient-boosted tree, neural net, 700M-parameter transformer — enters through one registration path with one governance record.

**The plural TSFM registry (updated for 2026):** Kronos (MIT, K-line specialist + path synthesizer + tokenizer-as-embedding), Chronos-2 (Apache, general + covariates), TiRex-2 (Apache, xLSTM streaming), Toto 2.0 (Apache, scaling option), TimesFM 2.5 (Apache, long-horizon), Lag-Llama (Apache, probabilistic baseline). Moirai/Moirai-2 hard-denied at commercial registration (CC BY-NC). Per-frequency-band baseline benchmarking against Auto-Theta/ARIMA/GARCH is non-optional.

**The baseline ladder is a registration precondition, not a cultural aspiration.** A model cannot be promoted to any user-facing surface unless the registry holds an out-of-sample comparison against all lower rungs and the candidate wins on *both* discrimination and calibration. Winning on discrimination alone is insufficient — a model that ranks better but is worse calibrated is a model that will be trusted more than it deserves.

**Calibration is the concrete engineering of Invariant I2.** MAPIE + crepes as sidecars. ACI (Adaptive Conformal Inference) as the default for time series, chosen over standard split conformal because standard conformal's coverage guarantee assumes exchangeability and financial returns are emphatically not exchangeable. ACI adjusts the target quantile online in response to realized coverage error and does not require exchangeability. Mondrian (regime-conditioned) calibration for when a single global interval is too wide in calm regimes and too narrow in stressed ones.

**The UI consequence, and it is the whole point.** Every forecast surface shows the coverage curve, not a confidence percentage in isolation. A user seeing "90% interval" alongside "realized coverage 72% over the last 500 observations, and 51% in high-volatility regimes" understands something true. A user seeing "90% confidence" alone does not.

## 18. The Risk and Execution Plane (Wave 5)

**Sixteen pre-trade checks, fixed evaluation order:** cheapest/most-deterministic first, all HardDeny checks before any SoftWarn. A deny should not depend on how far evaluation got. A fixed order makes the audit record comparable across runs. All checks recorded, including passes. The evidence chain for a denial includes the portfolio snapshot at evaluation time, so a denial is explainable months later without reconstructing state.

**The unknown-result problem (v1.0 §19.2):** `BrokerGateway::submit` returns three outcomes, not two. `Accepted`, `Rejected`, and `Unknown`. Unknown is the interesting one — a timeout, dropped connection, or 5xx after the request left the process leaves local state and broker state potentially divergent, and the naive response (retrying) is how duplicate orders happen. The unknown path: quarantine the instrument, reconcile, compare delta against local belief, append resolution to audit ledger, release quarantine only on success. Failing loudly and stopping is correct; guessing is not.

**The broker error taxonomy (from OpenAlice, clean-room ADR-024):** `BrokerErrorCode = CONFIG | AUTH | NETWORK | EXCHANGE | MARKET_CLOSED | CONNECTING | UNKNOWN`. `permanent = code === CONFIG || AUTH`. The `Connecting` state means "data pending, retry shortly, NOT a failure" — a read returns immediately without blocking, without counting as a health failure, without disabling the account. **Classify `MARKET_CLOSED` before `AUTH`** — venues return 403 for both, and getting the order wrong means a routine after-hours read permanently disables a healthy account.

**Reconciliation is a continuous loop, not a startup step** (from nofx, clean-room). Per-venue incremental fill sync from a persisted watermark with multi-method symbol detection, wrapped in a loop with exponential backoff capped at 5min. Halt the session on ambiguity is correct for v0.1; the production shape is snapshot → diff against journal → classify divergence → auto-heal benign cases → halt only on the genuinely irreconcilable.

**Cost-basis provenance (from OpenAlice, clean-room ADR-026):** `avgCostSource: 'broker' | 'wallet'`. Crypto spot via CCXT frequently has *no* real cost basis; you are synthesizing it from your own order history. That must be visible in the type, must flow into the evidence graph, and must be surfaced in the UI wherever a P&L figure derived from a wallet basis is displayed.

## 19. The Plugin Host (Wave 3)

**wasmtime + Component Model + WIT on `wasm32-wasip2`** (WASI Preview 2 stabilized April 2026; WASI 0.3 with native async shipped in Wasmtime 37+ in 2026). Extism prototype dropped — go direct to wasmtime + WIT. Capability-scoped: the host provides only the imports a plugin's granted capabilities entitle it to. A plugin without the network capability does not receive a network import, which means it cannot make a network call even if its code attempts to. This is Invariant I4 by construction.

**Mandatory wasmtime config (asserted by unit test):** Cranelift only (Winch prohibited per RUSTSEC-2026-0095), signals-based-traps on, guard pages on, fuel metering on, epoch interruption on, no ambient nondeterminism (`wasm_threads` off, `relaxed_simd` off — relaxed SIMD permits implementation-defined results and is disqualifying in a platform whose central claim is reproducibility).

**Plugin supply chain:** wasm binaries harder to review than source. Gate with `wasm-tools` disassembly, **capability diffing on imports/exports between versions** (a plugin update that newly imports a network capability must fail the gate automatically, not await human review), and cosign verification before load.

## 20. The Compliance Plane (Wave 1 transparency, Wave 6 full)

**Updated for 2026 regulatory reality.** The v1.1 addendum's "EU AI Act high-risk obligations land August 2, 2026" is wrong as of July 2026: the Digital Omnibus delayed standalone Annex III high-risk to **December 2, 2027** (16-month relief). Transparency obligations still bite August 2, 2026. The full compliance plane can stay in Wave 6 if scoped to transparency only.

**SR 11-7 was replaced by SR 26-2 / OCC Bulletin 2026-13 (April 2026)** — joint OCC/Fed/FDIC revised Model Risk Management guidance. Modernizes for AI/ML and dynamic models; concentrates validation on high-materiality models. **Critical carve-out: "Generative AI and agentic AI models are novel and rapidly evolving. As such, they are not within the scope of this guidance."** PRISMATIK's model registry already satisfies most MRM requirements — say so explicitly, because "we already have a compliant model inventory" is a concrete enterprise sales asset.

**SEC Predictive Data Analytics final rule** (Release 34-97990, June 2025) covers broker-dealers and RIAs using "covered technology" in "investor interactions." Must identify, eliminate, or neutralize conflicts of interest. If PRISMATIK makes recommendations or interacts with investors using predictive AI for a broker-dealer/RIA user, this rule applies. Build conflict-elimination documentation into the design from Wave 1.

**Disclosures are generated from the CalibrationRecord, not authored.** A disclosure that says "forecasts are probabilistic and may be wrong" is legally decorative. A disclosure that says "this model's 90% intervals contained the realized value 72% of the time over the last 500 observations, and 51% during high-volatility regimes" is a factual statement derived from data the platform already holds. Generate it. It is more honest, more defensible, and requires no legal review to keep current because it updates itself.

---

# Part IV. The Wave Delivery Plan

The wave framework is the strategic overlay on the phased plan. Each wave is a coherent business outcome ending in a shippable, sellable artifact. Each has explicit entry criteria, definition-of-done, and a go/no-go decision the wave is *designed* to test.

## IV.0 The Capacity Model and Honest Total

Estimates are in **engineering-days**: one focused day of a single senior engineer using AI-assisted development tooling. Not a person-day of calendar time, not a story point. At five focused days per week with no other obligations, the full plan is roughly two and a half years. At the two to three focused days per week a realistic full-time engineer has available, it is five to seven years to Wave 7.

**Total: ~616 engineering-days.** The wave framework below is built to test the central commercial hypothesis at Wave 1 (~80 days under Option B) rather than at Wave 3 (~335 days under Option A). If Wave 1 fails to find buyers, the loss is bounded at ~128 days instead of ~400.

**Estimate confidence bands:** H (high, ±20%), M (medium, ±50%), L (low, 2–3× possible). Every wave below carries the band on its work items.

## IV.1 Wave Sequencing: Two Options

### Option A — Architecture Order
Phases run 0 through 9 as written. Determinism, audit, and manifest complete before the first provider adapter. **Zero retrofit cost. Every subsequent line of code written against the kernel from start.** Cost: ~128 days before anything demonstrable to a prospect. **Choose this if funded, or if PRISMATIK is a long-horizon asset with no near-term revenue requirement, or if enterprise buyer is the target and reproducibility is why they buy.**

### Option B — Revenue-First with a Determinism Floor (RECOMMENDED)
Wave 0 is split. A minimal, non-negotiable floor ships first. The remainder interleaves across Waves 1–2, funded by revenue. **First sellable artifact at ~80 days instead of ~128**, a 37% reduction in time to first revenue.

**The floor (`P0-FLOOR`, ~24 days):** `Clock`/`Entropy`/`DetMap`/`DeterminismContext` (every call site written without them must be rewritten — multiplicative); determinism lints and grep gate (a gate added later has to be paid down against an existing violation set, which is how gates die); `AssetId` (every table, struct, signature — changing the identity type later touches everything); workspace + `deny.toml` + `clippy.toml` + CI skeleton; Tauri shell with capability policy and strict CSP (loosening later is easy; tightening later is not); tauri-specta bindings pipeline.

**The remainder interleaves across Waves 1–2**, with the hard condition: **everything in `P0-REMAINDER` is complete before Wave 3 opens.** Wave 3 introduces the strategy runtime and backtest engine, the first point where a missing manifest or audit ledger is a correctness problem rather than a missing feature. If `P0-REMAINDER` is not complete when Wave 3 opens, Wave 3 does not open.

The remainder of this Part IV is written in Option A wave numbering. Wave 0 marks every item as `FLOOR` or `DEFER`.

## IV.2 Wave 0 — Foundation

**Objective.** Establish the mechanisms whose absence would make every later wave more expensive: deterministic time and entropy, canonical identity, tamper-evident audit, signed reproducibility manifests, generated type bindings, sandboxed shell, and the CI gates that keep all of it true.

**Entry criteria.** None. This is the start.

**Duration.** ~72 days total. ~24 days if scoped to `P0-FLOOR` under Option B.

### Work items

| ID | Track | Work item | Days | Conf | Option B |
|---|---|---|---:|:---:|:---:|
| P0-OD-01 | OD | Cargo workspace, crate skeletons, `rust-toolchain.toml` pinned | 2 | H | FLOOR |
| P0-OD-02 | OD | `clippy.toml` with determinism `disallowed-methods`/`disallowed-types`, `-D warnings` | 1 | H | FLOOR |
| P0-OD-03 | OD | GitHub Actions CI: fmt, clippy, test, matrix on Windows + Linux | 2 | H | FLOOR |
| P0-OD-04 | OD | Project workbook, ADR template and index, `CONTRIBUTING.md`, `SECURITY.md` | 2 | H | FLOOR |
| P0-DK-01 | DK | `Clock` trait: `SystemClock`, `SimulatedClock`, `FrozenClock` | 3 | M | FLOOR |
| P0-DK-02 | DK | `Entropy` trait, splittable stream, order-independence property test | 4 | M | FLOOR |
| P0-DK-03 | DK | `DetMap`, `DetSet`, `DeterminismContext`, `PinnedArtifactSet` types | 2 | H | FLOOR |
| P0-DK-04 | DK | `determinism-grep` CI gate plus a deliberate-violation negative test | 2 | M | FLOOR |
| P0-DK-05 | DK | `prismatik-identity`: `AssetId`, `VenueId`, `ExternalIdentifier`, id factory | 3 | M | FLOOR |
| P0-DK-06 | DK | Bitemporal symbology store: `resolve_as_of`, identity chain, property test | 6 | M | DEFER |
| P0-DK-07 | DK | Merkle append-only audit ledger, inclusion + consistency proofs | 6 | M | DEFER |
| P0-DK-08 | DK | Audit startup verification + `criterion` benchmark vs 1ms p99 append | 2 | M | DEFER |
| P0-DK-09 | DK | `prismatik-manifest`: schema v1.0, canonical serialization, builder | 4 | M | DEFER |
| P0-DK-10 | DK | Dual signature (Ed25519 + ML-DSA), cross-machine verification test | 5 | L | DEFER |
| P0-DK-11 | DK | `prismatik-cli verify` standalone verifier, no application dependency | 3 | M | DEFER |
| P0-DK-12 | DK | `ArtifactStore` trait, content-addressed storage, hash+sig verification on load | 3 | M | DEFER |
| P0-DK-13 | DK | `prismatik-calendar`: artifact format, `SessionCalendar` trait, reader | 3 | M | DEFER |
| P0-DK-14 | DK | Calendar generator script with QuantLib cross-validation, zero-tolerance gate | 5 | L | DEFER |
| P0-SS-01 | SS | Tauri capability policy (default-deny, strict CSP, no inline/eval) — **patched ≥2.12 per CVE-2026-42184** | 3 | M | FLOOR |
| P0-SS-02 | SS | Key hierarchy: OS keychain, device root key, derived keys, `secrecy`/`zeroize` | 5 | M | DEFER |
| P0-SS-03 | SS | `deny.toml` license allowlist + advisory policy, blocking in CI | 2 | H | FLOOR |
| P0-SS-04 | SS | `cargo vet init`, import Mozilla/Google/Bytecode Alliance audit sets | 3 | M | DEFER |
| P0-SS-05 | SS | `prismatik-oss-registry`: manifest schema, license-class gate, coverage CI | 4 | M | DEFER |
| P0-SS-06 | SS | Release signing (cosign), SBOM (`cargo cyclonedx`), SLSA provenance | 4 | L | DEFER |
| P0-SS-07 | SS | Updater with provenance *verification* + negative test refusing invalid artifact | 4 | L | DEFER |
| P0-EX-01 | EX | Tauri 2 shell, window management, crash recovery, single instance | 3 | M | FLOOR |
| P0-EX-02 | EX | `tauri-specta` bindings pipeline, exact version pins, `bindings-drift` CI gate | 3 | M | FLOOR |
| P0-EX-03 | EX | Design tokens, CSS variable theme, light/dark, contrast validation | 4 | M | FLOOR |
| P0-EX-04 | EX | Motion primitives, reduced-motion support, first five components | 5 | M | FLOOR |
| P0-QM-01 | QM | `dst_replay_suite` skeleton on trivial pipeline, trace capture + digest comparison | 4 | L | DEFER |
| P0-QM-02 | QM | Golden manifest corpus harness + first two committed manifests | 3 | M | DEFER |

**Floor subtotal: 24 days. Deferred subtotal: 48 days. Phase total: 72 days.**

### Definition of Done

A phase is complete when every line below is demonstrably true, verified by a named artifact rather than by assertion.

| # | Criterion | Verified by |
|---|---|---|
| 1 | No call to `Instant::now`, `SystemTime::now`, `thread_rng`, or default-hasher map exists outside the determinism-crate allowlist | `determinism-grep` green; allowlist <10 entries |
| 2 | The grep gate demonstrably fails on an introduced violation | Committed negative test |
| 3 | Entropy split is order-independent | `proptest`, 10k iterations |
| 4 | A trivial pipeline replays byte-identically across 64 seeds | `dst_replay_suite` green |
| 5 | A manifest signed on machine A verifies on machine B with no shared state | Manual cross-machine run, recorded |
| 6 | Both signatures are required; a single valid signature fails verification | Committed negative test |
| 7 | The audit ledger detects a retroactive edit | Committed tamper test |
| 8 | Audit append meets 1ms p99 | `criterion` report committed |
| 9 | The updater refuses an artifact with invalid SLSA provenance | Committed negative test |
| 10 | Generated TypeScript bindings match the Rust source | `bindings-drift` green |
| 11 | Every workspace dependency appears in the component registry | `registry-coverage` green |
| 12 | Calendar generator produces zero disagreements against QuantLib for US equity venues | Generator report committed |
| 13 | The Tauri capability policy contains no `fs`, `http`, `process`, or `shell:execute` permission; Tauri ≥2.12 | Policy file review + version pin, recorded in ADR |
| 14 | ADRs 0020 through 0028 written and accepted | ADR index |

**Under Option B**, criteria 1, 2, 3, 10, and 13 gate the floor. The remainder gate the opening of Wave 3.

### Why this wave exists and why most of it cannot be deferred

**Wave 0 produces nothing a customer can see.** ~72 days of determinism kernels, Merkle ledgers, and CI gates, and at the end of it there is no product. For a funded team that is fine. For a one-engineer commercial operation it is the single largest risk in the plan, larger than any technical risk in the register. Option B resolves this by deferring everything that is additive rather than invasive.

The four items that cannot be deferred (the multiplicative-retrofit items): `Clock`/`Entropy` (every call site written without them must be rewritten), determinism lints (a gate added later has to be paid down against an existing violation set, which is how gates die), `AssetId` (every table, struct, signature), and the Tauri capability policy (loosening later is easy; tightening after features depend on the looseness is not).

### Risks specific to Wave 0

| Risk | Response |
|---|---|
| ML-DSA implementation maturity in Rust uneven; `P0-DK-10` could run 2× | Time-box to 8 days. If it overruns, ship Ed25519 only with the dual-signature *format* in place and the ML-DSA field present but empty, so the retrofit is a field population rather than a schema migration. Record as explicit exception in the PQC register. |
| Specta v2 RC breaks on a Tauri patch release | Exact pins; a `cargo update` is a deliberate, reviewed act. Fallback ADR-0031 already written. |
| Determinism lints produce excessive friction and get disabled | Allowlist capped at 10 entries; every addition requires a one-line justification in the file. A cap makes pressure visible. |
| The 24-day floor slips to 40 and Option B loses its advantage | Weekly checkpoint against the floor item list. If day 30 arrives with the floor incomplete, cut `P0-EX-03` and `P0-EX-04` to a single unstyled component set; finish styling during Wave 1. |

## IV.3 Wave 1 — MVP: Crypto Intelligence

**Objective.** A complete, sellable crypto intelligence product on the Wave 0 foundations. The first artifact a customer sees. **This wave exists to test the commercial hypothesis: does the evidence-chained, local-first, privacy-respecting crypto wedge find buyers?**

**Entry criteria.** Wave 0 exit gate, or `P0-FLOOR` subset under Option B.

**Duration.** ~56 days. First sellable artifact at ~80 days under Option B.

### Work items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P1-DP-01 | DP | `Provider` trait, `ProviderCapabilities`, `EntitlementSet`, `ProviderHealth` | 3 | M |
| P1-DP-02 | DP | `BudgetGovernor` on `governor` GCRA, priority classes, exact next-permit times | 4 | M |
| P1-DP-03 | DP | `ProviderChain` with declared fallbacks, `FailoverTrigger::Empty`, evidence-graph provenance | 4 | M |
| P1-DP-04 | DP | CoinGecko adapter (auth, search, global, markets, coin detail, market chart, OHLC, categories, exchanges) | 8 | M |
| P1-DP-05 | DP | Provider contract tests with recorded cassettes, replayable offline | 3 | M |
| P1-DP-06 | DP | Raw layer: Parquet writer, partitioning, append-only enforcement, supersession links | 4 | M |
| P1-DP-07 | DP | Normalization to canonical types, quality scoring, dedup | 4 | M |
| P1-DP-08 | DP | DuckDB analytical views over curated Parquet | 3 | M |
| P1-DP-09 | DP | SQLite operational state, migrations, task-graph persistence | 3 | M |
| P1-DP-10 | DP | Evidence + lineage plane, `EvidenceRef`, `Concludes` trait, invalidation hash including pinned set | 5 | M |
| P1-DP-11 | DP | In-process Tokio task graph, `PipelineTask`, triggers, crash recovery | 5 | M |
| P1-DP-12 | DP | LanceDB integration, asset/category description embeddings, `AnalogStore` skeleton | 4 | M |
| P1-AI-01 | AI | `prismatik-ai-router`: provider abstraction, capability-based routing, recorded-effect inference | 5 | M |
| P1-AI-02 | AI | LM Studio provider (T0/T1 tiers, JSON-schema structured output, loopback egress enforcement, model-digest pinning) | 6 | M |
| P1-AI-03 | AI | OpenRouter OAuth PKCE + Gemini OAuth (the OAuth tier) | 4 | M |
| P1-AI-04 | AI | Anthropic + OpenAI API-key providers (the API-key tier) | 3 | M |
| P1-AI-05 | AI | MCP 2026-07-28 RC alignment in `prismatik-ai-tools` (stateless core, OAuth 2.1 Resource Server) | 4 | M |
| P1-AI-06 | AI | `AnalystScope` common-facts/private-evidence split (from ai-trading-claude, MIT) | 3 | M |
| P1-EX-01 | EX | Command palette, workspace shell, panel system, layout persistence | 5 | M |
| P1-EX-02 | EX | `ChartDocument` + `ChartBackend` contracts, Lightweight Charts wrapper, theme bridge | 6 | M |
| P1-EX-03 | EX | Crypto command dashboard: global stats, dominance, trending, category map | 5 | M |
| P1-EX-04 | EX | Watchlist with live updates, virtualized rows, 500-row budget | 4 | M |
| P1-EX-05 | EX | Asset workspace: profile, chart, markets, exchanges, categories, evidence drawer | 6 | M |
| P1-EX-06 | EX | Market scanner with saved filters and result provenance | 4 | M |
| P1-EX-07 | EX | Rate-budget UI showing exact countdowns from GCRA, not spinners | 2 | H |
| P1-EX-08 | EX | Empty, loading, error, and degraded states for every surface | 3 | M |
| P1-QM-01 | QM | Alert-rule evaluator, streaming + scheduled split, dedup keys | 5 | M |
| P1-QM-02 | QM | Notification service, delivery channel abstraction, escalation | 4 | M |
| P1-OD-01 | OD | First-run experience state machine, demo mode, suitability profile | 5 | M |
| P1-OD-02 | OD | Backup, restore, cross-version migration | 4 | M |
| P1-OD-03 | OD | OpenTelemetry tracing with determinism telemetry attributes | 3 | M |
| P1-OD-04 | OD | Installer, code-signed, auto-update flow end-to-end | 4 | L |
| P1-OD-05 | OD | User documentation, licensing and disclosure surfaces, notice generator | 4 | M |

### Definition of Done

| # | Criterion | Verified by |
|---|---|---|
| 1 | Every rendered conclusion resolves a complete evidence chain to raw records | `test_no_orphan_conclusions` green |
| 2 | The rate-budget UI shows an exact retry time, never an indeterminate spinner | Manual verification against a throttled key |
| 3 | The application functions in degraded mode when CoinGecko is unreachable | Chaos test with provider blackholed |
| 4 | Stale data is visibly marked on every surface that displays it | Manual verification with a frozen feed |
| 5 | A clean-machine install completes and auto-update applies a signed increment | Recorded on a fresh VM |
| 6 | Cold start to interactive under 2.0s p95 on mid-tier hardware | Benchmark report |
| 7 | Chart pan/zoom at 1M points holds 16.7ms p99 frame | Benchmark report |
| 8 | Provider contract tests pass offline from cassettes | CI green with network disabled |
| 9 | A backup taken on version N restores on version N+1 | Migration test |
| 10 | Local AI tier is structurally incapable of network egress (loopback enforced) | Network-trace negative test |
| 11 | Every AI-generated conclusion carries uncertainty + contradictions + suggested verification | UI review across 20 sample queries |
| 12 | **The product has been used for one continuous week by its author as a user** | Workbook log |

### The Commercial Milestone

`P1-OD-04` (installer + auto-update) is the item that turns a codebase into a product. It is placed near the end because everything before it is a prerequisite, and it is called out here because a solo operation can spend a year building and never ship an installer.

**Suggested checkpoint at day 40 of Wave 1:** package whatever exists, install it on a clean machine, and use it for a week as a user rather than as its author. Every defect that surfaces in that week is a defect a customer would have found. This checkpoint costs two days and reliably returns more than it costs.

### The hypothesis this wave tests

If Wave 1 finds no buyers, that is a ~128-day loss rather than a ~400-day loss, and Wave 2 (equity/options) becomes the alternative wedge. The wave framework is explicitly designed to make this pivot cheap.

### Risks specific to Wave 1

| Risk | Response |
|---|---|
| CoinGecko free tier limits make the product feel slow | Aggressive content-keyed caching + budget governor surfacing honest wait times. If the free tier is unusable, the paid tier becomes a stated prerequisite and the pricing model absorbs it. |
| The crypto wedge does not find buyers | This is the point of shipping early. Pivot to equity/options (Wave 2) as the alternative wedge. |
| Feature creep from a demo that impresses | The exit gate is the scope. New ideas go to a Wave 2 candidate list, not into Wave 1. |
| LM Studio structured-output reliability insufficient for T0/T1 | Fall back to Groq-hosted Llama 4 Scout for T0/T1 (460 TPS, ~10–20× cheaper than OpenAI); keep local tier for T2 synthesis only. |

## IV.4 Wave 2 — Multi-Asset Intelligence

**Objective.** Extend to US equities with regulatory and macro context, make bitemporal symbology load-bearing (equity history is wrong without it), and ship the differentiating options wedge. **This wave produces the product that justifies the entire architecture.**

**Entry criteria.** Wave 1 exit gate. **Under Option B, `P0-DK-06` bitemporal symbology moves here and is mandatory.**

**Duration.** ~116 days (P2 ~54 + P3 ~62).

### Wave 2A — Equity, Filings, Macro, Real Symbology (~54 days)

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P2-DK-01 | DK | Bitemporal symbology production hardening, OpenFIGI ingestion, identity chain | 6 | M |
| P2-DK-02 | DK | Corporate-action ledger, event types including `OptionAdjustment`, conflict retention | 5 | M |
| P2-DK-03 | DK | Read-time adjustment factor computation from ledger (never destructive rewrite) | 4 | M |
| P2-DK-04 | DK | Calendar artifacts for NYSE, NASDAQ, ARCA, BATS with QuantLib cross-validation | 4 | M |
| P2-DP-01 | DP | SEC EDGAR adapter: submissions, company facts, filing index, full-text search | 7 | M |
| P2-DP-02 | DP | Filing parser: 13F, Forms 3/4/5, 8-K item extraction, SC 13D/13G | 8 | L |
| P2-DP-03 | DP | CFTC Commitments of Traders adapter and weekly schedule | 4 | M |
| P2-DP-04 | DP | FRED adapter, series metadata, release calendar, vintage handling | 4 | M |
| P2-DP-05 | DP | Equity OHLCV provider integration behind the `Provider` port | 4 | M |
| P2-DP-06 | DP | Feature store: `FeatureView`, `observation_delay`, offline point-in-time join | 6 | M |
| P2-DP-07 | DP | Point-in-time property test suite | 2 | H |
| P2-EX-01 | EX | Institutional intelligence: 13F views, ownership, insider activity | 6 | M |
| P2-EX-02 | EX | Event and catalyst engine with earnings and macro calendar | 5 | M |
| P2-EX-03 | EX | Universal instrument workspace generalized across asset classes | 5 | M |
| P2-EX-04 | EX | Perspective integration for streaming analytical grids | 4 | M |

### Wave 2B — Options Intelligence (~62 days, the differentiating wedge)

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P3-DP-01 | DP | Unusual Whales adapter behind `Provider` port, entitlement mapping | 6 | M |
| P3-DP-02 | DP | Options chain normalization, OCC symbology, contract identity | 5 | M |
| P3-DP-03 | DP | Flow observation ingestion at print level, ClickHouse for cloud profiles | 6 | L |
| P3-DP-04 | DP | Historical chain and IV surface storage, compression strategy | 5 | M |
| P3-QM-01 | QM | `prismatik-quant-kernel`: pricing, greeks, IV solve, selected RustQuant modules | 7 | M |
| P3-QM-02 | QM | QuantLib conformance sidecar, 1e-8 tolerance gate on pricing and greeks | 5 | L |
| P3-QM-03 | QM | Flow classification: directional, hedging, closing, spread-leg detection | 8 | L |
| P3-QM-04 | QM | Flow clustering and aggregation | 5 | L |
| P3-QM-05 | QM | Trade-quality score with disclosed and versioned formula | 4 | M |
| P3-EX-01 | EX | Chain explorer with dense grid, greeks columns, liquidity shading | 6 | M |
| P3-EX-02 | EX | Volatility lab: term structure, skew, IV rank, surface via wgpu | 7 | L |
| P3-EX-03 | EX | Strategy constructor with payoff diagram and break-even analysis | 6 | M |
| P3-EX-04 | EX | Dealer exposure views | 4 | M |
| P3-SS-01 | SS | `adjusted_contract` pre-trade check wired to corporate-action ledger | 2 | H |

### Definition of Done (Wave 2)

| # | Criterion | Verified by |
|---|---|---|
| 1 | A 2015→2026 equity universe resolves every ticker correctly through renames, splits, mergers | Hand-checked set of 50 known identity events |
| 2 | Corporate actions applied at read time; raw series unmodified | Byte comparison of raw Parquet before/after split ingestion |
| 3 | No feature value returned whose `event_time + observation_delay > as_of` | Property test, 10k cases |
| 4 | 13F holdings not observable before filing date, only period end | Targeted test on known filing |
| 5 | Calendar artifacts show zero disagreement with QuantLib across all four venues | Generator report |
| 6 | Adjusted option contracts flagged in corporate-action ledger; automation hard-denied | Test against known OCC memo |
| 7 | Pricing and greeks match QuantLib within 1e-8 relative across 500-case grid | Conformance report |
| 8 | Flow classification decisions expose evidence and confidence | UI review against 20 hand-labeled prints |
| 9 | IV surface renders at 60fps with functional Canvas fallback | Benchmark + fallback test |
| 10 | Trade-quality score formula and version appear alongside every score | UI review |

### Risks specific to Wave 2

| Risk | Response |
|---|---|
| `P2-DP-02` filing parsing is the largest low-confidence item in the wave | Scope to 13F + Form 4 only for the gate. 8-K item extraction and 13D/G move to a Wave 2.5 candidate list. Partial filing coverage is a feature gap; wrong filing parsing is a correctness failure. |
| SEC rate limits and user-agent requirements | Implement declared user agent + 10 req/sec ceiling in the adapter, governed by the same `BudgetGovernor`. |
| Equity price data licensing | Resolve provider + licensing **before** Wave 2 opens, not during. Phase 1 background task. |
| Flow classification is genuinely hard and partially unknowable | Ship classification with explicit confidence + "unclassified" outcome used freely. A classifier that always decides is a classifier that is often wrong. |
| Options data costs material and recurring | Model cost into pricing before Wave 2 opens. |
| ClickHouse introduction adds operational surface | Cloud and enterprise profiles only. Desktop stays on Parquet and DuckDB (architecture §15.2). |

## IV.5 Wave 3 — Quantitative Research Platform

**Objective.** Strategy authoring, deterministic backtesting, the canonical indicator kernel, the sandboxed plugin host, the plural TSFM registry, conformal calibration, the analog engine. **This is the defensible research platform.**

**Entry criteria.** Wave 2 exit gate **and, under Option B, complete `P0-REMAINDER`. This gate is hard.** Wave 3 is the first wave where a missing manifest or audit ledger is a correctness failure rather than a missing feature.

**Duration.** ~153 days (P4 ~78 + P5 ~61 + P5.5 ~14).

### Wave 3A — Strategy, Backtest, Indicator Kernel, Plugin Host (~78 days)

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P4-QM-01 | QM | `StrategyIR` types, `StrategyCapabilities`, serialization, schema version | 4 | M |
| P4-QM-02 | QM | DSL lexer on `logos`, recursive-descent parser, diagnostics with spans | 8 | L |
| P4-QM-03 | QM | DSL type checker and capability inference from source references | 6 | L |
| P4-QM-04 | QM | Data-access compilation to DataFusion logical plans | 6 | L |
| P4-QM-05 | QM | Point-in-time enforcement as DataFusion plan-rewrite rule | 4 | L |
| P4-QM-06 | QM | `Strategy` trait, runtime, `StrategyContext` over `DeterminismContext` | 5 | M |
| P4-QM-07 | QM | Backtest engine: event loop over pinned-calendar bar boundaries | 7 | M |
| P4-QM-08 | QM | `ExecutionAssumptions`, fill models, slippage, commission, assignment | 6 | M |
| P4-QM-09 | QM | Backtest metrics, walk-forward, out-of-sample, deflated Sharpe, **purged + embargoed (AFML)** | 6 | M |
| P4-QM-10 | QM | `Indicator` trait, warmup enforcement, `IndicatorDescriptor`, provenance | 4 | M |
| P4-QM-11 | QM | YATA adapter + first 30 indicators with golden vectors | 8 | M |
| P4-QM-12 | QM | `BarSampler` (time/tick/volume/dollar) — from AIAlpha, clean-room | 3 | M |
| P4-QM-13 | QM | Streaming equals batch property test across full indicator set | 2 | H |
| P4-QM-14 | QM | Two-implementation conformance harness with ai-algotrading-agent fixtures (MIT) | 3 | M |
| P4-SS-01 | SS | `prismatik-plugin-host`: wasmtime hardened engine config + config-assertion test | 4 | M |
| P4-SS-02 | SS | `CapabilitySet`, host-function registry, time+entropy from kernel | 5 | M |
| P4-SS-03 | SS | Plugin signing, install flow, capability grant in audit ledger | 4 | M |
| P4-SS-04 | SS | Capability-diffing gate on imports/exports + cosign verification at load | 3 | M |
| P4-SS-05 | SS | Capability containment test: syscall trace proving zero network from denied plugin | 3 | L |
| P4-EX-01 | EX | Visual strategy builder emitting `StrategyIR` via codegen | 8 | L |
| P4-EX-02 | EX | Backtest result workspace: equity curve, drawdown, trade list, metrics | 6 | M |
| P4-EX-03 | EX | Rust SDK crate + documentation for hand-written strategies | 4 | M |
| P4-QM-15 | QM | Python `StrategyIR` emitter in research sidecar, JSON schema + validator | 4 | M |

### Wave 3B — Simulation, TSFM Registry, Calibration (~61 days)

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P5-QM-01 | QM | Monte Carlo lab: `SimulationSource` variants, bootstrap, block bootstrap, parametric | 6 | M |
| P5-QM-02 | QM | Jump diffusion, stochastic volatility, regime-switching sources | 5 | M |
| P5-QM-03 | QM | Terminal-value, drawdown, probability-of-ruin distributions with seed pinning | 4 | M |
| P5-QM-04 | QM | Deterministic parallel reduction under Rayon, split entropy per path | 4 | L |
| P5-QM-05 | QM | `ModelRegistry`, `ModelRegistration`, license-class gate with hard deny | 4 | M |
| P5-QM-06 | QM | `PretrainingRecord`, contamination gate, security event on denial | 3 | M |
| P5-QM-07 | QM | `SeriesTokenizer` trait, codebook artifacts, `TokenizerBinding` validation | 5 | M |
| P5-QM-08 | QM | `TsfmRuntime` trait, ONNX runtime adapter, INT8 quantized CPU path | 7 | L |
| P5-QM-09 | QM | Kronos + Chronos-2 + TiRex-2 + Toto 2.0 + Lag-Llama artifact onboarding | 7 | L |
| P5-QM-10 | QM | Forecast caching keyed on asset, modality, context hash | 3 | M |
| P5-QM-11 | QM | `Calibrator` trait, `CalibrationRecord`, `CalibrationMethod` types | 4 | M |
| P5-QM-12 | QM | Calibration sidecar: MAPIE ACI + EnbPI, crepes Mondrian, Arrow transport | 6 | L |
| P5-QM-13 | QM | Baseline-ladder enforcement at promotion, both discrimination + calibration | 4 | M |
| P5-QM-14 | QM | `DriftDetector` suite: feature, calibration, embedding, token-usage, performance | 6 | L |
| P5-QM-15 | QM | Drift actions: annotate, widen, suppress, propose demotion | 3 | M |
| P5-EX-01 | EX | Monte Carlo visualization: path clouds via wgpu, distribution panels | 5 | M |
| P5-EX-02 | EX | Calibration ribbon primitive, realized coverage against nominal | 4 | M |
| P5-EX-03 | EX | Model-card surface, drift status, blind-spot disclosure | 4 | M |

### Wave 3C — Analog Engine Upgrade (~14 days)

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P55-QM-01 | QM | Embedding extraction from `TsfmRuntime::embed`, batch materialization (Kronos tokenizer first) | 3 | M |
| P55-QM-02 | QM | `AnalogStore` production implementation, LanceDB IVF-PQ index, version pinning | 4 | M |
| P55-QM-03 | QM | `AnalogQuery` with secondary filters, regime, sector, event type, venue | 3 | M |
| P55-QM-04 | QM | Leave-N-out sensitivity computation | 2 | M |
| P55-EX-01 | EX | Analog result surface with mandatory disclosures + sensitivity display | 2 | M |

### Definition of Done (Wave 3)

This is the strictest gate in the plan. Every criterion is enforced by a test, not by review.

| # | Criterion | Verified by |
|---|---|---|
| 1 | No forecast reaches UI without attached `CalibrationRecord` | Type system, no constructor omits it |
| 2 | Registry hard-denies CC BY-NC artifact in commercial profile | Negative test with synthetic Moirai-style registration |
| 3 | Backtest runtime hard-denies model whose pretraining cutoff violates the window; logs security event | Negative test + ledger inspection |
| 4 | Tokenizer/model version mismatch fails to load rather than degrading silently | Negative test |
| 5 | A rung-4 model that loses to rung 1 on calibration cannot be promoted | Negative test |
| 6 | Embedding drift above threshold auto-widens intervals with no human action | Simulated drift test |
| 7 | Monte Carlo with 1M paths under Rayon reproduces byte-identically across thread counts | Determinism test at 1, 4, 16 threads |
| 8 | Per-regime coverage displayed alongside every forecast | UI review |
| 9 | TSFM forecast p95 latency ≤500ms single-asset single-modality | Benchmark report |
| 10 | Same logical strategy authored in visual builder, DSL, Rust SDK produces identical `StrategyIR` | Three-way comparison test |
| 11 | Those three produce identical backtest results | Byte comparison of result-bundle hashes |
| 12 | Strategy compiled without `can_access_network` cannot reach socket even when code tries | Negative test with malicious strategy |
| 13 | `evaluate` bitwise identical to `next` sequence for all 30 indicators | Property test |
| 14 | Plugin with no granted capabilities makes zero syscalls of network/filesystem class | `strace`/ETW trace committed |
| 15 | Relaxed SIMD disabled and engine config assertion test passes | Committed test |
| 16 | A backtest emits a complete signed manifest that the standalone verifier accepts | End-to-end run |
| 17 | `CloseOnly` fill-model results carry mandatory badge in UI | UI review |
| 18 | Full `dst_replay_suite` over ingest→manifest passes 256 seeds | CI green |
| 19 | Analog search returns neighbours only alongside distance metric, sample size, applied filters, survivorship warning, leave-N-out sensitivity; no code path renders without them | UI review + code audit |
| 20 | Search of top 50 across 10M vectors completes within 120ms p95 | Benchmark report |

**Criterion 7 deserves emphasis:** byte-identical results across *different thread counts* is the property that proves the split-entropy design works. Same-thread-count reproducibility is much weaker and much easier to achieve accidentally.

### Risks specific to Wave 3

| Risk | Response |
|---|---|
| The DSL is the largest low-confidence cluster (24 days across four items) | Build type checker + IR *first*, parser second. A parser targeting a proven IR is bounded work; parser + IR designed together is not. If DSL overruns >50%, ship Wave 3 with visual builder + Rust SDK only; move DSL to Wave 3.5. |
| TSFM inference on desktop CPU misses the 500ms budget | Fall back to Chronos-Bolt (distilled, faster). If still misses, TSFM becomes a Team Cloud feature and desktop ships the classical model ladder only. Architecture already supports this via the profile table. |
| Kronos becomes unmaintained mid-wave | Plural registry mitigates. Chronos-2/TiRex-2/Toto 2.0 are Apache-2.0 and integrated through the same trait. |
| Conformal calibration misapplied to non-exchangeable data | ACI is default by type; standard split conformal restricted to cross-sectional tasks. Per-regime coverage reporting makes misapplication visible rather than silent. |
| Calibration sidecar adds Python runtime dependency to desktop install | Sidecar is opt-in on desktop per profile table. Calibration records can be computed on schedule and shipped as signed artifacts rather than locally. |

## IV.6 Wave 4 — Research-to-Decision Loop

**Objective.** Portfolio accounting on the audit ledger, the learning loop, and a paper broker that behaves like a real one. **The complete research-to-decision loop.**

**Entry criteria.** Wave 3 exit gate.

**Duration.** ~48 days.

### Work items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P6-DK-01 | DK | Audit ledger becomes portfolio write path; projections as read models (**evaluate TigerBeetle Rust client**) | 6 | L |
| P6-DK-02 | DK | Projection rebuild from ledger replay, startup reconciliation | 4 | M |
| P6-DK-03 | DK | Divergence detection between projection and ledger, alert on mismatch | 3 | M |
| P6-QM-01 | QM | Portfolio model: positions, lots, cost basis (`avg_cost_source` provenance), realized/unrealized PnL | 6 | M |
| P6-QM-02 | QM | Risk measures: exposure, concentration, correlation, scenario analysis | 6 | M |
| P6-QM-03 | QM | `RiskPolicy` + full 16-check pre-trade catalogue with fixed ordering | 6 | M |
| P6-QM-04 | QM | Paper broker with realistic fill, latency, rejection behaviour | 5 | M |
| P6-QM-05 | QM | **`prismatik-reconciliation`: continuous sync loop, divergence classification, auto-heal, halt on irreconcilable** (from nofx, clean-room) | 6 | L |
| P6-EX-01 | EX | Portfolio workspace, exposures, risk panel, scenario runner | 6 | M |
| P6-EX-02 | EX | Journal: entry capture, thesis linkage, outcome tagging, review workflow (**3-layer memory loop from prism-insight, clean-room**) | 5 | M |
| P6-QM-06 | QM | Post-trade learning: realized vs expected, thesis validation reporting | 4 | M |
| P6-QM-07 | QM | TimesFM onboarded to registry | 2 | M |

### Definition of Done (Wave 4)

| # | Criterion | Verified by |
|---|---|---|
| 1 | Portfolio state rebuilt from full ledger replay matches live projection exactly | Property test |
| 2 | A deliberately corrupted projection detected at startup and rebuilt | Negative test |
| 3 | Cash + market value + realized PnL reconciles against ledger | Property tested |
| 4 | Every pre-trade check evaluates in declared fixed order, all results recorded including passes | Audit-log inspection |
| 5 | Paper-broker rejections and partial fills exercised and handled | Test suite |
| 6 | Journal entries link to evidence chain that produced the thesis | UI review |
| 7 | Wallet-sourced cost basis visibly marked in UI wherever P&L derived from it | UI review |
| 8 | Reconciliation classifies divergence correctly across 20+ seeded cases | Test suite |
| 9 | Irreconcilable divergence halts the session and notifies user | Chaos test |

### Risks specific to Wave 4

| Risk | Response |
|---|---|
| `P6-DK-01` (audit-as-write-path) is the highest-risk item in the wave | Build ledger write path alongside conventional path first; compare continuously for a full wave; remove conventional path only once divergence has been zero across the wave. Cutting over on day one is the version of this that goes wrong. |
| Reconciliation state machine has more edge cases than expected | The nofx production pattern (incremental fill sync from watermark, multi-method symbol detection, exponential backoff capped at 5min) is well-documented; start there. |

## IV.7 Wave 5 — Controlled Execution

**Objective.** Live order submission through the deterministic execution boundary. **Full retail product.**

**Entry criteria.** Wave 4 exit gate. **This wave does not open on schedule pressure. It opens when Waves 0–4 have passed their gates.**

**Duration.** ~46 days.

### Work items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P7-QM-01 | QM | `BrokerGateway` trait, `SubmissionResult` including `Unknown` variant | 4 | M |
| P7-QM-02 | QM | Idempotency-key generation, collision detection, replay safety | 4 | M |
| P7-QM-03 | QM | Reconciliation: delta computation, quarantine, resolution, ledger append | 7 | L |
| P7-QM-04 | QM | First broker adapter (Alpaca), full lifecycle including cancel/replace | 8 | L |
| P7-QM-05 | QM | Broker error taxonomy: `BrokerErrorCode` with permanence bit, `Connecting` state, classify `MARKET_CLOSED` before `AUTH` (from OpenAlice, clean-room) | 4 | M |
| P7-QM-06 | QM | Fill-stream handling, partial fills, out-of-order events | 5 | M |
| P7-SS-01 | SS | Step-up authorization, live-execution enablement, privilege audit | 5 | M |
| P7-SS-02 | SS | Emergency stop: cancel-all, flatten, halt automation, tested runbook | 4 | M |
| P7-EX-01 | EX | Order ticket, intent review, risk-decision display, approval flow | 6 | M |
| P7-OD-01 | OD | Execution runbooks, incident procedures, reconciliation playbook | 3 | M |

### Definition of Done (Wave 5)

| # | Criterion | Verified by |
|---|---|---|
| 1 | Submission timeout produces `Unknown`, quarantines instrument, does not retry | Negative test |
| 2 | Reconciliation resolves divergence and appends resolution to ledger | Test |
| 3 | Failed reconciliation leaves instrument quarantined and notifies user | Negative test |
| 4 | Same idempotency key submitted twice produces exactly one broker order | Property test |
| 5 | Live execution requires explicit enablement + step-up auth, both audited | Audit-log inspection |
| 6 | Emergency stop halts automation and cancels working orders within stated time bound | Timed test |
| 7 | Every risk denial recorded with portfolio snapshot at evaluation time | Audit-log inspection |
| 8 | Chaos test: broker disconnection mid-submission handled without duplicate orders | Chaos test |

### Risks specific to Wave 5

| Risk | Response |
|---|---|
| The `Unknown` path is the item separating a working execution system from a dangerous one, and it is the one most likely under-tested | Build a broker simulator that produces timeouts, duplicate acks, out-of-order fills, post-submission disconnects on demand. Make it part of CI. This is `P7-QM-03` and why it carries 7 days. |

## IV.8 Wave 6 — Enterprise and Team

**Objective.** Multi-user deployment, tenancy, air-gapped operation, organizational model governance. **Second business.**

**Entry criteria.** Wave 5 exit gate **plus a signed enterprise customer or credible pipeline. Do not build this speculatively.** ~84 days of enterprise infrastructure with no enterprise customer is the most expensive mistake available in this plan.

**Duration.** ~84 days.

### Work items (summary; see v1.0 Phased Plan §12 for detail)

PostgreSQL backend, multi-tenant isolation (row-level security, tenant-scoped keys), NATS JetStream event transport, S3-compatible object storage, OpenLineage projection. OIDC + passkey auth, RBAC + ABAC policy engine. Vault/KMS/HSM integration. Air-gapped build (vendored crates, private registry, offline artifact staging). Container images, Kubernetes manifests, Terraform. Admin console. Enterprise observability. Organization-specific TSFM fine-tuning through the governance pipeline. Temporal evaluation ADR + possible durable-workflow migration.

### Definition of Done (Wave 6)

Two tenants cannot observe each other's data under any query path (red-team pass). Air-gapped install completes from staged artifacts with no network. Audit export produces a verifiable bundle. Fine-tuned models pass the same registration gates as pretrained ones, including license class and contamination review.

## IV.9 Wave 7 — Ecosystem and Marketplace

**Objective.** Third-party extension and published contracts. **Third business.**

**Entry criteria.** Wave 6 exit gate **plus demonstrated third-party demand.**

**Duration.** ~41 days.

Plugin marketplace (submission, review, signing, revocation), capability review workflow, publisher identity. Publish plugin SDK, chart contracts, indicator trait, manifest schema under Apache-2.0. Public documentation site, examples, contribution guide. Third-party manifest verification service + public tree-head publication. In-application marketplace surface with capability disclosure before install.

**Definition of Done:** a third party can build, sign, submit, publish a plugin without Mythos Systems writing code. A third party can verify a PRISMATIK research bundle using only the published schema + standalone verifier, with no PRISMATIK installation.

## IV.10 Continuous Tracks (every wave)

Four activities run across every wave and are budgeted at a percentage rather than as discrete items, because scheduling them as tasks guarantees they get cut.

| Track | Budget | Content |
|---|---|---|
| Security maintenance | 5% of every wave | Advisory response, dependency updates, pinned bumps, threat-model revision |
| Documentation | 8% of every wave | ADRs, API docs, runbooks, model cards, the workbook |
| Test debt | 7% of every wave | Property-test expansion, DST seed-corpus growth, conformance-vector additions |
| Refactoring | 5% of every wave | Boundary corrections, crate splits, naming consistency |

**25% overhead.** Not padding — a plan that omits it produces the same total with worse quality and a demoralizing final third. The 616-day total already includes this overhead inside per-item estimates.

---

# Part V. Cross-Cutting Concerns

The things that don't fit cleanly into any wave because they run through all of them.

## 21. The Reference Corpus Operating Procedure

The 21 projects in `reference/` are a **consolidation corpus for pattern research**, not embed targets. This reframes the v1.1 license pressure but does not eliminate it.

| License class | Read | Quote in ADRs | Adapt code | Vendor files |
|---|---|---|---|---|
| MIT / Apache-2.0 (Kronos, AIAlpha, ai-trading-claude, ai-algotrading-agent, QuantDinger, adata, stockbot-on-groq) | freely | freely, with attribution | permitted with NOTICE | permitted |
| LGPL-3.0 (OctoBot libraries) | freely | with attribution | dynamic link OK | link, don't vendor |
| AGPL-3.0 / GPL-3.0 (OpenAlice, nofx, ai-market-maker, ai-auto-trading-engine, prism-insight, OpenBB, go-stock, crypto-ai-trading-tool) | freely | name source, do not paste code | clean-room only | no |
| Commons Clause (pybroker) | freely | cite methods, not code | no | no |
| CC BY-NC-SA 4.0 (stocks-insights-ai-agent) | freely | with attribution | no (NC + SA) | no |
| No LICENSE (AI-Trader, aiagents-stock) | freely | all-rights-reserved | no | no |

**The single rule, unchanged:** every pattern taken gets an ADR that names the observed source and either attaches attribution (permissive) or asserts clean-room reimplementation (copyleft). The corpus is build-excluded and SBOM-excluded; it is read-only for design inspiration. **All local root `LICENSE` files are untrustworthy** (byte-identical Apache replacements) — license class is determined from SPDX declarations and source copyright headers and recorded in `reference/LICENSE_DISPOSITION.md`.

## 22. Security Architecture

**Key hierarchy:** OS keychain (OSX Keychain, Windows Credential Manager, Linux Secret Service) → device root key → derived key types. All secrets held in `secrecy` wrappers and `zeroize`d on drop. OAuth tokens and API keys live in the keychain via `prismatik-security`, never in process env beyond the moment of use. OAuth refresh is the auth module's job, not the router's.

**Capability enforcement:** Tauri capability policy default-deny with strict CSP (no inline/eval). Tauri patched ≥2.12 per CVE-2026-42184 (remote-URL-as-trusted-local-origin authentication bypass on Windows/Android). IPC boundary treated as untrusted with defense-in-depth. WASM plugin host capability-scoped: the host provides only the imports a plugin's granted capabilities entitle it to.

**Adversarial AI evaluation:** AgentDojo-style prompt-injection suite over filings, news, market commentary. **A malicious 8-K is a realistic attack vector for a filings-aware product.** LlamaFirewall-pattern guards at every tool-calling boundary. OWASP Top 10 for Agentic Applications 2026 (ASI01 Goal Hijack, ASI02 Tool Misuse/Excessive Agency) as the baseline. Inspect AI as the CI/CD red-team eval harness.

**Supply chain:** `cargo audit` (blocking on advisory), `cargo deny` (license allowlist + banned crates), `cargo vet` (mandatory for trusted-core and Determinism Kernel crates; Mozilla/Google/Bytecode Alliance audit sets imported), `cargo auditable` (release builds), `cargo cyclonedx` (every release artifact), cosign/Sigstore keyless signing, SLSA provenance **generated and verified** by the updater. **SBOMs are compliance/inventory artifacts, not defenses** — the controls that resist attack are the reviewed lockfile, cargo vet, artifact signing, and provenance verification at install time.

## 23. Observability and Operations

**Distributed tracing** via OpenTelemetry with determinism telemetry attributes (run_id, pinned_artifact_set digest, clock_kind, entropy_stream_path) on every span. This is what makes a failing DST seed reproducible on a developer machine.

**Observability as a separate, always-present compose file** (from QuantDinger, Apache-2.0): Prometheus v3.12, Alertmanager, Grafana 13, postgres-exporter, redis-exporters. All bound to 127.0.0.1. Adopt for the Team Cloud and Enterprise profiles.

**Audit export** produces a verifiable bundle: manifest + pinned-artifact refs with hashes + metrics + audit inclusion proof + dual signature. A third party with the public verification key and the published manifest schema verifies the bundle without PRISMATIK, without network, without trusting Mythos Systems.

## 24. The Three Profiles and What Ships

| Component | Personal Desktop | Team Cloud | Enterprise On-Prem | Disconnected Research |
|---|:---:|:---:|:---:|:---:|
| SQLite (operational state, task graph) | ✓ | ✓ | ✓ | ✓ |
| DuckDB + DataFusion (analytics) | ✓ | ✓ | ✓ | ✓ |
| Parquet on local disk (raw + curated) | ✓ | ✓ | ✓ | ✓ |
| LanceDB (embeddings, analog index) | ✓ | ✓ | ✓ | ✓ |
| PostgreSQL (multi-user, RBAC, tenancy) | — | ✓ | ✓ | optional |
| ClickHouse (tick and flow scale) | — | ✓ | ✓ | optional |
| NATS JetStream (durable events) | — | ✓ | ✓ | — |
| S3-compatible object store | — | ✓ | ✓ | — |
| Sidecars (CCXT, QuantLib, Qlib, MAPIE) | opt-in | ✓ | ✓ | pre-staged |
| TSFM runtime | ONNX INT8 CPU | GPU service | GPU or quantized CPU | pre-staged signed artifacts |
| LM Studio (local AI inference) | ✓ | optional | optional | ✓ (air-gapped) |

**Shipping six engines to a single-user desktop is a defect.** A desktop install ships SQLite, DuckDB, Parquet, and LanceDB only.

---

# Part VI. Risk Register

Risks the architecture and wave plan must navigate. Each carries a response, not just an acknowledgment.

## Strategic Risks

| # | Risk | Likelihood | Impact | Response |
|---|---|:---:|:---:|---|
| R1 | Wave 1 wedge (crypto intelligence) does not find buyers | M | H | Wave framework designed for cheap pivot to Wave 2 (equity/options) at ~128-day loss rather than ~400-day. |
| R2 | Reproducibility claim tested in court/diligence and found weaker than asserted | L | H | Determinism Kernel + DST + golden manifest corpus are the verification. Independent security review before any enterprise customer. |
| R3 | Enterprise buyer never materializes; Waves 6–7 never open | M | M | Waves 1–5 are a complete retail product. Waves 6–7 are speculative until a customer signs. |
| R4 | PRISMATIK's own IP posture challenged in funding/acquisition diligence due to reference-corpus exposure | M | H | Clean-room ADR discipline; corpus build-excluded and SBOM-excluded; `LICENSE_DISPOSITION.md` documents what was read and when. |

## Technical Risks

| # | Risk | Likelihood | Impact | Response |
|---|---|:---:|:---:|---|
| T1 | Audit-as-write-path (`P6-DK-01`) cutover introduces data corruption | M | H | Run ledger-write and conventional-write paths in parallel for full Wave 4; remove conventional only after zero divergence across the wave. |
| T2 | TSFM inference misses 500ms desktop budget | M | M | Fall back to Chronos-Bolt (distilled). If still misses, TSFM is Team Cloud only; desktop ships classical ladder. Profile table supports this. |
| T3 | DSL overruns >50% and blocks Wave 3 | M | M | Build type checker + IR first; parser second. If overrun, ship Wave 3 with visual builder + Rust SDK only; DSL to Wave 3.5. |
| T4 | wasmtime sandbox-escape advisory during Wave 3 | M | H | Pinned version + blocking `cargo deny`. Config hardening already in place (Cranelift only, signals-based-traps, guard pages, fuel, epoch). Advisory = same-week patch, not redesign. |
| T5 | LM Studio structured-output reliability insufficient for T0/T1 small models | M | M | Constrained decoding mandatory; reject anything failing schema validation. Fallback to Groq-hosted Llama 4 Scout for T0/T1 if local proves unreliable. |
| T6 | Reconciliation state machine has more edge cases than nofx reference suggests | M | M | Start from documented nofx pattern; build broker simulator that produces timeouts, duplicate acks, out-of-order fills, post-submission disconnects on demand in CI. |
| T7 | Bitemporal symbology + corporate-action edge cases (OCC-adjusted contracts, mergers, spin-offs) produce wrong backtests silently | M | H | Hand-checked set of 50 known identity events as Wave 2 exit gate. `adjusted_contract` HardDeny pre-trade check. OptionAdjustment in corporate-action ledger. |
| T8 | ML-DSA (post-quantum signature) implementation maturity in Rust uneven; dual-signature scheme delayed | H | M | Time-box `P0-DK-10` to 8 days. Ship Ed25519-only with dual-signature *format* in place and ML-DSA field empty. Retrofit is field population, not schema migration. PQC register exception. |
| T9 | Tauri IPC trust boundary CVE (CVE-2026-42184 class recurs) | M | H | Patched ≥2.12; defense-in-depth; treat IPC as untrusted; recurring pentest (Radically Open Security performed one for Tauri publicly). |
| T10 | io_uring ZCRX networking features carry CVEs (CVE-2026-43121 class) | M | M | io_uring for storage production-ready; for networking treat as maturing. Use thread-per-core pattern only on storage path initially; networking on Tokio. |

## Operational Risks

| # | Risk | Likelihood | Impact | Response |
|---|---|:---:|:---:|---|
| O1 | Solo operation cannot sustain 616-day plan; burnout or revenue pressure forces cuts | H | H | Option B (revenue-first with determinism floor). Wave 1 ships at ~80 days. Wave boundaries are designed pivot points. |
| O2 | Data provider pricing (Polygon, Unusual Whales, etc.) makes the product uneconomic | M | H | Model data costs into pricing before each wave opens. Provider plane supports BYO-key; user can route through their own subscriptions. |
| O3 | Regulatory change (SEC PDA enforcement, EU AI Act high-risk classification, CFTC crypto rules) forces architecture change | M | M | Compliance plane designed for transparency obligations from Wave 1. Model registry already satisfies most MRM guidance. Monitor SR 26-2 GenAI-specific follow-on guidance. |
| O4 | Reference corpus contaminated by pasting code rather than clean-room reimplementation | M | H | Build-excluded path; SBOM-scanning exclusion; ADR-024+ discipline; quarterly IP review. |

---

# Part VII. Success Metrics and Wave Review

## 25. How We Know a Wave Is Done

Every wave closes with a written review, not a feeling. The review is `docs/gates/wave-N.md` containing:

1. Each exit criterion, its verification artifact, pass/fail.
2. Every waiver, with reason, risk accepted, and the wave by which it must be resolved. A waiver without a resolution wave is not a waiver; it is a silent scope cut.
3. Estimate vs actual per work item — the only way the estimates improve.
4. Items deferred to a later wave with original identifiers.
5. New risks discovered, added to Part VI.
6. A go/no-go decision on the next wave, with reasoning recorded.

**Recalibrate after Wave 0.** Estimates are anchored on judgment, not measured throughput. Wave 0 produces the first real velocity data. Multiply every subsequent estimate by the Wave 0 actual/estimate ratio and update this document. A plan never recalibrated is a plan wrong for progressively longer.

## 26. Commercial Metrics (per wave)

| Metric | Wave 1 target | Wave 2 target | Wave 3 target | Wave 4+ target |
|---|---|---|---|---|
| Paying customers | 10+ within 90 days of ship | 50+ cumulative | 200+ cumulative | 500+ cumulative |
| Revenue | Validation only | Covers data costs | Covers one engineer | Funds team growth |
| Retention (30-day) | >40% | >50% | >60% | >70% |
| Verifiable research bundles exported | n/a | n/a | 100+ in wild | 1000+ in wild |
| Enterprise pipeline | n/a | n/a | 3+ conversations | 1+ signed |

**These are hypotheses to test, not commitments.** Wave 1's purpose is to test whether the crypto wedge validates at all; the specific numbers matter less than the direction.

## 27. Engineering Quality Metrics

| Metric | Target | How measured |
|---|---|---|
| DST seed corpus | 256+ seeds passing byte-identical | CI green |
| Property-test coverage | Every load-bearing trait has ≥1 property test | CI inventory |
| Conformance (indicators) | Streaming equals batch for all indicators | Property test |
| Conformance (pricing) | QuantLib within 1e-8 relative | Conformance report |
| Audit append latency | <1ms p99 | `criterion` benchmark |
| Cold start to interactive | <2.0s p95 mid-tier hardware | Benchmark report |
| Chart at 1M points | 16.7ms p99 frame (60fps) | Benchmark report |
| Analog search p95 | <120ms across 10M vectors | Benchmark report |
| TSFM forecast p95 latency | ≤500ms single-asset single-modality | Benchmark report |
| Allowed dependencies outside registry | 0 | `registry-coverage` green |
| Determinism-grep violations outside allowlist | 0; allowlist <10 entries | CI gate |

---

# Part VIII. Explicitly Not Now

Recording what is deliberately excluded is as valuable as recording what is included, because it prevents the same debate recurring quarterly.

| Item | Source | Not now because | Revisit |
|---|---|---|---|
| Causal market graph | v0.1 §37.1 | Research project, not product work | Post Wave 7 |
| Market digital twin | v0.1 §37.2 | Depends on microstructure data PRISMATIK lacks | Post Wave 7 |
| Federated strategy learning | v0.1 §37.3 | Requires multiple enterprise customers | Post Wave 6, demand-driven |
| Confidential computing / TEE broker integration | v0.1 §37.4 | Research-stage for LLM workloads; mature for trading engines only | 2027+ pilot |
| Multi-agent research council | v0.1 §37.7 | Single AI analyst not yet proven | Post Wave 4 |
| Pine Script compatibility | v0.4 §12 | Provenance + licensing risk exceeds value | Post Wave 7, if ever |
| TradingView Advanced Charts | v0.4 §8 | Separately licensed, not a foundation | Only if customer requires |
| NautilusTrader adoption | v0.4 §28 | LGPL implications unresolved; owns owned boundaries | Reference only, permanently |
| Temporal orchestration | v0.2 §7.1 | In-process task graph sufficient below Wave 6 | Wave 6, ADR-0036 |
| Additional TSFM families | v1.0 §6.7 | Five artifacts prove the registry; more is dilution | Wave 6 |
| Mobile client | not in corpus | No coherent mobile use case for dense analytical workspace | Demand-driven |
| io_uring on networking path | 2026 research | Storage path production-ready; networking ZCRX carries CVE-2026-43121-class risk | Post Wave 5 evaluation |
| Thread-per-core migration of full runtime | 2026 research | Tokio sufficient for control path; thread-per-core proven for storage data path only | Scoped evaluation Wave 4+ |

---

# Part IX. The Honest Total

The plan is ~616 engineering-days. At realistic solo capacity (2–3 focused days/week), that is five to seven years to Wave 7. That number is not an argument against the architecture. It is an argument for reading Part IV.1 (Option B) before Part IV.2 (Wave 0).

**The wave framework exists to test the central commercial hypothesis at Wave 1 (~80 days under Option B) rather than at Wave 3 (~335 days under Option A).** If Wave 1 finds no buyers, the loss is bounded at ~128 days. If it finds buyers, every subsequent wave is funded by revenue rather than by sunk-cost faith.

Three observations that should shape every decision:

**First, Wave 0 produces nothing a customer can see.** ~72 days of determinism kernels, Merkle ledgers, and CI gates. Option B reduces the floor to ~24 days by deferring everything additive. The floor cannot be deferred.

**Second, the waves are not equally optional.** Waves 0–2 build a coherent, sellable intelligence product. Waves 3–4 build the quantitative platform. Waves 5–7 build the execution, enterprise, and ecosystem businesses. These are three products with three different buyers. The plan should stop treating them as one continuous ramp.

**Third, the architecture's own logic argues for front-loading.** Section 11 of the architecture is right that retrofitting determinism costs multiples of building it first. That logic is sound and also exactly the logic that produces a 72-day gap before first value. Both things are true. The wave framework resolves the tension.

---

*End of PRISMATIK Enterprise Overview, Vision, and Wave Plan.*

*Author: Aaron Stovall · Mythos Systems · 2026-07-26 · Version 1.0*
