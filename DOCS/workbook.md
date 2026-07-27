# Project workbook — Turbo build status

**Updated:** 2026-07-26  
**Policy:** [`DOCS/waves/TURBO_GATE_POLICY.md`](waves/TURBO_GATE_POLICY.md)  
**Gate reviews:** [`DOCS/gates/`](gates/) (wave-0 … wave-7)

## Verdict

| Scope | Status |
|---|---|
| Wave 0–2 code floors | **green** (author-ops residuals waived per turbo) |
| Wave 3–7 turbo floors | **green (turbo OPEN)** — structural types + tests; EX scaffolds landing |
| Production release | **not claimed** — Machine B, prod cosign/SLSA, live tokens, soak |

## Per-wave trackers

| Wave | Workbook | Gate |
|---|---|---|
| 0 Foundation | [`Wave_0_Foundation.md`](waves/Wave_0_Foundation.md) | [`gates/wave-0.md`](gates/wave-0.md) |
| 1 MVP Crypto | [`Wave_1_Workbook.md`](waves/Wave_1_Workbook.md) | [`gates/wave-1.md`](gates/wave-1.md) |
| 2 Multi-Asset | [`Wave_2_Workbook.md`](waves/Wave_2_Workbook.md) | [`gates/wave-2.md`](gates/wave-2.md) |
| 3 Quant Platform | [`Wave_3_Workbook.md`](waves/Wave_3_Workbook.md) | [`gates/wave-3.md`](gates/wave-3.md) |
| 4 Research Loop | [`Wave_4_Workbook.md`](waves/Wave_4_Workbook.md) | [`gates/wave-4.md`](gates/wave-4.md) |
| 5 Execution | [`Wave_5_Workbook.md`](waves/Wave_5_Workbook.md) | [`gates/wave-5.md`](gates/wave-5.md) |
| 6 Enterprise | [`Wave_6_Workbook.md`](waves/Wave_6_Workbook.md) | [`gates/wave-6.md`](gates/wave-6.md) |
| 7 Ecosystem | [`Wave_7_Workbook.md`](waves/Wave_7_Workbook.md) | [`gates/wave-7.md`](gates/wave-7.md) |

## Verification commands

```bash
cargo test --workspace
cargo vet check --locked
python services/research-sidecar/strategy_ir/test_strategy_ir.py -v
pnpm --filter @prismatik/desktop check
```

## Batch 6 complete (2026-07-26)

- Desktop EX: `workspace/backtest`, `portfolio`, `marketplace` + `research_fixtures.rs` IPC
- Enterprise: `PostgresProfile`, `RowLevelSecurityContext`, `JetStreamEnvelope`
- P9-QM-01: `PublicVerifyService` + `PublicTreeHead` in `prismatik-cli`
- DoD #16: signed backtest manifest e2e (`prismatik-backtest::manifest`)
- P4-QM-11: +10 indicators (WMA, StdDev, ATR, MACD, Stoch %K, OBV, VWAP, Min, Max, Range)
- P4-QM-14: conformance harness (`conformance.rs` + `two_impl_conformance` test)
- P4-SS-05: `SyscallTracePolicy` / denied-network containment assertion

## Batch 8 complete (2026-07-26)

- P4-QM-11: **30** native indicators + external-indicator adapter façade
- P5-QM-04: Rayon `run_paths_parallel` + cargo-vet deploy exemptions
- P5-QM-08: `OnnxTsfmRuntime` façade (fails closed)
- P4-QM-09: deflated Sharpe haircut floor
- P5-EX-02/03 + P55-EX-01: calibration / models / analogs routes
- Terraform stub under `deploy/terraform/`
- EX wire: backtest / portfolio / marketplace invoke IPC
- P5-QM-12: calibration sidecar conformal Arrow IPC façade

## Batch 9 complete (2026-07-26)

- OIDC/Vault typed floors (`prismatik-security`)
- DataFusion PIT deepen + Lance IVF-PQ façade
- Plugin cosign load-time floor + soak bench types

