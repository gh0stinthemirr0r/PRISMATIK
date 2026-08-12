# Production Ingestion Spine — Build Plan

## Goal (per owner direction)
Stop shipping mocks/fixtures/scaffolding. Build the **real backend data path** end-to-end so live provider data flows into the desktop workspace with durable storage, provenance, and audit. Sequence: ingestion spine first (the load-bearing production foundation); later passes layer analytics/Part III on top.

## Where we're starting from (verified, not claimed)
- Dev server, Svelte check, default Tauri build, and 10 `corpus.rs` tests all green.
- `cargo test --workspace` is **RED** on 3 targets — all the **same root cause**: stale `lib.rs` files that don't declare real implementation files already on disk.
- There is substantially more real code than the crate roots expose. The dominant disease is "uncompiled-but-real," not "missing."

## Decisions locked in
- **Durable store:** content-addressed files (BLAKE3 blobs) + append-only Merkle JSONL for the observation log and audit ledger. **Zero new dependency tree** (respects briefing §4/§21/§54 fences). Built behind the existing `Repository` port so arrow/parquet can land later as a backend swap when §37's analytics plane is actually built. This is the best path because the corpus foundation (Part IV §47/§55) is ordered/append-only/hash-chained work — exactly what a Merkle log + content store is for — and the architecture's columnar stack is for the *analytical* plane, not this layer.
- **Rate limiter:** `governor` crate approved. Implement `GcraBudgetGovernor` behind the existing `BudgetGovernor` trait (briefing §16).
- **Scope:** ingestion spine first.

---

## Phase A — Wire the uncompiled real backend (low risk, high leverage)
*Turn dead code into compiled, tested code. Fixes the 3 failing workspace tests. No new deps.*

**A1. prismatik-audit reconciliation (dual-phase, not pure wiring)**
- `ledger.rs` re-declares `AuditLedger` trait + `VerificationReport`, which also live in `trait_def.rs`/`proof.rs`. Reconcile: keep the richer `ledger.rs`/`entry.rs` versions, update `trait_def.rs` to re-export from them (or fold together), and update `lib.rs` to declare `entry`, `ledger`, `write_path`.
- Ensure `proof.rs` exports the helpers `ledger.rs` needs: `hash_leaf`, `hash_children`, `verify_inclusion`, `TreeHead::empty`.
- Add `[dev-dependencies]` (tokio, criterion) + `[[bench]]` section to `Cargo.toml`; fix `benches/audit_append.rs` to use the real API. This makes the failing `audit_append` target pass.
- Gate: `cargo test -p prismatik-audit` (incl. the 1ms p99 latency test) green.

**A2. prismatik-manifest**
- Wire `types.rs`, `canonical.rs`, `golden.rs` into `lib.rs`. Implement the missing signing primitives the golden path needs: `sign_manifest_ed25519`, `signing_key_from_seed` (in determinism or manifest), and `StandaloneVerifier::transitional()`/`verify_json()`. Fix `examples/gen_goldens.rs`. Golden fixture files already exist.
- Gate: `cargo test -p prismatik-manifest` green including `gen_goldens`.

**A3. prismatik-identity**
- Wire `corpus.rs`, `openfigi.rs`, `factory.rs`. Implement the missing resolver pieces `openfigi.rs` needs (`InMemoryResolver`, `ValidityInterval`, `AssetId::from_canonical_bytes`, `MicCode::new`/`from_str_unchecked`) — **extend the existing types per ORPHANED_MODULE_REMEDIATION.md, never delete** (briefing §1 rule). Fix `tests/identity_corpus.rs`.
- Gate: `cargo test -p prismatik-identity` green.

**A4. prismatik-market-data: export adapters**
- Add `pub mod adapters;` to `lib.rs` so the 6 real provider adapters (CoinGecko, Alpaca, FRED, SEC EDGAR, CFTC, Finnhub) become reachable. Verify each compiles against current types; fix field drift minimally (do not relax `#![warn]` lints — write any missing docs).
- Gate: `cargo test -p prismatik-market-data --lib` still green.

**Phase A exit gate:** `cargo test --workspace` goes from RED (3 failing) to fully GREEN. `cargo clippy --workspace --lib -- -D warnings` clean.

---

