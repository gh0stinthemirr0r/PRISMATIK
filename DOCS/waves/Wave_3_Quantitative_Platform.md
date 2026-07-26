# Wave 3 — Quantitative Research Platform

**Maps to v1.0 phases:** P4 (Strategy, Backtest, Indicator Kernel, Plugin Host) + P5 (Simulation, TSFM Registry, Calibration) + P5.5 (Analog Engine Upgrade)
**Business outcome:** the defensible research platform. Verifiable research artifacts third parties can validate without installing PRISMATIK.
**Duration:** ~153 days (Wave 3A ~78 days + Wave 3B ~61 days + Wave 3C ~14 days).
**Date:** 2026-07-26

---

## Objective

Strategy authoring in three modes (visual builder, DSL, Rust SDK) plus Python research-mode emission, all converging on one `StrategyIR`. Deterministic backtesting on pinned-calendar bar boundaries. The canonical indicator kernel with golden vectors. The sandboxed wasmtime plugin host. The plural TSFM registry. Conformal calibration. The analog engine. By the end of Wave 3, the platform produces calibrated forecasts and signed reproducibility manifests that a standalone verifier accepts.

## Entry Criteria

Wave 2 exit gate **and, under Option B, complete `P0-REMAINDER`. This gate is hard.** Wave 3 is the first wave where a missing manifest or audit ledger is a correctness failure rather than a missing feature. **If `P0-REMAINDER` is not complete when Wave 3 opens, Wave 3 does not open.**

## Why This Wave Has the Strictest Gate

Every criterion in Wave 3's Definition of Done is enforced by a test, not by review. The reason: Wave 3 is where the platform's central claims (Invariants I2, I3, I4) become operational. A model that surfaces a forecast without a calibration record, a TSFM registry that accepts a CC BY-NC artifact in commercial profile, a backtest that contaminates its window with a model whose pretraining cutoff overlaps the test period, or a plugin with no granted capabilities that nevertheless reaches a socket — any of these is a defect in the product's central promise, not a missing feature.

---

## Wave 3A — Strategy, Backtest, Indicator Kernel, Plugin Host (~78 days)

### Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P4-QM-01 | QM | `StrategyIR` types, `StrategyCapabilities`, serialization, schema version | 4 | M |
| P4-QM-02 | QM | DSL lexer on `logos`, recursive descent parser, diagnostics with spans | 8 | L |
| P4-QM-03 | QM | DSL type checker and capability inference from source references | 6 | L |
| P4-QM-04 | QM | Data access compilation to DataFusion logical plans | 6 | L |
| P4-QM-05 | QM | Point-in-time enforcement as a DataFusion plan rewrite rule | 4 | L |
| P4-QM-06 | QM | `Strategy` trait, runtime, `StrategyContext` over `DeterminismContext` | 5 | M |
| P4-QM-07 | QM | Backtest engine: event loop over pinned-calendar bar boundaries | 7 | M |
| P4-QM-08 | QM | `ExecutionAssumptions`, fill models, slippage, commission, assignment | 6 | M |
| P4-QM-09 | QM | Backtest metrics, walk-forward, out-of-sample, deflated Sharpe, **purged + embargoed (AFML)** | 6 | M |
| P4-QM-10 | QM | `Indicator` trait, warmup enforcement, `IndicatorDescriptor`, provenance | 4 | M |
| P4-QM-11 | QM | YATA adapter plus the first 30 indicators with golden vectors | 8 | M |
| P4-QM-12 | QM | `BarSampler` (time/tick/volume/dollar) — from AIAlpha, clean-room with attribution | 3 | M |
| P4-QM-13 | QM | Streaming equals batch property test across the full indicator set | 2 | H |
| P4-QM-14 | QM | Two-implementation conformance harness with ai-algotrading-agent fixtures (MIT) | 3 | M |
| P4-SS-01 | SS | `prismatik-plugin-host`: wasmtime hardened engine config plus config assertion test | 4 | M |
| P4-SS-02 | SS | `CapabilitySet`, host function registry, time and entropy from the kernel | 5 | M |
| P4-SS-03 | SS | Plugin signing, install flow, capability grant recorded in the audit ledger | 4 | M |
| P4-SS-04 | SS | Capability-diffing gate on imports/exports + cosign verification at load | 3 | M |
| P4-SS-05 | SS | Capability containment test: syscall trace proving zero network from a denied plugin | 3 | L |
| P4-EX-01 | EX | Visual strategy builder emitting `StrategyIR` via codegen | 8 | L |
| P4-EX-02 | EX | Backtest result workspace: equity curve, drawdown, trade list, metrics | 6 | M |
| P4-EX-03 | EX | Rust SDK crate and documentation for hand-written strategies | 4 | M |
| P4-QM-15 | QM | Python `StrategyIR` emitter in the research sidecar, JSON schema and validator | 4 | M |

