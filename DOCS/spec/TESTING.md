# PRISMATIK Testing Specification

**Document:** `spec/TESTING.md`
**Status:** NORMATIVE — RFC 2119 keywords apply
**Companion to:** `PRISMATIK_Unified_Solution_Architecture_v1.0.md` §27 (testing layers), wave DoD criteria
**Date:** 2026-07-26

---

## 0. Purpose

This document specifies the **complete test plan**: property test catalog, deterministic simulation testing (DST) corpus, conformance fixtures, golden vectors, integration tests, chaos tests, and adversarial eval suites. Every test in PRISMATIK MUST be enumerated here or in an ADR; ad-hoc tests are acceptable but should migrate to this catalog.

The cardinal rule (v1.0 §27): an invariant without a gate is an aspiration. The seven invariants (I1–I7) each have a CI gate named below.

---

## 1. Test Layers

| Layer | What it verifies | Counted in CI? |
|---|---|---|
| **Unit** | Single function behavior | Yes (all) |
| **Property** | Invariants over random inputs | Yes (all) |
| **Conformance** | Output matches a known-good oracle | Yes |
| **Golden vector** | Output matches committed golden output | Yes |
| **DST (deterministic simulation)** | Replay produces byte-identical traces | Yes (256+ seeds) |
| **Integration** | Multi-crate behavior | Yes |
| **End-to-end** | User-facing scenarios | Yes |
| **Chaos** | Failure injection, network partitions | Yes (subset in CI; full weekly) |
| **Adversarial** | Prompt injection, OWASP ASI01/ASI02 | Yes (regression suite) |
| **Performance** | Benchmarks vs budgets | Yes (`criterion`; regressions are warnings) |
| **Backward-compat** | Old manifests/bundles verify | Yes |

---

## 2. Property Test Catalog

Every property test uses `proptest` (Rust) with at least 1,000 cases (10,000 for load-bearing invariants). Property tests are higher-leverage than contract tests because they verify invariants across input spaces.

### 2.1 Determinism Properties (v1.0 §12, Wave 0)

| Test | Path | Property | Iterations |
|---|---|---|---|
| `entropy_split_order_independence` | `prismatik-determinism/tests/property/entropy.rs` | For any parent seed + label set, sibling streams produce identical values regardless of consumption order | 10,000 |
| `clock_simulated_deterministic` | `prismatik-determinism/tests/property/clock.rs` | SimulatedClock advances only when advanced; same advance sequence → same time sequence | 1,000 |
| `detmap_iteration_order_stable` | `prismatik-determinism/tests/property/detmap.rs` | DetMap iteration order is byte-identical across runs for the same insert sequence | 1,000 |
| `seed_replay_byte_identical` | `tests/property/seed_replay.rs` | For any seed, run twice → identical canonical bytes | 1,000 |

### 2.2 Identity Properties (v1.0 §12.6, Wave 2)

| Test | Property | Iterations |
|---|---|---|
| `bitemporal_resolution_stable` | `resolve_as_of` returns the same `AssetId` for the same `(identifier, as_of)` across runs | 10,000 |
| `identity_chain_total` | For any asset, `identity_chain` covers all transitions; no gaps | 1,000 |
| `corporate_action_invariants` | Split numerator/denominator > 0; spin-off child exists | 1,000 |

### 2.3 Feature Store Properties (v1.0 §15.3, Wave 2)

| Test | Property | Iterations |
|---|---|---|
| `pit_observation_delay` ⭐ | For any view, entity, instant: NO returned value has `event_time + observation_delay > as_of` | 10,000 |

⭐ This is the load-bearing property (Wave 2 DoD criterion 3). It catches look-ahead bias, which is the silent killer of feature stores.

### 2.4 Strategy and Backtest Properties (Wave 3)

| Test | Property |
|---|---|
| `strategy_ir_round_trip` | Visual → IR → JSON → IR → equal to original IR |
| `three_modes_produce_identical_ir` ⭐ | Same logical strategy authored in visual / DSL / Rust SDK → byte-identical IR |
| `three_modes_produce_identical_results` ⭐ | Those three → byte-identical backtest result bundle hashes |
| `backtest_walk_forward_purged` | No training sample's label horizon overlaps test window |
| `backtest_embargo_enforced` | Buffer window after test is empty |
| `monte_carlo_thread_count_independent` ⭐ | 1M paths reproduce byte-identically at 1, 4, 16 threads |