## Phase B — Corpus Day 2: the live ingest spine (briefing §52 Day 2)
*Stitch adapter fetch → ObservationDraft → seal → durable persist → audit append, under GCRA.*

**B1. Add `governor` to workspace deps + implement `GcraBudgetGovernor`**
- Add `governor` to root `Cargo.toml` `[workspace.dependencies]` (small, pure-Rust; run cargo-deny/cargo-vet per §54 item 8).
- Implement `GcraBudgetGovernor` in `market-data/src/governor.rs` behind the existing `BudgetGovernor` trait, keyed per `PriorityClass` + `Entitlement`. Property test: admits up to quota then `Defer`s.

**B2. Durable observation store (no new dep tree)**
- In `prismatik-storage`, implement a real backend behind the `Repository` port: a `FileObservationStore` writing (a) BLAKE3 content-addressed blobs for raw payloads where policy permits (`PayloadRetention`), and (b) an append-only Merkle JSONL ledger of sealed `Observation` records (one line per observation, hash-chained via `prev_hash`, mirroring the audit ledger pattern). Idempotent on duplicate `ObservationId`. Fsync + atomic rename on each append.
- This satisfies Part IV §47.1/§47.2 and §55 items 1–5 (versioned policy, observation_time, supersession chains, metadata-only visibility, raw-body gating).

**B3. Audit bridge from corpus → audit ledger**
- Add `prismatik-audit` dependency to `prismatik-market-data` (or keep the bridge in application — prefer application to keep market-data Layer-2 clean). After `Observation::seal()` / `ObservationLog::append()`, construct an `AuditEntry` (`AuditAction::Operational { code: "observation_sealed" }`, actor `System{component: "ingest"}`, subject the `ObservationId`, outcome `Allowed`/`Denied`) and call `AuditLedger::append`. Emit retrieval-attempt, success, rejection, and supersession events (briefing §52 Day 2).
- Durable audit ledger: same Merkle JSONL pattern as B2 (reuse the writer), so the audit chain survives restart and is externally verifiable via `prismatik-cli verify` on a published tree head (briefing §14.4, invariant I7).

**B4. Live HTTP transport in application**
- Wire `http_live.rs` (`ReqwestTransport`) + `equity_live.rs` into `prismatik-application/src/lib.rs` as compiled modules. `reqwest` is already pinned at workspace level and lives in Layer 4 (application) — boundary rule honored (no reqwest in domain crates).

**B5. Application ingest command**
- New `prismatik-application` ingest use-case: accepts `Arc<dyn Clock>` (SystemClock for live), `Arc<dyn HttpTransport>`, `Arc<dyn BudgetGovernor>`, `&SourcePolicyRegistry`, and the durable stores. Flow per briefing §52 Day 2 / Part IV §44.1:
  `budget.admit()` → adapter fetch → `ObservationDraft::new(&clock,…)` → `Observation::seal(draft, policy)` (policy hard-deny enforced) → durable observation append → audit event. Idempotent on retries via `ObservationId`. Bounded concurrency, per-source budget, backoff, cancellation (tokio).
- Replaces the in-memory-only `DefaultPrismatikApp` task stub with a real `IngestCommand`.

**B6. End-to-end ingest test**
- Live or cassette-backed: provider fetch → sealed observation → durable file written → audit ledger entry → `inclusion_proof` verifies. Assert `observation_time` stamped by the injected Clock, `prev_hash` chain intact, idempotent re-ingest returns `AlreadyPresent`.

**Phase B exit gate:** the Part IV §55 corpus-foundation DoD (items 1–11) is satisfiable for the one-wire provider. No `SourcePolicy` approvals are invented by the agent (briefing §54) — sources remain pending human terms review; the pipeline is exercised against a self-hosted/demo endpoint or cassette.

---

## Phase C — Reconcile drifted Tauri adapters; replace fixtures with real data
*Now that market-data exports adapters + governor + correct types, the 5 desktop adapter modules become fixable. Frontend already has the try-`invoke`-then-fallback seam, so real data flows automatically.*

