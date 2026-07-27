# Orphaned Module Remediation

**Status:** OPEN — 43 modules across 15 crates
**Created:** 2026-07-27
**Branch:** `feat/wave0-floor-continue`
**Prerequisite commit:** `a3540bc` (21 modules already wired)

---

## 1. The Problem

A large body of committed code exists as `.rs` files under `crates/*/src/` that are **never declared
with `mod`** in their crate's `lib.rs`. Rust does not compile a file that is not reachable from the
crate root, so this code is dead: never compiled, never linted, never tested, invisible to CI.

This is why the workspace gates looked green while the code was broken — the compiler never saw it.

The root cause splits cleanly in two:

| Cause | Fix shape | Already done? |
|---|---|---|
| **A — missing dependency line.** Module is complete and correct; the crate's `Cargo.toml` just never gained the `serde` / `thiserror` / `serde_json` entry it needs. | Add dep + `pub mod`, done. | ✅ 21 modules, commit `a3540bc` |
| **B — missing base API.** Module was written against a *later, fuller* version of its crate's own types than what was landed. Fields, enum variants, methods, and helper types it references do not exist. | Extend the base types to the shape the module expects. | ❌ 43 modules, this document |

**Every item below is cause B.** Adding `pub mod` alone will not fix any of them.

### Verify the current state

```bash
cargo test --workspace --lib
```

Expected today: **106 passing, 0 failing.** Library targets and the clippy gate are clean. Three
non-lib targets still fail (`prismatik-audit` bench `audit_append`, `prismatik-identity` test
`identity_corpus`, `prismatik-manifest` example `gen_goldens`) — they reference orphaned code and
will resolve as part of Tier 3 below.

### Find orphans at any time

```bash
for c in crates/*/; do lib="$c/src/lib.rs"; [ -f "$lib" ] || continue; for f in "$c"src/*.rs; do b=$(basename "$f" .rs); case "$b" in lib|main) continue;; esac; grep -q "mod $b\b" "$lib" || echo "$(basename $c) :: $b"; done; done
```

---

## 2. Working Method

For each crate, in the tier order given:

1. Add the dependency lines listed in the crate's section to `crates/<name>/Cargo.toml`.
2. Add `pub mod <name>;` to `crates/<name>/src/lib.rs` for each orphan.
3. `cargo check -p <name> --lib` and work the errors by **extending the base type**, never by
   editing the orphaned module to match the thinner type. The orphaned module is the specification;
   the landed type is the thing that is behind.
4. Add `pub use` re-exports for anything the crate's own `tests/`, `benches/`, or `examples/`
   reference.
5. Gate before moving on:
   ```bash
   cargo fmt --all && cargo clippy -p <name> --all-targets -- -D warnings && cargo test -p <name>
   ```

**Do not** delete an orphaned module to make the build pass. If a module turns out to be genuinely
obsolete, say so explicitly and get a decision — silent deletion loses real work.

**Do not** relax `#![warn(missing_docs, missing_debug_implementations)]` or `#![forbid(unsafe_code)]`
in any crate. CI runs `clippy -D warnings`; newly-wired public items need doc comments.

### Dependency order

Some crates depend on others' fixes. Respect this order:

```
determinism ──┬─→ identity ──→ (identity_corpus test)
              └─→ manifest ──→ audit ──→ backtest
market-data ──┬
storage ──────┴─→ application     (do application LAST)
```

---

## 3. Tier 1 — Blocked only on a decision (2 crates, 3 modules)

### `prismatik-storage` — `object_store`, `parquet_raw`

Needs three crates that are **not in `[workspace.dependencies]`**: `arrow`, `parquet`, `hex`.
`hex` is already there; `arrow` and `parquet` are new third-party dependencies.

**This is a supply-chain decision, not a code decision.** The repo has `deny.toml`,
`supply-chain/audits.toml`, and a cargo-vet gate. Adding two large new dependency trees needs
whoever owns that policy to sign off and to run the vet/audit flow. Do not add them unilaterally.