### Design Rules Enforced From the Start

- **One IR, three authoring modes (plus Python research-mode emission).** Python is not an authoring mode for executable strategies — it is a research mode that *emits* `StrategyIR` in the sidecar, which then executes on the Rust runtime. Reason: Invariant I3 (determinism). A Python strategy executing inside a backtest brings an entire interpreter's worth of ambient nondeterminism into the deterministic core. Researchers keep their tooling; the runtime keeps its guarantees.
- **The Strategy DSL is non-Turing-complete.** Purpose-built, hand-written recursive-descent parser over a `logos` lexer, compiling to DataFusion logical plans for data access and `StrategyIR` rule trees for logic. Non-Turing-complete is the most important word: a strategy language with unbounded loops needs fuel metering, a timeout, and a story about what a half-executed strategy means for portfolio state. A language where every expression terminates by construction needs none of those.
- **Point-in-time enforcement is a DataFusion plan rewrite rule**, not a caller-discipline convention. A strategy author cannot bypass it by writing the query differently.
- **Purged + embargoed walk-forward** (from AIAlpha, clean-room): drop training samples whose label horizon overlaps the test window (purging) and drop a buffer after the test window (embargo). Without these, overlapping labels leak across the split and every out-of-sample number is optimistic.
- **`BarSampler`** (from AIAlpha, clean-room): time/tick/volume/dollar. Information-driven bars (dollar bars sample on cumulative traded value, producing returns closer to IID) matter more for crypto than equities, and v1.0's indicator kernel had no bar-sampling primitive.
- **Plugin supply chain:** wasm binaries are harder to review than source. Gate with `wasm-tools` disassembly, **capability diffing on imports/exports between versions** (a plugin update that newly imports a network capability must fail the gate automatically, not await human review), and cosign verification before load.

---

## Wave 3B — Simulation, TSFM Registry, Calibration (~61 days)

### Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P5-QM-01 | QM | Monte Carlo lab: `SimulationSource` variants, bootstrap, block bootstrap, parametric | 6 | M |
| P5-QM-02 | QM | Jump diffusion, stochastic volatility, regime switching sources | 5 | M |
| P5-QM-03 | QM | Terminal value, drawdown, and probability of ruin distributions with seed pinning | 4 | M |
| P5-QM-04 | QM | Deterministic parallel reduction under Rayon, split entropy per path | 4 | L |
| P5-QM-05 | QM | `ModelRegistry`, `ModelRegistration`, license class gate with hard deny | 4 | M |
| P5-QM-06 | QM | `PretrainingRecord`, contamination gate, security event on denial | 3 | M |
| P5-QM-07 | QM | `SeriesTokenizer` trait, codebook artifacts, `TokenizerBinding` validation | 5 | M |
| P5-QM-08 | QM | `TsfmRuntime` trait, ONNX runtime adapter, INT8 quantized CPU path | 7 | L |
| P5-QM-09 | QM | Kronos + Chronos-2 + TiRex-2 + Toto 2.0 + Lag-Llama artifact onboarding | 7 | L |
| P5-QM-10 | QM | Forecast caching keyed on asset, modality, and context hash | 3 | M |
| P5-QM-11 | QM | `Calibrator` trait, `CalibrationRecord`, `CalibrationMethod` types | 4 | M |
| P5-QM-12 | QM | Calibration sidecar: MAPIE ACI and EnbPI, crepes Mondrian, Arrow transport | 6 | L |
| P5-QM-13 | QM | Baseline ladder enforcement at promotion, both discrimination and calibration | 4 | M |
| P5-QM-14 | QM | `DriftDetector` suite: feature, calibration, embedding, token usage, performance | 6 | L |
| P5-QM-15 | QM | Drift actions: annotate, widen, suppress, propose demotion | 3 | M |
| P5-EX-01 | EX | Monte Carlo visualization: path clouds via wgpu, distribution panels | 5 | M |
| P5-EX-02 | EX | Calibration ribbon primitive, realized coverage against nominal | 4 | M |
| P5-EX-03 | EX | Model card surface, drift status, blind spot disclosure | 4 | M |