## Batch 10 complete (2026-07-26)

- OpenLineage emit sinks · flow HandLabelCorpus · series-tokenizer embed batch
- Journal EX + MkDocs scaffold
- OSS `PublishChecklist` + NOTICE
- Machine B / SLSA author-ops templates (not claimed done)
- Plugin cosign + soak plan floors
- AI router `deny_uncalibrated` + tools `ToolInvocationGate` Network deny-by-default

## Batch 11 complete (2026-07-26)

- P4-QM-09: annualized Sharpe (`mean excess / σ · √bars_per_year`) + DSR haircut
- P5-QM-01: seeded SV (log-σ AR(1)) + Markov regime path generators
- P5-QM-12: `SidecarTransport::Loopback` in-process calibrate; Remote stays fail-closed
- P6-QM-02: Pearson correlation / sample covariance matrices (no placeholder)
- P4-QM-02/03: DSL builtins `sma`/`ema`/`rsi` → IR indicator refs
- EX: `get_strategy_ir_preview` calls `prismatik-strategy::parse_typecheck_to_ir_stub`

## Batch 12 complete (2026-07-26)

- Simulation: `run_path_series` (cumulative paths; terminals match `run_paths`)
- EX: `get_monte_carlo_paths` → `prismatik-simulation` SV series
- EX: `get_calibration_ribbon` → `CalibrationSidecarClient::loopback`
- P6-QM-02: `CorrelationCluster` fails when max |ρ| exceeds limit
- P8-SS: session `is_expired` negatives + RBAC deny-matrix tests

## Batch 13 complete (2026-07-26)

- EX backtest: `compute_metrics` + walk-forward folds + signed stub manifest; `annualizedSharpe` (no placeholder field)
- EX analogs: `prismatik-analog-store::search` with mandatory disclosure strings
- Analog store: `AnalogResult` / hit constructors refuse omitted disclosures; Lance IVF-PQ stays fail-closed
- OpenLineage: in-memory sink success + HTTP fail-closed (`prismatik-events`)

## Batch 14 complete (2026-07-26)

- TSFM: keep plural-registry / license-deny / tokenizer logic; drop research-repo names → `OnboardedTsfmFamily::{FinancialBar,…,RestrictedLicense}`
- ADR-0032 amended: research GitHub names are not integrations; living scorecards use role vocabulary

## Batch 15 complete (2026-07-26)

- EX portfolio: `prismatik-portfolio` micros PnL + cost-basis provenance
- EX analogs: adapted to `AnalogResult` neighbours + shared disclosures
- DSL: `and` / `or` with comparison-tighter precedence; multi-indicator IR extraction
- PIT: deeper `enforce_pit` nesting / join / AsOfFilter collapse negatives

## Batch 16 complete (2026-07-26)

- EX marketplace: `prismatik-oss-registry::MarketplaceListing` + `assert_installable` gate rows
- EX orders: `draft_paper_order_ticket` + MaxPosition risk gate + deterministic idempotency
- EX journal: `turbo_fixture_summary` (10 labels / all classes, author target 20)
- Options: paired `turbo_fixture_prints` classifier goldens + corpus summary types
- Execution: typed `OrderIntent` / `PaperOrderTicket` floor API

## Batch 17 complete (2026-07-26)

- EX model card: `OnboardedTsfmFamily` + drift suite action + ladder `can_promote`
- EX calibration: multi-backend loopback + coverage-gap drift assessment
- EX simulation: `summarize_terminals` (mean / drawdown proxy / ruin)
- EX plugin host: `HardenedEngineConfig` status + `install_plugin` preview wired to marketplace

## Batch 18 complete (2026-07-26)

- EX strategy IR: canonical `ir_digest` + capabilities + rule block from DSL AST
- EX backtest: unsigned manifest `blake3` digest + OpenLineage START/COMPLETE recording
- EX auth: `SessionPolicy` + RBAC deny-matrix probe (`get_workspace_auth_floor`)

## Batch 19 complete (2026-07-26) — naming remediation + determinism gate made true