Once approved: add `arrow` and `parquet` to `[workspace.dependencies]`, then
`arrow.workspace = true`, `parquet.workspace = true`, `hex.workspace = true` to the crate.

### `prismatik-cli` — `public_verify`

Deps to add: `prismatik-determinism`, `serde`, `serde_json`.

Then:
- Export `CliError` from the crate root (`pub use` from wherever it is defined).
- `ManifestV1` needs an `audit` field — comes free with the `prismatik-manifest` work in Tier 3.
- One `E0782` (bare trait used as a type) — add `dyn`.

`main.rs` is a binary root, not a module. Do **not** add `pub mod main;`.

---

## 4. Tier 2 — Small, self-contained (5 crates, 11 modules)

Each is a handful of derives, enum variants, or struct fields. Good parallel work; no cross-crate
coupling.

### `prismatik-journal` — `learning`
Dep: `serde`.
- `OutcomeTag`: add `#[derive(Serialize, Deserialize)]`.
- `OutcomeTag`: add variants `Incomplete`, `Scratch`.

### `prismatik-oss-registry` — `marketplace`, `publish`
Deps: `serde`, `thiserror`.
- `LicenseClass`: add `#[derive(Serialize, Deserialize)]`.
- Export from crate root: `ComponentRecord`, `RegistryError`, `commercial_license_gate`.

### `prismatik-analog-store` — `lancedb`
Dep: `thiserror`.
- Add crate-root const `EMBEDDING_DIMENSIONS`.
- One `E0782` (bare trait used as a type) — add `dyn`.

### `prismatik-indicator-core` — `conformance`, `external_indicator`, `registry`
No new deps.
- `IndicatorDescriptor`: add fields `kind`, `provenance`, `warmup_behavior`, `warmup_len`.
- `WarmupBehavior`: add variant `Nan`.

Note the architecture requires `IndicatorDescriptor` to carry provenance (§18.4) — this field is
load-bearing for the golden-vector conformance story, not cosmetic.

### `prismatik-calibration` — `forecast`, `ladder`, `sidecar`
Deps: `serde`, `thiserror`.
- `CalibrationRecord`: add `#[derive(Serialize, Deserialize)]`.
- `CalibrationMethod`: add variants `AdaptiveConformal`, `MondrianConformal`, `EnbPI`.
- `trait_def`: add `CategorizerId`, `PredictionBatch`, `RealizationBatch`, `NullCalibrator`.

These variant names are the ones the architecture names normatively (§17.3: ACI as the time-series
default, Mondrian for regime-conditioned calibration, EnbPI). Match the spec spelling.

---

## 5. Tier 3 — Substantive API build-out (8 crates, 29 modules)

Do these in the dependency order given in §2.

### 5.1 `prismatik-identity` — `corpus`, `factory`, `openfigi`
No new deps. **Do this first** — it unblocks the `identity_corpus` test target.

| Missing | Where |
|---|---|
| `InMemoryResolver` | `resolver.rs` — in-memory `SymbologyResolver` impl with `insert`, `insert_valid`, `insert_transition`, `snapshot_artifact` |
| `ValidityInterval` | `resolver.rs` — `new(valid_from, valid_to) -> Result<Self, SymbologyError>`; bitemporal validity window |
| `IdentityTransition` fields | `asset_id`, `effective_at`, `new_identifiers`, `dropped_identifiers`, `memo` |
| `AssetId::from_canonical_bytes` | `asset_id.rs` |
| `AssetId::from_hash` | `asset_id.rs` |
| `MicCode::from_str_unchecked` | `asset_id.rs` |

**Conflict to resolve:** `lib.rs` currently defines an inline stub `pub struct OpenFigiMapper` with
`normalize_figi` / `to_external_id` plus a `mapper_tests` module. `openfigi.rs` defines the real one.
Delete the stub, move `normalize_figi` and `to_external_id` onto the real type as associated
functions (they are stateless), and keep the two existing tests passing.

Re-export for the test target: `IdentityCorpus`, `IdentityEvent`, `IdentityEventType`,
`assert_event_as_of`, `FigiMapping`, `OpenFigiMapper`.