⭐ Wave 3 DoD criteria 7, 10, 11.

### 2.5 Indicator Properties (Wave 3)

| Test | Property |
|---|---|
| `streaming_eq_batch` ⭐ | For all 30 indicators, `evaluate(window) == next_seq_over(window)` bitwise |
| `warmup_returns_nan` | Reading an indicator before its warmup length returns NaN (not partial) |

⭐ Wave 3 DoD criterion 13.

### 2.6 Calibration Properties (Wave 3)

| Test | Property |
|---|---|
| `calibration_coverage_in_bounds` | Realized coverage is within sampling-error bounds of nominal (under exchangeable data) |
| `aci_converges` | ACI coverage converges to nominal over time, even under non-exchangeable data |
| `mondrian_per_regime` | Per-regime coverage reported alongside global |

### 2.7 Audit Ledger Properties (v1.0 §14, Wave 0)

| Test | Property |
|---|---|
| `tamper_detection` ⭐ | Modifying any byte in the audit log breaks either inclusion or consistency proof |
| `append_only_monotonic` | `tree_size` is monotonically increasing; gaps are detectable |
| `replay_rebuilds_projection` | Full ledger replay → identical projection to live state |

⭐ Wave 0 DoD criterion 7.

### 2.8 Execution Properties (Wave 5)

| Test | Property |
|---|---|
| `idempotency_key_single_order` ⭐ | Same key submitted twice → exactly one broker order |
| `unknown_no_retry` | `SubmissionResult::Unknown` does not trigger resubmission; quarantines instead |

⭐ Wave 5 DoD criterion 4.

### 2.9 IPC Properties

| Test | Property |
|---|---|
| `bindings_drift_zero` | Generated TypeScript bindings match Rust source exactly |
| `ipc_input_validation` | Every command rejects malformed input deterministically |

---

## 3. DST Suite (Deterministic Simulation Testing)

### 3.1 Framework

madsim + turmoil (per v1.0 §6.9). The DST suite runs the full ingest → feature → backtest → manifest pipeline under simulated time, simulated network, and injected failures.

### 3.2 Seed Corpus

The seed corpus is a set of seeds (each a `u64`) committed to `tests/dst/seeds.txt`. Each seed runs the full pipeline; the TRACE-level log is captured and hashed.

**Wave 0 floor:** 64 seeds. **Wave 3 target:** 256+ seeds. **Wave 5+ target:** 1,000+ seeds.

### 3.3 Verification Method

Per S2 storage team's approach (v1.0 §6.9): rerun the same seed and diff TRACE logs byte-for-byte. A divergence is a build failure with the failing seed printed for local reproduction.

```bash
# pseudocode for the DST CI job
for seed in $(cat tests/dst/seeds.txt); do
    trace_v1=$(run_pipeline --seed $seed --trace-level TRACE | blake3)
    trace_v2=$(run_pipeline --seed $seed --trace-level TRACE | blake3)
    [ "$trace_v1" = "$trace_v2" ] || { echo "DST DIVERGENCE at seed $seed"; exit 1; }
done
```

### 3.4 Failure Injection

The DST suite injects failures via madsim:
- Network: packet loss, partition, latency spikes.
- Disk: write failures, fsync delays.
- Time: clock skew, monotonic time jumps.
- Process: panic injection, OOM simulation.

The pipeline MUST recover or fail closed; never produce inconsistent state.

---

## 4. Conformance Fixtures (Oracles)

### 4.1 QuantLib Pricing Conformance (Wave 3)

`tests/conformance/pricing/` — 500-case grid of options pricing scenarios.

- **Oracle:** QuantLib sidecar (`services/sidecars/quantlib-oracle/`).
- **Subject:** `prismatik-quant-kernel`.
- **Tolerance:** 1e-8 relative.
- **Wave 3 DoD criterion 1:** all 500 cases within tolerance.

### 4.2 Calendar Cross-Validation (Wave 0)

`tests/conformance/calendar/` — every supported venue × every session.

- **Oracle:** QuantLib's calendar module.
- **Subject:** PRISMATIK's calendar artifact generator.
- **Tolerance:** ZERO disagreements.
- **Wave 0 DoD criterion 12:** zero disagreements.

### 4.3 TSFM Baseline Ladder (Wave 3)