**Naming hygiene:** renamed every source artifact whose filename carried a delivery-wave
label to a semantic, content-based name (filenames should describe what the file *contains*,
not the phase that produced it):

| Old | New |
|---|---|
| `apps/desktop/src-tauri/src/wave2.rs` | `equity_fixtures.rs` |
| `apps/desktop/src-tauri/src/wave3_ex.rs` | `research_fixtures.rs` |
| `apps/desktop/src-tauri/permissions/wave2.toml` | `equity_fixtures.toml` |
| `crates/prismatik-strategy/src/wave3.rs` | `strategy_ir.rs` |
| `crates/prismatik-filings/tests/wave2a_parsers.rs` | `parser_conformance.rs` |
| `crates/prismatik-market-data/tests/wave2a_adapters.rs` | `adapter_conformance.rs` |
| `crates/prismatik-storage/migrations/M0003_wave1_ops.sql` | `M0003_workspace_ops.sql` |
| `DOCS/user/Wave_1_User_Guide.md` | `DOCS/user/MVP_User_Guide.md` |
| `ProviderChain::wave1_crypto_defaults` | `ProviderChain::crypto_default_chains` |
| `ProviderChain::wave2_equity_defaults` | `ProviderChain::equity_default_chains` |

All module declarations, permission scopes, migration version keys, and doc cross-references
updated. Strategic delivery-plan docs (`DOCS/waves/Wave_*.md`, `DOCS/gates/wave-N.md`) kept
their vocabulary — "Wave" is the project's term for its delivery phases there.

**Determinism gate (Wave 0 DoD #1) — was documented "met", was not actually met.**
`cargo clippy --workspace --all-targets -- -D warnings` was red on the working tree:
~30 disallowed-method/type violations (ambient `OffsetDateTime::now_utc`, `Uuid::new_v4`,
`HashMap`) across `prismatik-storage`, `prismatik-features`, `prismatik-options`,
`prismatik-portfolio`, `prismatik-tsfm`, `prismatik-execution`, `prismatik-cli`, plus a
broken `clippy.toml` path (`rand::thread_rng` does not exist in rand 0.10). Fixed for real:

- `SqliteBackend` now holds an injected `Arc<dyn Clock>`; all operational timestamps route
  through `clock.now()` and operational ids derive from clock state (no ambient RNG). Added
  `open_with_clock` / `open_in_memory_with_clock` for deterministic test/replay.
- `apply_migrations` takes the `applied_at_unix` timestamp as a caller parameter.
- `prismatik-features` / `prismatik-options` / `prismatik-portfolio` / `prismatik-tsfm` /
  `prismatik-execution` switched `HashMap` → `DetMap` (FxHasher, stable iteration order).
- `prismatik-cli::PublicVerifyService` uses `SystemClock` (legitimate outermost-shell wall clock).
- `clippy.toml`: removed the nonexistent `rand::thread_rng` / `rand::rngs::StdRng::new` paths.

**Full clippy `-D warnings` gate now green**, including `--all-targets` (test code):
resolved `result_large_err` (boxed plugin-host error payloads), `too_many_arguments`
(`EvidenceRecord` params struct for `put_evidence`; scoped allows on MACD/fixture helpers),
`needless_range_loop`, `manual_contains`, `should_implement_trait` (real `FromIterator` impls),
`neg_cmp_op_on_partial_ord`, `cast_abs_to_unsigned`, `uninlined_format_args`,
`field_reassign_with_default`, `unnecessary_literal_unwrap`, and the `SystemTime::now` in the
options JSONL test (switched to `tempfile::TempDir`).

**Verification (CI-matching):** `cargo fmt --all -- --check` → 0 diffs.
`cargo clippy --workspace --all-targets -- -D warnings` → clean.
`cargo test --workspace` → **499 passed, 0 failed** across 81 groups.

## Still turbo-waived (author-ops, not blocking)

- Machine B cross-verify · prod cosign/SLSA · live tokens · soak hardware · OIDC/Vault live · native wgpu · Apache crates.io publish