This is the crate where correctness matters most — bitemporal symbology is Wave 2 DoD #1 and the
architecture is explicit that identifiers are attributes with validity intervals, never keys.

### 5.2 `prismatik-manifest` — `canonical`, `golden`, `types`
Deps: `blake3`, `hex`. Unblocks the `gen_goldens` example.

- `verify.rs`: add `sign_manifest_ed25519`.
- Crate root exports: `AuditBlock`, `DeterminismBlock`, `EntropyStreamSeed`, `ManifestKind`,
  `ProducerInfo`, `ProducerProfile`, `build_golden_pair`.
- `ManifestV1`: add `audit` field (also unblocks `prismatik-cli`).
- `ManifestBuilder`: add `pinned` method.

**Requires in `prismatik-determinism` first:** `SigningIdentityKind` and `signing_key_from_seed`
(a `SigningIdentity` exists; these do not).

Per §3 of the architecture, manifests are **dual-signed (Ed25519 + ML-DSA) from Phase 0**. If the
signing helpers are being written now, write them against the dual-signature shape — retrofitting a
signature scheme onto an existing manifest corpus is explicitly called out as far more expensive.

### 5.3 `prismatik-audit` — `entry`, `ledger`, `write_path`
Dep: `blake3`. Also needs `criterion` + `tokio` as **dev-deps** for the `audit_append` bench.

| Missing | Where |
|---|---|
| `hash_leaf`, `hash_children`, `verify_inclusion` | `proof.rs` |
| `InclusionProof` fields | `leaf_hash`, `audit_path`, `head` |
| `ConsistencyProof` field | `nodes` |
| `TreeHead::empty()` | `proof.rs` |
| `AuditError` variants | `PrevHashMismatch`, `PositionOutOfRange`, `InconsistentSizes`, `Integrity` |
| `InMemoryAuditLedger` | crate root export |
| `Actor::System` | needs a `component` field |
| `SubjectRef::None` | add variant |
| `AuditAction::Operational` | add variant |

This implements Invariant I7 and gap G02. `test_audit_chain_integrity` is specified to verify Merkle
inclusion proofs on every CI run *and every application start* — build the proof helpers to be cheap
enough for that.

### 5.4 `prismatik-backtest` — `manifest`, `walk_forward`
Deps: `blake3`, `hex`, `prismatik-determinism`, `prismatik-manifest`, `serde_json`, `time`.
**Do after 5.2.**

- Crate root: add `BacktestWindow`.
- `BacktestConfig`: add fields `strategy_id`, `window`, `execution`, `bar_interval_seconds`.
- `BacktestResult`: add fields `status`, `bars_processed`.
- `BacktestError`: add variant `InvalidWalkForward`.

Walk-forward must be **purged + embargoed** (drop training samples whose label horizon overlaps the
test window; drop a buffer after it). Without both, overlapping labels leak across the split and
every out-of-sample number is optimistic. This is a correctness requirement, not a refinement.

### 5.5 `prismatik-execution` — `emergency`, `error`, `fills`, `order`
Deps: `serde`, `thiserror`.

- `IdempotencyKey`: add `#[derive(Serialize, Deserialize)]`.
- **Pre-trade check ids are currently `&'static str`.** The orphaned modules use them as enum
  variants: `MaxAccountLoss`, `AccountPermissions`, `MarketStatus`, `DuplicateOrder`,
  `AdjustedContract`, `CalendarMismatch`, `CorrelationCluster`, `EventProximity`, `LiquidityVolume`,
  and the rest of the §19.1 catalog. Introduce a `PreTradeCheckId` enum covering all sixteen checks.
- **In `prismatik-risk`:** add `RiskEvalInput` and `RiskLimits`, exported from the crate root.

The enum is the right call and worth the churn: the architecture requires checks to evaluate in a
fixed declared order with all HardDeny before any SoftWarn, and an enum makes that ordering
expressible and testable instead of stringly-typed.

### 5.6 `prismatik-tsfm` — `cache`, `embed`, `families`, `finetune`, `onnx`
Deps: `serde`, `thiserror`.