**C1. Fix the 5 adapter modules** (`apps/desktop/src-tauri/src/{market,state,watchlist,equity_fixtures,research_fixtures}.rs`):
- `watchlist.rs`, `equity_fixtures.rs`: already compile clean (zero workspace deps) — no work.
- `market.rs`: rewrite against real exported adapters + `CassetteTransport`; remove phantom `CoinGeckoAuth`/`demo_cassette_json` references; coin-detail/markets now come from the adapter (or fallback to cassette when offline).
- `state.rs`: drop phantom `GcraBudgetGovernor` field-shape mismatches (`remaining_ratio`, `next_permit_at`, `Permit.id`), fix `PriorityClass`/`Entitlement` variants, add `Display` on `ProviderId` (or format via the u16). Now that B1 provides a real `GcraBudgetGovernor`, expose real budget state.
- `research_fixtures.rs`: the bulk of the 101 errors. Reconcile against ~13 owning crates per the per-crate breakdown: add `pub use` re-exports where types exist but aren't exported (calibration, security, options/hand_labels, manifest); fix field-shape drift (backtest `BacktestConfig`, portfolio `Position` decimal-String vs micros, simulation `MonteCarloConfig`, `StrategyIr`→`StrategyIR` case); add missing types per ORPHANED_MODULE_REMEDIATION.md (oss-registry marketplace, tsfm families) — **extend, don't delete**.

**C2. Re-enable `full-command-surface` by default.** Remove the feature gate so all commands register unconditionally; default and full builds converge.

**C3. Verify real data reaches the workspace.** Each of the 15 experience routes already calls `invoke()` first and falls back to fixtures on failure (the seam exists). With commands enabled, confirm the `invoke` path succeeds and the "sample evidence" / "desktop recovery · deterministic fixtures" labels are no longer shown for routes that have real backing. Add a typed `$lib/tauri.ts` (`isTauriRuntime()` is currently duplicated 3×) to dedupe — low-risk refactor, not required for data flow.

**Phase C exit gate:** `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml --features full-command-surface` clean with **zero** errors (down from ~101). Default Tauri build + Svelte check still green. `pnpm dev` shows real ingested data on at least the market/equity routes.

---

## Cross-cutting invariants (enforced throughout, per briefing §1/§2)
- **Never delete an orphaned module** to make the build pass; extend base types. `corpus.rs`/adapters/ledger are the spec.
- **Never relax** `#![warn(missing_docs, missing_debug_implementations)]` or `#![forbid(unsafe_code)]`. New public items get docs.
- **No `reqwest`/`governor` in domain crates** — only application. No ambient `SystemTime`/`Instant`/`thread_rng` outside determinism (use `SystemClock` for `retrieved_at` only, per clock.rs doc).
- **Cache keys** must include `DeterminismContext` + `as_of` + artifact set (briefing §2, I5/I6). Staleness hard-denies.
- **No `SourcePolicy` approvals invented** (§54); sources stay pending human terms review.
- **Honesty in docs/walkthrough** (§4): record failures, not just successes.

## Verification gates (final)
1. `cargo test --workspace` — fully GREEN (the 3 currently-failing targets fixed).
2. `cargo clippy --workspace --lib -- -D warnings` — clean.
3. `cargo fmt --all -- --check` — clean.
4. Default Tauri `cargo check` AND `--features full-command-surface` — both clean, 0 errors.
5. `pnpm --filter @prismatik/desktop check` — 0/0.
6. New end-to-end ingest test passes (B6).
7. Dev server shows real ingested data on market/equity routes.

## Out of scope for this pass (explicitly deferred)
- arrow/parquet columnar analytics plane (§37) — deferred until that layer is built; current store is behind `Repository` for a clean swap.
- Parts II/III crates (narrative/consensus/hypothesis/trend/etc.) — proposals, not scheduled.
- Native `wgpu` renderer — separately approved work (briefing §57).
- OS keychain secret management — separate security pass.
- All §21 licensing/regulatory decisions — human-only.

## Note on dirty worktree (briefing §66)
The current uncommitted work (`corpus.rs`, `integrations.rs`, `WindowChrome.svelte`, onboarding, the untracked jobs/sessions overlay) is the foundation I'm building **on top of**, not discarding. The untracked jobs/sessions/api.ts/stores.ts browser-dashboard overlay remains **not** reconnected to the root shell (briefing §56) — that stays out of scope unless you decide otherwise.