For each TSFM registered, must beat lower rungs on out-of-sample data:
- Rung 0: naive persistence
- Rung 1: ARIMA, GARCH, EWMA
- Rung 2: gradient-boosted trees
- Rung 3: task-specific NN
- Rung 4: TSFM

Each comparison includes BOTH discrimination (CRPS, pinball loss) AND calibration (realized coverage). Winning on discrimination alone is insufficient.

### 4.4 Indicator Two-Implementation Parity (Wave 3)

Per v1.2 §3.7 (from ai-algotrading-agent, MIT): vendor the `hist-10m/*.csv` fixtures; port SMA-cross + trailing-stop to `prismatik-indicator-core`; assert bit-comparable results within declared `NumericalTolerance`.

**Trailing-stop intrabar order is the load-bearing detail:** the engine ratchets `highPrice` BEFORE evaluating exits on the same bar. The fixture must verify this exact ordering — using `Last`, not `High`/`Low` (the known simplification).

---

## 5. Golden Vectors

Golden vectors are committed expected outputs for known inputs. They are versioned alongside the code; a change to expected output is a deliberate commit with a changelog entry.

### 5.1 Indicator Golden Vectors

`tests/golden/indicators/` — one CSV per indicator. Input: a fixed OHLCV series. Expected output: the indicator's value at each step.

When YATA bumps version or the formula changes, regenerate golden vectors in the same commit.

### 5.2 Manifest Golden Corpus

`tests/golden/manifests/` — committed signed manifests with known results. CI re-executes each and asserts byte-identical results.

**Wave 0 floor:** 2 manifests. **Wave 3 target:** 10+ manifests covering each `kind` (backtest, walk_forward, monte_carlo, tsfm_forecast, calibration_fit).

This is the mechanism that catches door 7 (silently upgraded reference data, per v1.0 §12.3): a calendar bump that changes a single session fails the golden corpus loudly.

### 5.3 Strategy IR Golden

`tests/golden/strategy_ir/` — fixed strategies in each authoring mode, with their expected canonical IR JSON.

---

## 6. Integration Tests

### 6.1 Multi-Crate Flows

`tests/integration/` — flows that span crates:
- `tests/integration/ingest_to_feature.rs` — provider → raw → normalized → feature materialization
- `tests/integration/strategy_to_backtest.rs` — author → compile → backtest → manifest
- `tests/integration/draft_to_submit.rs` — draft → risk evaluate → approve → submit (paper)
- `tests/integration/reconciliation_loop.rs` — divergence → reconcile → heal

### 6.2 Provider Contract Tests

`tests/integration/providers/<provider>/` — cassette-replay tests (see `spec/PROVIDER_ADAPTERS.md` §13). Run with network disabled.

---

## 7. End-to-End Tests

`tests/e2e/` — full user-facing scenarios via the IPC layer (or a headless shell around it):

| Scenario | Path |
|---|---|
| First-run experience | `tests/e2e/first_run.rs` |
| Search asset → view chart → save to watchlist | `tests/e2e/asset_explore.rs` |
| Author strategy in DSL → backtest → view metrics | `tests/e2e/strategy_backtest.rs` |
| Run paper session → submit order → fill → reconcile | `tests/e2e/paper_session.rs` |
| Export research bundle → standalone verify | `tests/e2e/bundle_export_verify.rs` |
| Restore backup on new version | `tests/e2e/backup_restore.rs` |

---

## 8. Chaos Tests

### 8.1 In-CI Chaos

`tests/chaos/` — small chaos suite run on every CI:
- `provider_blackholed.rs` — CoinGecko unreachable; assert graceful degradation
- `frozen_feed.rs` — provider returns stale data; assert staleness markers surface
- `submission_timeout.rs` — broker doesn't respond; assert `Unknown` path quarantines
- `reconciliation_divergence.rs` — broker state diverges; assert reconciler heals or halts

### 8.2 Weekly Full Chaos

A more thorough chaos suite runs weekly:
- Network partitions between trusted core and sidecars.
- Disk full scenarios.
- Process kills mid-write.
- Clock skew injection.

---

## 9. Adversarial / Red-Team Suite

### 9.1 Prompt Injection (per v1.2 §4.4)

AgentDojo-style prompt injection tests against the AI plane. Test inputs:
- Malicious 8-K filing content (`tests/adversarial/filings/malicious_8k.txt`)
- Market commentary with embedded tool-injection (`tests/adversarial/news/`)
- News with prompt-injection payloads (`tests/adversarial/news/`)