### The Plural TSFM Registry (updated for 2026)

Five model families onboard through the same trait, each with a distinct role:

| Artifact | License | Role |
|---|---|---|
| **Kronos** | MIT | K-line specialist + generative path synthesizer for Monte Carlo + tokenizer-as-embedding source |
| **Chronos-2** | Apache-2.0 | General + covariate-informed forecasts; current general-purpose leader on fev-bench/GIFT-Eval |
| **TiRex-2** | Apache-2.0 | xLSTM streaming, covariate-aware, constant-cost live tick forecasts |
| **Toto 2.0** | Apache-2.0 | Scaling option (5 sizes 4M→2.5B; first TSFM that reliably improves with scale) |
| **Lag-Llama** | Apache-2.0 | Probabilistic baseline for calibration comparison |
| **Moirai / Moirai-2** | CC BY-NC 4.0 | **REJECTED at commercial registration.** Research benchmarking only. |

Per-frequency-band baseline benchmarking against Auto-Theta / ARIMA / GARCH is non-optional. Foundation models get relatively stronger as frequency drops, so do not assume a TSFM beats a classical baseline at intraday resolution.

### The Baseline Ladder Is a Registration Precondition

A model cannot be promoted to any user-facing surface unless the registry holds an out-of-sample comparison against all lower rungs and the candidate wins on **both** discrimination and calibration. Winning on discrimination alone is insufficient — a model that ranks better but is worse calibrated is a model that will be trusted more than it deserves, which is the precise failure the evidence plane exists to prevent.

| Rung | Family | Must beat |
|---|---|---|
| 0 | Naive persistence, random walk, unconditional mean | nothing, this is the floor |
| 1 | Classical statistical (ARIMA, GARCH, EWMA volatility) | rung 0 |
| 2 | Regularized linear, gradient boosted trees | rungs 0 and 1 |
| 3 | Task-specific neural network | rungs 0 through 2 |
| 4 | TSFM, zero-shot or fine-tuned | rungs 0 through 3 |
| 5 | Ensemble | every constituent, individually |

---

## Wave 3C — Analog Engine Upgrade (~14 days)

### Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P55-QM-01 | QM | Embedding extraction from `TsfmRuntime::embed`, batch materialization (**Kronos tokenizer first** — closes G06) | 3 | M |
| P55-QM-02 | QM | `AnalogStore` production implementation, LanceDB IVF-PQ index, version pinning | 4 | M |
| P55-QM-03 | QM | `AnalogQuery` with secondary filters, regime, sector, event type, venue | 3 | M |
| P55-QM-04 | QM | Leave-N-out sensitivity computation | 2 | M |
| P55-EX-01 | EX | Analog result surface with mandatory disclosures and sensitivity display | 2 | M |

### The Three Uses of Kronos

The Integration Guide (v1.1 §1.2) is right: **adopt (c) first, then (b), then (a).**

- **(a) Point/quantile forecasting.** Standard TSFM use. The crowded, least-differentiated use — Chronos-2 competes directly here.
- **(b) Generative path synthesis for the Monte Carlo lab.** Kronos reports +22% generative fidelity on synthetic K-line sequences. Because it is autoregressive over a learned financial token space, sampling it produces *regime-plausible* paths rather than the parametric or bootstrap paths in your simulation plane. Uncorrelated failure modes are the entire point of the Simulation Comparison surface.
- **(c) The tokenizer as a standalone embedding source.** Closes G06 with a domain-appropriate embedding for LanceDB analog search. Much cheaper than running the predictor, and directly closes a load-bearing gap.

---

## Definition of Done

This is the strictest gate in the plan. Every criterion is enforced by a test, not by review.