- `TsfmRuntime` trait: add methods `embed` and `model_id` (currently the orphans try to `impl` them
  as non-members — `E0407`).
- `registry`: add `SeriesModality`.
- `runtime`: add `TsfmRuntimeError`.
- `TokenizerError`: implement `Display` (needed for `thiserror` `#[source]` chaining).
- One `E0609` (`.0` on a `String`) — newtype mismatch, inspect at the call site.

### 5.7 `prismatik-options` — `hand_labels`, `store`
Deps: `prismatik-identity`, `serde`, `serde_json`, `thiserror`, `time`. **Do after 5.1.**

- `OptionContract`: add fields `root`, `underlying`, `right`, `strike_millis`; add
  `#[derive(Serialize, Deserialize)]`.
- Crate root: add `OptionRight`, `OptionSide`/`FlowSide`, `OptionsFlowPrint`, `classify_flow`.
- `FlowClassification`: add `#[derive(Serialize, Deserialize)]`.
- Needs `AssetId::from_canonical_bytes` from 5.1.

Flow classification must expose confidence and retain an `Unclassified` outcome that is used freely
— a classifier that always decides is a classifier that is often wrong. Preserve that if the
orphaned module already has it; add it if not.

### 5.8 `prismatik-application` — 8 modules (`alerts`, `backup`, `dst_replay`, `equity_live`, `first_run`, `http_live`, `runtime`, `soak`)
Deps: `async-trait`, `blake3`, `hex`, `reqwest`, `serde`, `serde_json`, `thiserror`, `time`.
**Do this LAST** — it consumes APIs from `market-data` and `storage`.

Missing exports it needs from other crates:

| From | Symbols |
|---|---|
| `prismatik-market-data` | `RawObservation`, `RawStore`, `RawStoreError`, `NormalizePipeline`, `NormalizedBatch`, `IngestError`, `GcraBudgetGovernor`, `HttpTransport`, `HttpRequest`, `HttpResponse`, `HttpMethod`, `TransportError`, `adapters` module |
| `prismatik-storage` | `PreferenceStore`, `AnalyticalBackend`, `ParquetRawWriter`, `RawRecord`, `RawAppendError` |

Several of these live in the modules wired in `a3540bc` (`market-data::http`, `::cassette`,
`::chain_exec`) and may only need `pub use` re-exports. The storage ones are gated on Tier 1.

`prismatik-application` is the only crate permitted to depend on `prismatik-storage` — preserve that
rule. Do not let any domain crate reach for a storage backend.

---

## 6. Definition of Done

| # | Criterion | Command |
|---|---|---|
| 1 | Zero orphaned modules | the `for` loop in §1 prints nothing |
| 2 | All targets compile | `cargo check --workspace --all-targets` |
| 3 | All tests pass | `cargo test --workspace` |
| 4 | Lint gate clean | `cargo clippy --workspace --all-targets -- -D warnings` |
| 5 | Format gate clean | `cargo fmt --all -- --check` |
| 6 | Release builds | `cargo build --workspace --release` |
| 7 | No module deleted to pass | review the diff |
| 8 | No lint attribute relaxed | `git diff` shows no changes to `#![warn(...)]` / `#![forbid(...)]` |

## 7. Suggested Commit Slicing

One commit per crate, message naming the modules wired and the base APIs extended. Do not squash the
whole remediation into one commit — the base-type changes are the interesting part of the diff and
they deserve to be reviewable per crate.

## 8. Guard Against Recurrence

Once the count reaches zero, add the §1 detector to CI as a gate. An orphaned module is silent by
construction — it produces no warning, no error, and no test failure, which is exactly the class of
defect this platform's architecture is otherwise built to make impossible.

```bash
# fails if any src/*.rs is not reachable from its crate root
test -z "$(for c in crates/*/; do lib="$c/src/lib.rs"; [ -f "$lib" ] || continue; for f in "$c"src/*.rs; do b=$(basename "$f" .rs); case "$b" in lib|main) continue;; esac; grep -q "mod $b\b" "$lib" || echo "$b"; done; done)"
```
