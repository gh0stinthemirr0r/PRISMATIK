# Wave 3 Workbook — Quantitative Platform

**Started:** 2026-07-26  
**Updated:** 2026-07-26  
**Gate status:** **OPEN** under [`TURBO_GATE_POLICY.md`](TURBO_GATE_POLICY.md) (P0-DK-10 Machine B **waived** for OPEN; residual before public signed binaries)

## Entry gate

Wave 3 requires:

1. Wave 2 exit gate — **green** per `DOCS/waves/Wave_2_Workbook.md`.
2. Complete **`P0-REMAINDER`** — **satisfied for OPEN** with Machine B waived per turbo policy.

`P0-REMAINDER` = every Wave 0 item marked `DEFER` under Option B in `DOCS/waves/Wave_0_Foundation.md`.

## P0-REMAINDER scorecard

| ID | Item | Status | Evidence |
|---|---|---|---|
| P0-DK-06 | Bitemporal symbology `resolve_as_of` | **done** | `crates/prismatik-identity` (`SymbologyResolver`, OpenFIGI + 51-event corpus); Wave 2A P2-DK-01 |
| P0-DK-07 | Merkle audit ledger + inclusion/consistency | **done** | `crates/prismatik-audit` — `InMemoryAuditLedger`, proofs, tamper test |
| P0-DK-08 | Audit startup verify + 1ms p99 criterion | **done** (harness) | `startup_verify` + `benches/audit_append.rs` + `append_p99_smoke_under_1ms`; report `DOCS/waves/P0_DK_08_Audit_Append_Benchmark.md` |
| P0-DK-09 | Manifest schema v1 + canonical + builder | **done** | `crates/prismatik-manifest` types/builder/canonical |
| P0-DK-10 | Dual signature Ed25519 + ML-DSA | **floor + waived** | Format + negatives + PQC exception; Machine B **waived for OPEN** — `TURBO_GATE_POLICY.md` (resolve before public signed binaries) |
| P0-DK-11 | `prismatik-cli verify` standalone | **done** (floor) | `crates/prismatik-cli` + `StandaloneVerifier` (no application dep) |
| P0-DK-12 | `ArtifactStore` content-addressed | **done** | `prismatik-determinism::artifact_store` (`MemoryArtifactStore`) |
| P0-DK-13 | Calendar artifact + `SessionCalendar` | **done** | `crates/prismatik-calendar` (Wave 2A P2-DK-04) |
| P0-DK-14 | QuantLib calendar generator zero-tolerance | **done** | `scripts/build_calendar_artifact.py` + `scripts/run-calendar-cross-validation.sh`; CI job `calendar-cross-validation`; report `crates/prismatik-calendar/artifacts/quantlib_cross_validation_report.json` |
| P0-SS-02 | Key hierarchy / OS keychain / secrecy | **done** | `crates/prismatik-security` — `keychain`, `keys`, `zeroize_secret` + unit tests (`MemoryKeychain` round-trip, HKDF derive, invalid root length) |
| P0-SS-04 | `cargo vet` trusted-core imports | **done** | `supply-chain/{config.toml,audits.toml,imports.lock}` + BA/Google/Mozilla imports; `cargo vet check --locked` green; CI `cargo-vet` + `scripts/cargo-vet-trusted-core.{sh,ps1}` |
| P0-SS-05 | OSS registry + license-class gate | **done** | `prismatik-oss-registry` license-class gate + `registry_coverage` test; CI job `registry-coverage` in `.github/workflows/ci.yml` |
| P0-SS-06 | cosign + SBOM + SLSA provenance | **floor** | `scripts/generate-sbom.{sh,ps1}`, `scripts/cosign-verify-example.md`, `artifacts/sbom/`, CI `sbom-generate` (continue-on-error) + `updater-provenance`; prod cosign/SLSA still author-ops — `DOCS/waves/P0_SS_06_Release_Signing.md` |
| P0-SS-07 | Updater provenance verification + negative test | **done** | `prismatik-security::updater` — `verify_or_refuse`; negatives: `refuses_invalid_artifact`, `refuses_missing_attestation`, `refuses_wrong_builder_identity`; CI `updater-provenance` |
| P0-QM-01 | `dst_replay_suite` skeleton (64 + 256 seeds) | **done** | `prismatik-application::dst_replay` — 64 default CI + `dst_replay_suite_256` |
| P0-QM-02 | Golden manifest corpus (2 manifests) | **done** | `crates/prismatik-manifest/golden/*.json` |

**Gate verdict:** **GREEN / OPEN (turbo).** Machine B waived for OPEN; production cosign/SLSA and live tokens remain author-ops per `TURBO_GATE_POLICY.md`.

## Wave 3A implementation (OPEN)