| # | Criterion | Verified by |
|---|---|---|
| 1 | No forecast reaches the UI without an attached `CalibrationRecord` | Type system, no constructor exists that omits it |
| 2 | The registry hard denies a CC BY-NC artifact in a commercial profile | Negative test with a synthetic Moirai-style registration |
| 3 | The backtest runtime hard denies a model whose pretraining cutoff violates the window, and logs a security event | Negative test plus ledger inspection |
| 4 | A tokenizer and model version mismatch fails to load rather than degrading silently | Negative test |
| 5 | A rung-4 model that loses to rung 1 on calibration cannot be promoted | Negative test |
| 6 | Embedding drift above threshold automatically widens intervals with no human action | Simulated drift test |
| 7 | Monte Carlo with 1M paths under Rayon reproduces byte identically across runs **and thread counts** | Determinism test at 1, 4, and 16 threads |
| 8 | Per-regime coverage is displayed alongside every forecast | UI review |
| 9 | TSFM forecast p95 latency is at or under 500 ms single-asset single-modality | Benchmark report |
| 10 | The same logical strategy authored in the visual builder, the DSL, and the Rust SDK produces identical `StrategyIR` | Three-way comparison test |
| 11 | Those three produce identical backtest results | Byte comparison of result-bundle hashes |
| 12 | A strategy compiled without `can_access_network` cannot reach a socket even when its code attempts | Negative test with a deliberately malicious strategy |
| 13 | `evaluate` is bitwise identical to the `next` sequence for all 30 indicators | Property test |
| 14 | A plugin with no granted capabilities makes zero syscalls of the network or filesystem class | `strace` or ETW trace, committed |
| 15 | Relaxed SIMD is disabled and the engine config assertion test passes | Committed test |
| 16 | A backtest emits a complete signed manifest that the standalone verifier accepts | End-to-end run |
| 17 | `CloseOnly` fill-model results carry a mandatory badge in the UI | UI review |
| 18 | Full `dst_replay_suite` over ingest to manifest passes 256 seeds | CI green |
| 19 | Analog search returns neighbours only alongside distance metric, sample size, applied filters, survivorship warning, and leave-N-out sensitivity, with no code path that renders neighbours without them | UI review + code audit |
| 20 | Search of the top 50 across 10M vectors completes within 120 ms p95 | Benchmark report |

**Criterion 7 deserves emphasis:** byte-identical results across *different thread counts* is the property that proves the split-entropy design works. Same-thread-count reproducibility is much weaker and much easier to achieve accidentally.

## Risks

| Risk | Response |
|---|---|
| The DSL is the largest low-confidence cluster in the plan, 24 days across four items | Build the type checker and IR **first** and the parser second. A parser targeting a proven IR is bounded work; a parser and an IR designed together is not. If the DSL overruns by more than 50 percent, ship Wave 3 with the visual builder and Rust SDK only and move the DSL to Wave 3.5. |
| wasmtime advisory during the wave | Pinned version plus blocking `cargo deny`. An advisory is a same-week patch, not a redesign, because the config hardening is already in place. |
| Visual builder scope expands without limit | The builder emits `StrategyIR` and nothing else. Any capability not expressible in the IR is out of scope by construction. |
| TSFM inference on desktop CPU misses the 500 ms budget | Fall back to Chronos-Bolt, which is the distilled variant and substantially faster. If that also misses, TSFM becomes a Team Cloud feature and the desktop profile ships the classical model ladder only. The architecture already supports this through the profile table. |
| Kronos becomes unmaintained mid-wave | Plural registry already mitigates. Chronos-2/TiRex-2/Toto 2.0 are Apache-2.0 and integrated through the same trait. |
| Conformal calibration is applied incorrectly to non-exchangeable data | ACI is the default by type; standard split conformal is restricted to cross-sectional tasks. Per-regime coverage reporting makes a misapplication visible rather than silent. |
| The calibration sidecar adds a Python runtime dependency to the desktop install | Sidecar is opt-in on desktop per the profile table. Calibration records can be computed on a schedule and shipped as signed artifacts rather than computed locally. |

## Commercial Metrics

| Metric | Wave 3 target |
|---|---|
| Paying customers | 200+ cumulative |
| Revenue | Covers one engineer |
| Retention (30-day) | >60% |
| Verifiable research bundles exported | 100+ in the wild |
| Enterprise pipeline | 3+ conversations |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26*