The AI plane MUST:
- Not call risk-increasing tools based on injected content.
- Surface injection attempts as audit events.
- Maintain tool-class discipline (RiskReducing tools only directly; RiskIncreasing behind approval).

### 9.2 OWASP ASI01 (Goal Hijack)

Tests where injected content attempts to redirect the agent's goal. LlamaFirewall-pattern guards must catch these.

### 9.3 OWASP ASI02 (Tool Misuse/Excessive Agency)

Tests where injected content attempts to invoke tools outside their declared capability. The capability-class taxonomy (ReadOnly/RiskReducing/RiskIncreasing) MUST enforce.

### 9.4 Inspect AI Harness

UK AISI's Inspect framework as the CI red-team harness. Tracks pass/fail over time; regressions block release.

### 9.5 TradeTrap Faithfulness (per v1.2 §4.2)

Tests whether stated agent reasoning matches actual decision drivers (arXiv:2512.02261). If agent reasoning is surfaced as evidence, faithfulness is a correctness property.

---

## 10. Performance Tests

### 10.1 Benchmarks (`criterion`)

| Benchmark | Budget | Wave |
|---|---|---|
| Audit append latency | <1ms p99 | 0 |
| Cold start to interactive | <2.0s p95 mid-tier | 1 |
| Chart pan/zoom at 1M points | 16.7ms p99 frame | 1 |
| TSFM forecast single-asset | ≤500ms p95 | 3 |
| Analog search top-50 over 10M vectors | ≤120ms p95 | 5.5 |
| Manifest signing (Ed25519) | <10ms | 0 |
| Manifest signing (ML-DSA-65) | <500ms | 0 |

### 10.2 Regression Policy

Performance regressions are CI WARNINGS, not failures — except for audit append latency (which is on the sensitive path; regression is a failure).

---

## 11. Backward-Compatibility Tests

### 11.1 Old Manifest Verification

`tests/backward_compat/manifests/` — committed manifests from prior PRISMATIK versions. The current `prismatik-cli verify` MUST accept all of them.

### 11.2 Backup Migration

A backup taken on version N MUST restore on version N+1 (Wave 1 DoD criterion 9). Tested with backups from each prior minor version.

---

## 12. Coverage Targets

| Crate | Target coverage |
|---|---|
| `prismatik-determinism` | 95%+ |
| `prismatik-identity` | 95%+ |
| `prismatik-audit` | 95%+ |
| `prismatik-manifest` | 90%+ |
| `prismatik-risk` | 95%+ |
| `prismatik-execution` | 90%+ |
| All other crates | 80%+ |

Coverage is measured by `cargo tarpaulin` (or equivalent) and reported in CI. Coverage regressions are warnings; coverage below target for `prismatik-determinism`/`identity`/`audit`/`risk` blocks release.

---

## 13. Test Cadence

| Suite | Frequency |
|---|---|
| Unit, property, conformance, golden vector | Every commit |
| Integration | Every commit |
| DST (256+ seeds) | Every commit (Wave 3+) |
| E2E | Every PR to main |
| In-CI chaos | Every commit |
| Weekly full chaos | Weekly |
| Adversarial (regression suite) | Every commit |
| Full adversarial scan | Weekly |
| Performance benchmarks | Every PR; tracked over time |
| Backward compat | Every PR |

---

## 14. Test Data Management

### 14.1 Synthetic Data

Default. Synthetic data generators produce known distributions; expected outputs are computed from the generator.

### 14.2 Real Data Cassettes

For provider adapters. Re-recorded quarterly. No real credentials in cassettes; placeholders like `$TEST_COINGECKO_KEY` are substituted at test time.

### 14.3 Contaminated Data Awareness

When using real market data for tests, the test MUST record the data's `event_time` range and assert it doesn't overlap with model pretraining cutoffs (per Invariant I5). A test that uses post-cutoff data with a pretrained model is a contamination bug.

---

## 15. Cross-References

| Topic | Document |
|---|---|
| CI workflow definitions | `spec/CI_WORKFLOWS.md` |
| Risk register for test failures | Enterprise Overview Part VI |
| DST framework (madsim) | `spec/CRATE_ARCHITECTURE.md` §1.1 |
| Test fixtures (formats) | `spec/DATA_SCHEMAS.md` |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26 · Version 1.0*