| ID | Status | Notes |
|---|---|---|
| P4-QM-01 | floor | StrategyIR + schema `1.0.0` |
| P4-QM-02 | floor | logos DSL lexer/parser + `to_ir_stub` |
| P4-QM-03 | floor | DSL type-check + capability inference |
| P4-QM-04 | floor | LogicalPlanStub Scan/Filter/Project (+ Join/AsOfFilter); no DataFusion link |
| P4-QM-05 | floor | `enforce_pit` inserts `AsOfFilter`, binds every Scan; neg: future join w/o as-of |
| P4-QM-06 | floor | `Strategy` / `StrategyContext` / `NullStrategy` |
| P4-QM-07 | floor | `BarBoundaryIter`, `BacktestEvent`, `run_stub` |
| P4-QM-08 | floor | fills + slippage/commission micros |
| P4-QM-09 | floor | metrics + purged/embargoed walk-forward |
| P4-QM-10 | floor | Indicator + SMA/EMA warmup NaN |
| P4-QM-11 | floor | **30** native indicators + goldens; external façade `try_adapt_external_indicator` fails closed |
| P4-QM-14 | floor | `conformance.rs` two-impl harness + `two_impl_conformance` test |
| P4-QM-12 | floor | BarSampler time/tick/**volume/dollar** |
| P4-QM-13 | floor | streaming≡batch property tests + IndicatorRegistry |
| P4-SS-01 | floor | Hardened wasmtime config assertion (no link) |
| P4-SS-02 | floor | CapabilitySet + host registry |
| P4-SS-03 | floor | PluginManifest install + sig/capability gates |
| P4-SS-04 | floor | CapabilityDiffGate + `cosign::try_verify_at_load` fails closed (`CosignError::NotLinked`; real cosign = author-ops residual) |
| P4-SS-05 | floor | `SyscallTracePolicy` / `CapabilityContainmentReport` + denied-network zero-Network assertion (ETW/strace residual) |
| P4-QM-15 | floor | Python StrategyIR emitter + schema under `services/research-sidecar/strategy_ir/` |
| P4-EX-03 | floor | `prismatik-strategy-sdk` `StrategyIrBuilder` |
| P4-EX-01 | **floor scaffold** | `workspace/strategy` SMA/EMA/RSI blocks + DSL preview + `get_strategy_ir_preview` |
| P4-EX-02 | **floor scaffold** | `workspace/backtest` + signed manifest summary |
| DoD #16 | floor | `prismatik-backtest::manifest` signed stub e2e |
| DoD #10 | floor | three-way IR structural parity (hand / DSL / Python golden) |

## Wave 3B / 3C floors (turbo)

| ID | Status | Notes |
|---|---|---|
| P5-QM-01..04 | floor | `prismatik-simulation` seeded MC + `split_seed` + **Rayon** `run_paths_parallel` (1 vs 4 thread identity) |
| P5-EX-01 | **floor scaffold** | `workspace/simulation` histogram + path table + `get_monte_carlo_paths` (wgpu deferred) |
| P5-EX-02 | **floor scaffold** | `workspace/calibration` ribbon + nominal vs realized coverage + `get_calibration_ribbon` |
| P5-EX-03 | **floor scaffold** | `workspace/models` model card + drift + blind spots + `get_model_card` |
| P5-QM-05..10 | floor | `prismatik-tsfm` registry / contamination / cache |
| P5-QM-11..15 | floor | `prismatik-calibration` I2 + ladder + drift Widen + **P5-QM-12** sidecar façade |
| P55-EX-01 | **floor scaffold** | `workspace/analogs` hits + mandatory disclosures + `get_analog_hits` |
| P55-QM-01 | floor | `materialize_embeddings` + `PrismatikSeriesTokenizer` (stub runtime empty vectors) |
| P55-QM-02..04 | floor | analog disclosures + LeaveNOut + filters; LanceDB `try_open_ivf_pq` fails closed (`BackendNotLinked`) |

Turbo continues filling remaining P4-* (DSL typecheck, external indicator adapter, DataFusion PIT) / EX / Waves 4–7 UI & ops in parallel.

## Residual before public signed release (not blocking turbo OPEN)

1. Fill **Machine B** in [`P0_DK_10_Cross_Machine_Verify.md`](P0_DK_10_Cross_Machine_Verify.md) (dual-sig format + negatives + host identity; **WAIVED** for turbo OPEN — required before public signed binaries)
2. Complete SLSA L3 / cosign author-ops checklist in [`P0_SS_06_Release_Signing.md`](P0_SS_06_Release_Signing.md) (beyond SS-06 floor; scripts + CI `updater-provenance` / `sbom-generate`)
3. Live tokens / soak / hand labels — author-ops (`TURBO_GATE_POLICY.md`)

~~P0-SS-02~~ **done** · ~~P0-SS-04~~ **done** · ~~P0-SS-05~~ **done** · ~~P0-SS-06 floor~~ **done** · ~~P0-SS-07~~ **done** · ~~P0-DK-08~~ **done** · ~~P0-DK-14~~ **done**

## Verification (2026-07-26)

```text
Turbo OPEN: see DOCS/waves/TURBO_GATE_POLICY.md
P0-SS-04 cargo vet check --locked GREEN; trusted-core GREEN
DK-10 Machine B waived for OPEN (resolve before public signed binaries)
```
