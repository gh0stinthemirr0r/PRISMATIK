# PRISMATIK Phased Implementation Plan v1.0

**Document:** `PRISMATIK_Phased_Implementation_Plan_v1.0.md`
**Companion to:** `PRISMATIK_Unified_Solution_Architecture_v1.0.md`
**Organization:** Mythos Systems
**Author:** Aaron Stovall
**Version:** 1.0.0
**Date:** 2026-07-24
**Status:** Proposed delivery plan of record

---

## 0. How to Use This Plan

The architecture document says what to build. This says in what order, why that order, how to know a phase is finished, and what to cut when the schedule tells the truth.

Every work item carries a stable identifier of the form `P{phase}-{track}-{n}`, for example `P0-DK-03`. Those identifiers are the primary key across the issue tracker, commit messages, ADRs, and the project workbook. They do not change when work moves between phases; a deferred item keeps its original identifier and gains a deferral note, because renumbering destroys the history of what was planned and when.

Six tracks run across the plan:

| Track | Code | Scope |
|---|---|---|
| Determinism and Kernel | DK | Clock, entropy, artifacts, manifest, audit ledger, identity, calendar |
| Data and Providers | DP | Provider ports, ingestion, storage, lineage, feature store |
| Quant and Models | QM | Strategy IR, backtest, indicators, TSFM, calibration, drift |
| Experience | EX | Design system, Svelte components, charts, workspaces, IPC bindings |
| Security and Supply Chain | SS | Keys, capabilities, sandbox, signing, SBOM, gates |
| Operations and Delivery | OD | CI, observability, packaging, updater, docs, runbooks |

---

## 1. Capacity Model and the Honest Total

### 1.1 The Unit

Estimates are in **engineering days**, defined as one focused day of a single senior engineer using AI assisted development tooling. This is not a person day of calendar time and it is not a story point. It is the unit that makes the total legible.

Estimates carry a confidence band because a plan that pretends to precision it does not have is worse than one that admits the range.

| Band | Meaning |
|---|---|
| **H** | High confidence. Well understood work, similar to work done before, plus or minus 20 percent. |
| **M** | Medium. Known approach, unknown friction, plus or minus 50 percent. |
| **L** | Low. Research or integration with an unfamiliar external system, could be 2x to 3x. |

### 1.2 The Total

| Phase | Objective | Days | Cumulative | Confidence |
|---|---|---:|---:|:---:|
| 0 | Foundation, governance, Determinism Kernel | 72 | 72 | M |
| 1 | Crypto intelligence MVP | 56 | 128 | M |
| 2 | Equity, filings, macro, real symbology | 54 | 182 | M |
| 3 | Options intelligence | 62 | 244 | M |
| 4 | Quant research, indicator kernel, plugin host | 78 | 322 | L |
| 5 | Simulation, TSFM registry, calibration | 61 | 383 | L |
| 5.5 | Analog engine upgrade | 14 | 397 | M |
| 6 | Portfolio, journal, paper trading | 48 | 445 | M |
| 7 | Controlled execution | 46 | 491 | L |
| 8 | Enterprise, on premises, team | 84 | 575 | L |
| 9 | Ecosystem and marketplace | 41 | 616 | L |

**616 engineering days.** At five focused days per week with no other obligations, that is roughly two and a half years. At the two to three focused days per week that a full time Senior Network Security Engineer realistically has available, it is five to seven years to Phase 9.

That number is not an argument against the architecture. It is an argument for reading Section 2 before Section 3.

### 1.3 What the Total Means

Three observations that should shape every decision below.

**First, Phase 0 produces nothing a customer can see.** Seventy two days of determinism kernels, Merkle ledgers, and CI gates, and at the end of it there is no product. For a funded team that is fine. For a one engineer commercial operation it is the single largest risk in the plan, larger than any technical risk in the register.

**Second, the phases are not equally optional.** Phases 0 through 3 build a coherent, sellable product. Phases 4 through 6 build the quantitative platform. Phases 7 through 9 build the enterprise business. These are three products with three different buyers, and the plan should stop treating them as one continuous ramp.

**Third, the architecture's own logic argues for front loading.** Section 12 of the architecture document is right that retrofitting determinism costs multiples of building it first. That logic is sound and it is also exactly the logic that produces a seventy two day gap before first value. Both things are true and Section 2 resolves the tension.

---

## 2. Two Sequencing Options

The architecture document presents one ordering. It is the correct ordering for a funded team. It is probably not the correct ordering for this operation, so both are laid out and the choice is explicit rather than accidental.

### 2.1 Option A, Architecture Order

Phases run 0 through 9 as written. Determinism, audit, and manifest are complete before the first provider adapter is written.

**Advantages.** Zero retrofit cost. Every subsequent line of code is written against the kernel from the start. The reproducibility claim is true from the first commit, not from a later date with an asterisk.

**Cost.** 128 days, roughly six to twelve calendar months at realistic solo capacity, before anything is demonstrable to a prospect. No revenue, no external validation, no feedback signal on whether the crypto wedge is the right wedge.

**Choose this if** the operation is funded, or if PRISMATIK is a long horizon asset with no near term revenue requirement, or if the enterprise buyer is the target and reproducibility is the reason they will buy.

### 2.2 Option B, Revenue First with a Determinism Floor

Phase 0 is split. A minimal, non negotiable floor ships first. The remainder of Phase 0 is deferred and interleaved into Phase 1 and Phase 2.

**The floor, `P0-FLOOR`, is 24 days, not 72.** It contains only the items whose retrofit cost is genuinely multiplicative:

| Item | Why it cannot be deferred |
|---|---|
| `Clock` and `Entropy` traits plus `DeterminismContext` | Every call site written without them must be rewritten. This is the multiplicative one. |
| Determinism lints and the grep gate | A gate added later has to be paid down against an existing violation set, which is how gates get disabled. |
| `AssetId` as the internal canonical identifier | Every table, every struct, every function signature. Changing the identity type later touches everything. |
| Workspace, `deny.toml`, `clippy.toml`, CI skeleton | Cheap now, tedious later. |
| Tauri shell with capability policy and strict CSP | Loosening a policy later is easy. Tightening one after features depend on the looseness is not. |
| `tauri-specta` bindings pipeline | Hand written types written now are hand written types deleted later. |

Everything else in Phase 0, the Merkle audit ledger, the manifest builder, dual signatures, calendar artifacts, the component registry, the golden manifest corpus, and the full DST suite, is **additive rather than invasive**. Adding an audit ledger to a system that already has a clock trait is a bounded piece of work. Adding a clock trait to a system that does not have one is not.

**The reordered plan:**

```text
  P0-FLOOR (24d) ──▶ Phase 1 Crypto MVP (56d) ──▶ SHIP AND CHARGE
                                                        │
                                                        ▼
                              P0-REMAINDER (48d) interleaved across
                              Phase 2 and Phase 3, funded by revenue
```

First sellable artifact at **80 days instead of 128**, a 37 percent reduction in time to first revenue, at the cost of carrying an explicit, tracked, and time boxed deferral.

**The condition, and it is not negotiable.** The deferred items are entered in the issue tracker on day one with their original identifiers, a stated deferral reason, and a hard deadline of "before Phase 4 begins." Phase 4 introduces the strategy runtime and the backtest engine, which is the first point where a missing manifest or a missing audit ledger becomes a correctness problem rather than a missing feature. If `P0-REMAINDER` is not complete when Phase 4 opens, Phase 4 does not open.

**Choose this if** the operation needs revenue or external validation before committing multiple years, which for a solo commercial operation is the usual case.

### 2.3 Recommendation

**Option B**, with the Phase 4 gate treated as genuinely hard.

The reasoning is not that the architecture is wrong. It is that a seventy two day investment in verifiability is only rational if the product it verifies turns out to be a product someone wants, and the crypto intelligence wedge in Phase 1 is the cheapest available test of that question. Running the test first is better risk management than deferring it behind two and a half months of infrastructure.

The counter argument deserves a fair hearing: the whole thesis of PRISMATIK is that verifiability *is* the product, so shipping a crypto tracker without it is shipping a different product. That argument is correct about the end state and wrong about the sequence. A Phase 1 crypto product without a Merkle ledger is still evidence chained, still provenance stamped, and still deterministic, because the floor includes the clock and the identity model. What it lacks is third party verifiability, which no Phase 1 customer will ask for. The differentiator arrives on schedule for the customers who actually value it.

The rest of this document is written in Option A phase numbering, because the phases are the same phases either way. Section 3.1 marks every item that belongs to `P0-FLOOR`.

---

## 3. Phase 0 — Foundation, Governance, and the Determinism Kernel

**Objective.** Establish the mechanisms whose absence would make every later phase more expensive: deterministic time and entropy, canonical identity, tamper evident audit, signed reproducibility manifests, generated type bindings, and the CI gates that keep all of it true.

**Entry criteria.** None. This is the start.

**Duration.** 72 days total. 24 days if scoped to `P0-FLOOR` under Option B.

### 3.1 Work Items

`FLOOR` marks items in the Option B minimum. `DEFER` marks items moved to `P0-REMAINDER` under Option B.

| ID | Track | Work item | Days | Conf | Option B |
|---|---|---|---:|:---:|:---:|
| P0-OD-01 | OD | Cargo workspace, crate skeletons, `rust-toolchain.toml` pinned, `.editorconfig`, `rustfmt.toml` | 2 | H | FLOOR |
| P0-OD-02 | OD | `clippy.toml` with determinism `disallowed-methods` and `disallowed-types`, wired to `-D warnings` | 1 | H | FLOOR |
| P0-OD-03 | OD | GitHub Actions CI skeleton: fmt, clippy, test, matrix on Windows and Linux | 2 | H | FLOOR |
| P0-OD-04 | OD | Project workbook, ADR template and index, `CONTRIBUTING.md`, `SECURITY.md` | 2 | H | FLOOR |
| P0-DK-01 | DK | `prismatik-determinism`: `Clock` trait with `SystemClock`, `SimulatedClock`, `FrozenClock` | 3 | M | FLOOR |
| P0-DK-02 | DK | `Entropy` trait, splittable stream implementation, split order independence property test | 4 | M | FLOOR |
| P0-DK-03 | DK | `DetMap`, `DetSet`, `DeterminismContext`, `PinnedArtifactSet` types | 2 | H | FLOOR |
| P0-DK-04 | DK | `determinism-grep` CI gate plus a deliberate violation test proving the gate fails | 2 | M | FLOOR |
| P0-DK-05 | DK | `prismatik-identity`: `AssetId`, `VenueId`, `ExternalIdentifier` enum, id factory backed by `Entropy` | 3 | M | FLOOR |
| P0-DK-06 | DK | Bitemporal symbology store: schema, `resolve_as_of`, identity chain, as of property test | 6 | M | DEFER |
| P0-DK-07 | DK | `prismatik-audit`: Merkle append only ledger, inclusion proof, consistency proof | 6 | M | DEFER |
| P0-DK-08 | DK | Audit startup verification plus `criterion` benchmark against the 1 ms p99 append budget | 2 | M | DEFER |
| P0-DK-09 | DK | `prismatik-manifest`: schema v1.0 types, canonical serialization, builder | 4 | M | DEFER |
| P0-DK-10 | DK | Dual signature (Ed25519 plus ML-DSA), sign and verify, cross machine verification test | 5 | L | DEFER |
| P0-DK-11 | DK | `prismatik-cli verify` standalone verifier, no application dependency | 3 | M | DEFER |
| P0-DK-12 | DK | `ArtifactStore` trait, content addressed storage, hash and signature verification on load | 3 | M | DEFER |
| P0-DK-13 | DK | `prismatik-calendar`: artifact format, `SessionCalendar` trait, reader | 3 | M | DEFER |
| P0-DK-14 | DK | Calendar artifact generator script with QuantLib cross validation and zero tolerance gate | 5 | L | DEFER |
| P0-SS-01 | SS | Tauri capability policy, default deny, strict CSP with no inline or eval | 3 | M | FLOOR |
| P0-SS-02 | SS | Key hierarchy: OS keychain integration, device root key, derived key types, `secrecy` and `zeroize` | 5 | M | DEFER |
| P0-SS-03 | SS | `deny.toml` license allowlist and advisory policy, blocking in CI | 2 | H | FLOOR |
| P0-SS-04 | SS | `cargo vet init`, import Mozilla, Google, Bytecode Alliance audit sets, trusted core policy | 3 | M | DEFER |
| P0-SS-05 | SS | `prismatik-oss-registry`: manifest schema, license class gate, coverage CI check | 4 | M | DEFER |
| P0-SS-06 | SS | Release signing with cosign, SBOM via `cargo cyclonedx`, SLSA provenance generation | 4 | L | DEFER |
| P0-SS-07 | SS | Updater with provenance **verification** and a test proving it refuses an invalid artifact | 4 | L | DEFER |
| P0-EX-01 | EX | Tauri 2 shell, window management, crash recovery, single instance | 3 | M | FLOOR |
| P0-EX-02 | EX | `tauri-specta` bindings pipeline, exact version pins, `bindings-drift` CI gate | 3 | M | FLOOR |
| P0-EX-03 | EX | Design tokens package, CSS variable theme, light and dark, contrast validation | 4 | M | FLOOR |
| P0-EX-04 | EX | Motion system primitives, reduced motion support, first five production components | 5 | M | FLOOR |
| P0-QM-01 | QM | Skeleton `dst_replay_suite` on a trivial pipeline, trace capture and digest comparison | 4 | L | DEFER |
| P0-QM-02 | QM | Golden manifest corpus harness plus first two committed manifests | 3 | M | DEFER |

Floor subtotal: 24 days. Deferred subtotal: 48 days. Phase total: 72 days.

### 3.2 Sequencing Within Phase 0

The internal order matters because several items are blocking.

```text
Week 1-2   P0-OD-01 ─▶ P0-OD-02 ─▶ P0-OD-03 ─▶ P0-OD-04
                          │
Week 2-4                  ├─▶ P0-DK-01 ─▶ P0-DK-02 ─▶ P0-DK-03 ─▶ P0-DK-04
                          │                                          │
Week 4-5                  └─▶ P0-DK-05 ◀───────────────────────────┘
                                  │
Week 5-7   P0-EX-01 ─▶ P0-SS-01 ─▶ P0-EX-02 ─▶ P0-EX-03 ─▶ P0-EX-04
                                                    │
           ══════ OPTION B FLOOR COMPLETE (day 24) ═╪══════ proceed to Phase 1
                                                    │
Week 8-10  P0-DK-07 ─▶ P0-DK-08 ─▶ P0-DK-09 ─▶ P0-DK-10 ─▶ P0-DK-11
Week 11-12 P0-DK-12 ─▶ P0-DK-13 ─▶ P0-DK-14
Week 13-14 P0-SS-02 ─▶ P0-SS-04 ─▶ P0-SS-05 ─▶ P0-SS-06 ─▶ P0-SS-07
Week 15    P0-DK-06 ─▶ P0-QM-01 ─▶ P0-QM-02
```

`P0-DK-04`, the grep gate, must land before any other crate is written. Its entire value is preventing violations from accumulating, and a gate introduced after violations exist gets an allowlist, and an allowlist is how gates die.

### 3.3 Exit Gate

A phase is complete when every line below is demonstrably true, verified by a named artifact rather than by assertion.

| # | Criterion | Verified by |
|---|---|---|
| 1 | No call to `Instant::now`, `SystemTime::now`, `thread_rng`, or a default hasher map exists outside the determinism crate allowlist | `determinism-grep` green, allowlist reviewed and under 10 entries |
| 2 | The grep gate demonstrably fails on an introduced violation | Committed negative test |
| 3 | Entropy split is order independent | `proptest` case, 10k iterations |
| 4 | A trivial pipeline replays byte identically across 64 seeds | `dst_replay_suite` green |
| 5 | A manifest signed on machine A verifies on machine B with no shared state | Manual cross machine run, recorded in the workbook |
| 6 | Both signatures are required; a single valid signature fails verification | Committed negative test |
| 7 | The audit ledger detects a retroactive edit | Committed tamper test |
| 8 | Audit append meets 1 ms p99 | `criterion` report committed |
| 9 | The updater refuses an artifact with invalid SLSA provenance | Committed negative test |
| 10 | Generated TypeScript bindings match the Rust source | `bindings-drift` green |
| 11 | Every workspace dependency appears in the component registry | `registry-coverage` green |
| 12 | Calendar generator produces zero disagreements against QuantLib for US equity venues | Generator report committed |
| 13 | The Tauri capability policy contains no `fs`, `http`, `process`, or `shell:execute` permission | Policy file review, recorded in ADR |
| 14 | ADRs 0020 through 0027 are written and accepted | ADR index |

**Under Option B**, criteria 1, 2, 3, 10, and 13 gate the floor. The remainder gate the opening of Phase 4.

### 3.4 Phase 0 Risks

| Risk | Response |
|---|---|
| ML-DSA implementation maturity in Rust is uneven, `P0-DK-10` could run 2x | Time box to 8 days. If it overruns, ship Ed25519 only with the dual signature *format* in place and the ML-DSA field present but empty, so the retrofit is a field population rather than a schema migration. Record as an explicit exception in the PQC register. |
| Specta v2 RC breaks on a Tauri patch release | Exact pins, and a `cargo update` is a deliberate reviewed act. Fallback ADR-0031 already written. |
| Determinism lints produce excessive friction and get disabled | The allowlist is capped at 10 entries and every addition requires a one line justification in the file. A cap makes pressure visible. |
| The 24 day floor slips to 40 and Option B loses its advantage | Weekly checkpoint against the floor item list. If day 30 arrives with the floor incomplete, cut `P0-EX-03` and `P0-EX-04` to a single unstyled component set and finish the styling during Phase 1. |

---

## 4. Phase 1 — Crypto Intelligence MVP

**Objective.** A complete, sellable crypto intelligence product on the Phase 0 foundations. This is the first artifact a customer sees and the first test of whether the wedge is real.

**Entry criteria.** Phase 0 exit gate, or the `P0-FLOOR` subset under Option B.

**Duration.** 56 days.

### 4.1 Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P1-DP-01 | DP | `Provider` trait, `ProviderCapabilities`, `EntitlementSet`, `ProviderHealth` | 3 | M |
| P1-DP-02 | DP | `BudgetGovernor` on `governor` GCRA, priority classes, exact next permit times | 4 | M |
| P1-DP-03 | DP | CoinGecko adapter: auth, search, global, markets, coin detail, market chart, OHLC, categories, exchanges | 8 | M |
| P1-DP-04 | DP | Provider contract tests with recorded cassettes, replayable offline | 3 | M |
| P1-DP-05 | DP | Raw layer: Parquet writer, partitioning, append only enforcement, supersession links | 4 | M |
| P1-DP-06 | DP | Normalization to canonical types, quality scoring, deduplication | 4 | M |
| P1-DP-07 | DP | DuckDB analytical views over curated Parquet | 3 | M |
| P1-DP-08 | DP | SQLite operational state, migrations, task graph persistence | 3 | M |
| P1-DP-09 | DP | Evidence and lineage plane, `EvidenceRef`, `Concludes` trait, invalidation hash including pinned set | 5 | M |
| P1-DP-10 | DP | In process Tokio task graph, `PipelineTask`, triggers, crash recovery from SQLite | 5 | M |
| P1-DP-11 | DP | LanceDB integration, asset and category description embeddings, `AnalogStore` skeleton | 4 | M |
| P1-EX-01 | EX | Command palette, workspace shell, panel system, layout persistence | 5 | M |
| P1-EX-02 | EX | `ChartDocument` and `ChartBackend` contracts, Lightweight Charts wrapper, theme bridge | 6 | M |
| P1-EX-03 | EX | Crypto command dashboard: global stats, dominance, trending, category map | 5 | M |
| P1-EX-04 | EX | Watchlist with live updates, virtualized rows, 500 row budget | 4 | M |
| P1-EX-05 | EX | Asset workspace: profile, chart, markets, exchanges, categories, evidence drawer | 6 | M |
| P1-EX-06 | EX | Market scanner with saved filters and result provenance | 4 | M |
| P1-EX-07 | EX | Rate budget UI showing exact countdowns from GCRA, not spinners | 2 | H |
| P1-EX-08 | EX | Empty, loading, error, and degraded states for every surface | 3 | M |
| P1-QM-01 | QM | Alert rule evaluator, streaming and scheduled split, dedup keys | 5 | M |
| P1-QM-02 | QM | Notification service, delivery channel abstraction, escalation | 4 | M |
| P1-OD-01 | OD | First run experience state machine, demo mode, suitability profile | 5 | M |
| P1-OD-02 | OD | Backup, restore, cross version migration | 4 | M |
| P1-OD-03 | OD | OpenTelemetry tracing with determinism telemetry attributes | 3 | M |
| P1-OD-04 | OD | Installer, code signed, auto update flow end to end | 4 | L |
| P1-OD-05 | OD | User documentation, licensing and disclosure surfaces, notice generator | 4 | M |

### 4.2 The Commercial Milestone

`P1-OD-04` is the item that turns a codebase into a product. It is placed near the end because everything before it is a prerequisite, and it is called out here because a solo operation can spend a year building and never ship an installer.

**Suggested checkpoint at day 40 of Phase 1:** package whatever exists, install it on a clean machine, and use it for a week as a user rather than as its author. Every defect that surfaces in that week is a defect a customer would have found. This checkpoint costs two days and reliably returns more than it costs.

### 4.3 Exit Gate

| # | Criterion | Verified by |
|---|---|---|
| 1 | Every rendered conclusion resolves a complete evidence chain to raw records | `test_no_orphan_conclusions` green |
| 2 | The rate budget UI shows an exact retry time, never an indeterminate spinner | Manual verification against a throttled key |
| 3 | The application functions in degraded mode when CoinGecko is unreachable | Chaos test with the provider blackholed |
| 4 | Stale data is visibly marked on every surface that displays it | Manual verification with a frozen feed |
| 5 | A clean machine install completes and auto update applies a signed increment | Recorded on a fresh VM |
| 6 | Cold start to interactive is under 2.0 s p95 on mid tier hardware | Benchmark report |
| 7 | Chart pan and zoom at 1M points holds a 16.7 ms p99 frame | Benchmark report |
| 8 | Provider contract tests pass offline from cassettes | CI green with network disabled |
| 9 | A backup taken on version N restores on version N plus 1 | Migration test |
| 10 | The product has been used for one continuous week by its author as a user | Workbook log |

### 4.4 Phase 1 Risks

| Risk | Response |
|---|---|
| CoinGecko free tier limits make the product feel slow | Aggressive caching keyed on content, plus the budget governor surfacing honest wait times. If the free tier is unusable, the paid tier becomes a stated prerequisite and the pricing model absorbs it. |
| The crypto wedge does not find buyers | This is the point of shipping early. If Phase 1 finds no buyers, that is a 128 day loss rather than a 400 day loss, and Phase 2 equity or Phase 3 options becomes the wedge instead. |
| Feature creep from a demo that impresses | The exit gate is the scope. New ideas go to a Phase 2 candidate list, not into Phase 1. |

---

## 5. Phase 2 — Equity, Filings, Macro, and Real Symbology

**Objective.** Extend to US equities with regulatory and macro context, and make bitemporal symbology load bearing, because equity history is wrong without it.

**Entry criteria.** Phase 1 exit gate. **Under Option B, `P0-DK-06` bitemporal symbology moves here and is mandatory, not deferred further.**

**Duration.** 54 days.

### 5.1 Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P2-DK-01 | DK | Bitemporal symbology production hardening, OpenFIGI ingestion, identity chain | 6 | M |
| P2-DK-02 | DK | Corporate action ledger, event types including `OptionAdjustment`, conflict retention | 5 | M |
| P2-DK-03 | DK | Read time adjustment factor computation from the ledger, never destructive rewrite | 4 | M |
| P2-DK-04 | DK | Calendar artifacts for NYSE, NASDAQ, ARCA, BATS with QuantLib cross validation | 4 | M |
| P2-DP-01 | DP | SEC EDGAR adapter: submissions, company facts, filing index, full text search | 7 | M |
| P2-DP-02 | DP | Filing parser: 13F, Forms 3, 4, 5, 8-K item extraction, SC 13D and 13G | 8 | L |
| P2-DP-03 | DP | CFTC Commitments of Traders adapter and weekly schedule | 4 | M |
| P2-DP-04 | DP | FRED adapter, series metadata, release calendar, vintage handling | 4 | M |
| P2-DP-05 | DP | Equity OHLCV provider integration behind the `Provider` port | 4 | M |
| P2-DP-06 | DP | Feature store: `FeatureView`, `observation_delay`, offline point in time join | 6 | M |
| P2-DP-07 | DP | Point in time property test suite | 2 | H |
| P2-EX-01 | EX | Institutional intelligence: 13F views, ownership, insider activity | 6 | M |
| P2-EX-02 | EX | Event and catalyst engine with the earnings and macro calendar | 5 | M |
| P2-EX-03 | EX | Universal instrument workspace generalized across asset classes | 5 | M |
| P2-EX-04 | EX | Perspective integration for streaming analytical grids | 4 | M |

### 5.2 Exit Gate

| # | Criterion | Verified by |
|---|---|---|
| 1 | A 2015 to 2026 equity universe resolves every ticker correctly through renames, splits, and mergers | Hand checked set of 50 known identity events |
| 2 | Corporate actions are applied at read time; the raw series is unmodified | Byte comparison of raw Parquet before and after a split ingestion |
| 3 | No feature value is returned whose `event_time` plus `observation_delay` exceeds `as_of` | Property test, 10k cases |
| 4 | 13F holdings are not observable before their filing date, only their period end | Targeted test on a known filing |
| 5 | Calendar artifacts show zero disagreement with QuantLib across all four venues | Generator report |
| 6 | Adjusted option contracts are flagged in the corporate action ledger | Test against a known OCC memo |

### 5.3 Phase 2 Risks

| Risk | Response |
|---|---|
| `P2-DP-02` filing parsing is the largest low confidence item in the phase | Scope to 13F and Form 4 only for the gate. 8-K item extraction and 13D/G move to a Phase 2.5 candidate list. Partial filing coverage is a feature gap; wrong filing parsing is a correctness failure. |
| SEC rate limits and user agent requirements | Implement the declared user agent and the ten requests per second ceiling in the adapter, governed by the same `BudgetGovernor`. |
| Equity price data licensing | Resolve the provider and licensing question **before** Phase 2 opens, not during. This is a Phase 1 background task. |

---

## 6. Phase 3 — Options Intelligence

**Objective.** The differentiating wedge. Options flow, chain analytics, volatility, and structure construction.

**Entry criteria.** Phase 2 exit gate.

**Duration.** 62 days.

### 6.1 Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P3-DP-01 | DP | Unusual Whales adapter behind the `Provider` port, entitlement mapping | 6 | M |
| P3-DP-02 | DP | Options chain normalization, OCC symbology, contract identity | 5 | M |
| P3-DP-03 | DP | Flow observation ingestion at print level, ClickHouse for cloud profiles | 6 | L |
| P3-DP-04 | DP | Historical chain and IV surface storage, compression strategy | 5 | M |
| P3-QM-01 | QM | `prismatik-quant-kernel`: pricing, greeks, IV solve, selected RustQuant modules | 7 | M |
| P3-QM-02 | QM | QuantLib conformance sidecar, 1e-8 tolerance gate on pricing and greeks | 5 | L |
| P3-QM-03 | QM | Flow classification: directional, hedging, closing, spread leg detection | 8 | L |
| P3-QM-04 | QM | Flow clustering and aggregation | 5 | L |
| P3-QM-05 | QM | Trade quality score with a disclosed and versioned formula | 4 | M |
| P3-EX-01 | EX | Chain explorer with dense grid, greeks columns, and liquidity shading | 6 | M |
| P3-EX-02 | EX | Volatility lab: term structure, skew, IV rank, surface via wgpu | 7 | L |
| P3-EX-03 | EX | Strategy constructor with payoff diagram and break even analysis | 6 | M |
| P3-EX-04 | EX | Dealer exposure views | 4 | M |
| P3-SS-01 | SS | `adjusted_contract` pre trade check wired to the corporate action ledger | 2 | H |

### 6.2 Exit Gate

| # | Criterion | Verified by |
|---|---|---|
| 1 | Pricing and greeks match QuantLib within 1e-8 relative across a 500 case grid | Conformance report |
| 2 | Flow classification decisions expose their evidence and their confidence | UI review against 20 hand labelled prints |
| 3 | Adjusted contracts are visibly flagged and blocked from automation | Test against a known OCC adjustment |
| 4 | The IV surface renders at 60 fps with a Canvas fallback that is functional | Benchmark plus fallback test |
| 5 | Trade quality score formula and version appear alongside every score | UI review |

### 6.3 Phase 3 Risks

| Risk | Response |
|---|---|
| Flow classification is genuinely hard and partially unknowable | The architecture already names this in v0.1 §35.5. Ship classification with explicit confidence and an "unclassified" outcome that is used freely. A classifier that always decides is a classifier that is often wrong. |
| Options data costs are material and recurring | Model the cost into pricing before Phase 3 opens. |
| ClickHouse introduction adds operational surface | Cloud and enterprise profiles only. Desktop stays on Parquet and DuckDB, per architecture §15.2. |

---

## 7. Phase 4 — Quant Research, Indicator Kernel, and Plugin Host

**Objective.** Strategy authoring, deterministic backtesting, the canonical indicator kernel, and the sandboxed plugin host.

**Entry criteria.** Phase 3 exit gate **and, under Option B, complete `P0-REMAINDER`. This gate is hard.** Phase 4 is the first phase where a missing manifest or a missing audit ledger is a correctness failure rather than a missing feature.

**Duration.** 78 days.

### 7.1 Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P4-QM-01 | QM | `StrategyIR` types, `StrategyCapabilities`, serialization, schema version | 4 | M |
| P4-QM-02 | QM | DSL lexer on `logos`, recursive descent parser, diagnostics with spans | 8 | L |
| P4-QM-03 | QM | DSL type checker and capability inference from source references | 6 | L |
| P4-QM-04 | QM | Data access compilation to DataFusion logical plans | 6 | L |
| P4-QM-05 | QM | Point in time enforcement as a DataFusion plan rewrite rule | 4 | L |
| P4-QM-06 | QM | `Strategy` trait, runtime, `StrategyContext` over `DeterminismContext` | 5 | M |
| P4-QM-07 | QM | Backtest engine: event loop over pinned calendar bar boundaries | 7 | M |
| P4-QM-08 | QM | `ExecutionAssumptions`, fill models, slippage, commission, assignment | 6 | M |
| P4-QM-09 | QM | Backtest metrics, walk forward, out of sample, deflated Sharpe | 5 | M |
| P4-QM-10 | QM | `Indicator` trait, warmup enforcement, `IndicatorDescriptor`, provenance | 4 | M |
| P4-QM-11 | QM | YATA adapter plus the first 30 indicators with golden vectors | 8 | M |
| P4-QM-12 | QM | Streaming equals batch property test across the full indicator set | 2 | H |
| P4-SS-01 | SS | `prismatik-plugin-host`: wasmtime hardened engine config plus config assertion test | 4 | M |
| P4-SS-02 | SS | `CapabilitySet`, host function registry, time and entropy from the kernel | 5 | M |
| P4-SS-03 | SS | Plugin signing, install flow, capability grant recorded in the audit ledger | 4 | M |
| P4-SS-04 | SS | Capability containment test: syscall trace proving zero network from a denied plugin | 3 | L |
| P4-EX-01 | EX | Visual strategy builder emitting `StrategyIR` via codegen | 8 | L |
| P4-EX-02 | EX | Backtest result workspace: equity curve, drawdown, trade list, metrics | 6 | M |
| P4-EX-03 | EX | Rust SDK crate and documentation for hand written strategies | 4 | M |
| P4-QM-13 | QM | Python `StrategyIR` emitter in the research sidecar, JSON schema and validator | 4 | M |

### 7.2 Exit Gate

| # | Criterion | Verified by |
|---|---|---|
| 1 | The same logical strategy authored in the visual builder, the DSL, and the Rust SDK produces identical `StrategyIR` | Three way comparison test |
| 2 | Those three produce identical backtest results | Byte comparison of result bundle hashes |
| 3 | A strategy compiled without `can_access_network` cannot reach a socket even when its code tries | Negative test with a deliberately malicious strategy |
| 4 | `evaluate` is bitwise identical to the `next` sequence for all 30 indicators | Property test |
| 5 | A plugin with no granted capabilities makes zero syscalls of the network or filesystem class | `strace` or ETW trace, committed |
| 6 | Relaxed SIMD is disabled and the engine config assertion test passes | Committed test |
| 7 | A backtest emits a complete signed manifest that the standalone verifier accepts | End to end run |
| 8 | `CloseOnly` fill model results carry a mandatory badge in the UI | UI review |
| 9 | Full `dst_replay_suite` over ingest to manifest passes 256 seeds | CI green |

### 7.3 Phase 4 Risks

| Risk | Response |
|---|---|
| The DSL is the largest low confidence cluster in the plan, 24 days across four items | Build the type checker and IR **first** and the parser second. A parser targeting a proven IR is bounded work; a parser and an IR designed together is not. If the DSL overruns by more than 50 percent, ship Phase 4 with the visual builder and Rust SDK only and move the DSL to Phase 4.5. |
| wasmtime advisory during the phase | Pinned version plus blocking `cargo deny`. An advisory is a same week patch, not a redesign, because the config hardening is already in place. |
| Visual builder scope expands without limit | The builder emits `StrategyIR` and nothing else. Any capability not expressible in the IR is out of scope by construction. |

---

## 8. Phase 5 — Simulation, TSFM Registry, and Calibration

**Objective.** Monte Carlo simulation, the plural TSFM registry, and conformal calibration. This phase makes Invariant I2 real.

**Entry criteria.** Phase 4 exit gate.

**Duration.** 61 days.

### 8.1 Work Items

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
| P5-QM-09 | QM | Kronos, Chronos-Bolt, and Lag-Llama artifact onboarding through the registry | 5 | L |
| P5-QM-10 | QM | Forecast caching keyed on asset, modality, and context hash | 3 | M |
| P5-QM-11 | QM | `Calibrator` trait, `CalibrationRecord`, `CalibrationMethod` types | 4 | M |
| P5-QM-12 | QM | Calibration sidecar: MAPIE ACI and EnbPI, crepes Mondrian, Arrow transport | 6 | L |
| P5-QM-13 | QM | Baseline ladder enforcement at promotion, both discrimination and calibration | 4 | M |
| P5-QM-14 | QM | `DriftDetector` suite: feature, calibration, embedding, token usage, performance | 6 | L |
| P5-QM-15 | QM | Drift actions: annotate, widen, suppress, propose demotion | 3 | M |
| P5-EX-01 | EX | Monte Carlo visualization: path clouds via wgpu, distribution panels | 5 | M |
| P5-EX-02 | EX | Calibration ribbon primitive, realized coverage against nominal | 4 | M |
| P5-EX-03 | EX | Model card surface, drift status, blind spot disclosure | 4 | M |

### 8.2 Exit Gate

This is the strictest gate in the plan. Every criterion is enforced by a test, not by review.

| # | Criterion | Verified by |
|---|---|---|
| 1 | No forecast reaches the UI without an attached `CalibrationRecord` | Type system, no constructor exists that omits it |
| 2 | The registry hard denies a CC BY-NC artifact in a commercial profile | Negative test with a synthetic Moirai style registration |
| 3 | The backtest runtime hard denies a model whose pretraining cutoff violates the window, and logs a security event | Negative test plus ledger inspection |
| 4 | A tokenizer and model version mismatch fails to load rather than degrading silently | Negative test |
| 5 | A rung 4 model that loses to rung 1 on calibration cannot be promoted | Negative test |
| 6 | Embedding drift above threshold automatically widens intervals with no human action | Simulated drift test |
| 7 | Monte Carlo with 1M paths under Rayon reproduces byte identically across runs and thread counts | Determinism test at 1, 4, and 16 threads |
| 8 | Per regime coverage is displayed alongside every forecast | UI review |
| 9 | TSFM forecast p95 latency is at or under 500 ms single asset single modality | Benchmark report |

Criterion 7 deserves emphasis: byte identical results across **different thread counts** is the property that proves the split entropy design works. Same thread count reproducibility is much weaker and much easier to achieve accidentally.

### 8.3 Phase 5 Risks

| Risk | Response |
|---|---|
| TSFM inference on desktop CPU misses the 500 ms budget | Fall back to Chronos-Bolt, which is the distilled variant and substantially faster. If that also misses, TSFM becomes a Team Cloud feature and the desktop profile ships the classical model ladder only. The architecture already supports this through the profile table. |
| Kronos becomes unmaintained mid phase | Plural registry already mitigates. Chronos and TimesFM are Apache-2.0 and integrated through the same trait. |
| Conformal calibration is applied incorrectly to non exchangeable data | ACI is the default by type; standard split conformal is restricted to cross sectional tasks. Per regime coverage reporting makes a misapplication visible rather than silent. |
| The calibration sidecar adds a Python runtime dependency to the desktop install | Sidecar is opt in on desktop per the profile table. Calibration records can be computed on a schedule and shipped as signed artifacts rather than computed locally. |

---

## 9. Phase 5.5 — Analog Engine Upgrade

**Objective.** TSFM embeddings become the primary analog similarity space with hand crafted features as secondary filters.

**Entry criteria.** Phase 5 exit gate.

**Duration.** 14 days.

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P55-QM-01 | QM | Embedding extraction from `TsfmRuntime::embed`, batch materialization | 3 | M |
| P55-QM-02 | QM | `AnalogStore` production implementation, LanceDB IVF-PQ index, version pinning | 4 | M |
| P55-QM-03 | QM | `AnalogQuery` with secondary filters, regime, sector, event type, venue | 3 | M |
| P55-QM-04 | QM | Leave N out sensitivity computation | 2 | M |
| P55-EX-01 | EX | Analog result surface with mandatory disclosures and sensitivity display | 2 | M |

**Exit gate.** Analog search returns neighbours only alongside distance metric, sample size, applied filters, survivorship warning, and leave N out sensitivity, with no code path that renders neighbours without them. Search of the top 50 across 10M vectors completes within 120 ms p95.

---

## 10. Phase 6 — Portfolio, Journal, and Paper Trading

**Objective.** Portfolio accounting on the audit ledger, the learning loop, and a paper broker that behaves like a real one.

**Entry criteria.** Phase 5.5 exit gate.

**Duration.** 48 days.

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P6-DK-01 | DK | Audit ledger becomes the portfolio write path, projections as read models | 6 | L |
| P6-DK-02 | DK | Projection rebuild from ledger replay, startup reconciliation | 4 | M |
| P6-DK-03 | DK | Divergence detection between projection and ledger, alert on mismatch | 3 | M |
| P6-QM-01 | QM | Portfolio model, positions, lots, cost basis, realized and unrealized PnL | 6 | M |
| P6-QM-02 | QM | Risk measures, exposure, concentration, correlation, scenario analysis | 6 | M |
| P6-QM-03 | QM | `RiskPolicy` and the full 16 check pre trade catalogue with fixed ordering | 6 | M |
| P6-QM-04 | QM | Paper broker with realistic fill, latency, and rejection behaviour | 5 | M |
| P6-EX-01 | EX | Portfolio workspace, exposures, risk panel, scenario runner | 6 | M |
| P6-EX-02 | EX | Journal: entry capture, thesis linkage, outcome tagging, review workflow | 5 | M |
| P6-QM-05 | QM | Post trade learning: realized versus expected, thesis validation reporting | 4 | M |
| P6-QM-06 | QM | TimesFM onboarded to the registry | 2 | M |

**Exit gate.**

| # | Criterion |
|---|---|
| 1 | Portfolio state rebuilt from a full ledger replay matches the live projection exactly |
| 2 | A deliberately corrupted projection is detected at startup and rebuilt |
| 3 | Cash plus market value plus realized PnL reconciles against the ledger, property tested |
| 4 | Every pre trade check evaluates in the declared fixed order, with all results recorded including passes |
| 5 | Paper broker rejections and partial fills are exercised and handled |
| 6 | Journal entries link to the evidence chain that produced the thesis |

**Risk.** `P6-DK-01` is the inversion described in architecture §14.2 and is the highest risk item in this phase. Build the ledger write path alongside the conventional path first, compare them continuously for a full phase, and remove the conventional path only once divergence has been zero across the phase. Cutting over on day one is the version of this that goes wrong.

---

## 11. Phase 7 — Controlled Execution

**Objective.** Live order submission through the deterministic execution boundary.

**Entry criteria.** Phase 6 exit gate. **This phase does not open on schedule pressure. It opens when Phases 0 through 6 have passed their gates.**

**Duration.** 46 days.

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P7-QM-01 | QM | `BrokerGateway` trait, `SubmissionResult` including the `Unknown` variant | 4 | M |
| P7-QM-02 | QM | Idempotency key generation, collision detection, replay safety | 4 | M |
| P7-QM-03 | QM | Reconciliation: delta computation, quarantine, resolution, ledger append | 7 | L |
| P7-QM-04 | QM | First broker adapter, full lifecycle including cancel and replace | 8 | L |
| P7-QM-05 | QM | Fill stream handling, partial fills, out of order events | 5 | M |
| P7-SS-01 | SS | Step up authorization, live execution enablement, privilege audit | 5 | M |
| P7-SS-02 | SS | Emergency stop: cancel all, flatten, halt automation, with a tested runbook | 4 | M |
| P7-EX-01 | EX | Order ticket, intent review, risk decision display, approval flow | 6 | M |
| P7-OD-01 | OD | Execution runbooks, incident procedures, reconciliation playbook | 3 | M |

**Exit gate.**

| # | Criterion |
|---|---|
| 1 | A submission timeout produces `Unknown`, quarantines the instrument, and does not retry |
| 2 | Reconciliation resolves a divergence and appends the resolution to the ledger |
| 3 | A failed reconciliation leaves the instrument quarantined and notifies the user |
| 4 | The same idempotency key submitted twice produces exactly one broker order, property tested |
| 5 | Live execution requires explicit enablement plus step up authorization, both audited |
| 6 | Emergency stop halts all automation and cancels working orders within a stated time bound |
| 7 | Every risk denial is recorded with the portfolio snapshot at evaluation time |
| 8 | Chaos test: broker disconnection mid submission is handled without duplicate orders |

**Risk.** The `Unknown` path is the item that separates a working execution system from a dangerous one, and it is the one most likely to be under tested because it is hard to provoke. Build a broker simulator that produces timeouts, duplicate acknowledgements, out of order fills, and post submission disconnections on demand, and make it part of CI. This is `P7-QM-03` and it is why that item carries seven days.

---

## 12. Phase 8 — Enterprise, On Premises, and Team

**Objective.** Multi user deployment, tenancy, air gapped operation, and organizational model governance.

**Entry criteria.** Phase 7 exit gate plus a signed enterprise customer or a credible pipeline. **Do not build this speculatively.** Eighty four days of enterprise infrastructure with no enterprise customer is the most expensive mistake available in this plan.

**Duration.** 84 days.

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P8-DP-01 | DP | PostgreSQL backend, migrations, connection management | 6 | M |
| P8-DP-02 | DP | Multi tenant isolation model, row level security, tenant scoped keys | 8 | L |
| P8-DP-03 | DP | NATS JetStream event transport with the shared envelope | 5 | M |
| P8-DP-04 | DP | S3 compatible object storage for artifacts and bundles | 4 | M |
| P8-DP-05 | DP | OpenLineage projection and emission sink | 3 | M |
| P8-SS-01 | SS | OIDC and passkey authentication, session policy | 6 | M |
| P8-SS-02 | SS | RBAC and ABAC policy engine, approval workflows | 8 | L |
| P8-SS-03 | SS | Vault, KMS, and HSM integration for server keys | 6 | L |
| P8-SS-04 | SS | Air gapped build: vendored crates, private registry, offline artifact staging | 6 | L |
| P8-OD-01 | OD | Container images, Kubernetes manifests, Terraform modules | 8 | M |
| P8-OD-02 | OD | Admin console: providers, entitlements, health, cost, audit export | 8 | M |
| P8-OD-03 | OD | Enterprise observability: metrics, SLOs, alerting, runbooks | 5 | M |
| P8-QM-01 | QM | Organization specific TSFM fine tuning through the governance pipeline | 6 | L |
| P8-QM-02 | QM | Fine tune manifests: base model, dataset hash, differential privacy posture | 4 | L |
| P8-OD-04 | OD | Temporal evaluation ADR and, if adopted, durable workflow migration | 5 | L |

**Exit gate.** Two tenants cannot observe each other's data under any query path, verified by a red team pass. An air gapped install completes from staged artifacts with no network. Audit export produces a verifiable bundle. Fine tuned models pass the same registration gates as pretrained ones, including license class and contamination review.

---

## 13. Phase 9 — Ecosystem and Marketplace

**Objective.** Third party extension and published contracts.

**Entry criteria.** Phase 8 exit gate plus demonstrated third party demand.

**Duration.** 41 days.

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P9-SS-01 | SS | Plugin marketplace: submission, review, signing, revocation | 10 | L |
| P9-SS-02 | SS | Plugin capability review workflow and publisher identity | 6 | M |
| P9-OD-01 | OD | Publish plugin SDK, chart contracts, indicator trait, manifest schema under Apache-2.0 | 6 | M |
| P9-OD-02 | OD | Public documentation site, examples, contribution guide | 8 | M |
| P9-QM-01 | QM | Third party manifest verification service and public tree head publication | 6 | L |
| P9-EX-01 | EX | In application marketplace surface with capability disclosure before install | 5 | M |

**Exit gate.** A third party can build, sign, submit, and publish a plugin without Mythos Systems writing code. A third party can verify a PRISMATIK research bundle using only the published schema and the standalone verifier, with no PRISMATIK installation.

---

## 14. Continuous Tracks

Four activities run across every phase and are budgeted at a percentage rather than as work items, because scheduling them as discrete tasks guarantees they get cut.

| Track | Budget | Content |
|---|---|---|
| Security maintenance | 5 percent of every phase | Advisory response, dependency updates, pinned version bumps, threat model revision |
| Documentation | 8 percent of every phase | ADRs, API docs, runbooks, model cards, the workbook |
| Test debt | 7 percent of every phase | Property test expansion, DST seed corpus growth, conformance vector additions |
| Refactoring | 5 percent of every phase | Boundary corrections, crate splits, naming consistency |

Twenty five percent overhead. That is not padding, and a plan that omits it is a plan that produces the same total with worse quality and a demoralizing final third. The 616 day total **already includes** this overhead inside the per item estimates.

---

## 15. Milestone Summary

| Milestone | Option A day | Option B day | Meaning |
|---|---:|---:|---|
| Determinism floor complete | 24 | 24 | Every later line of code is written against the kernel |
| Phase 0 complete | 72 | ~250 (interleaved) | Full verifiability infrastructure |
| **First sellable artifact** | **128** | **80** | Crypto MVP installable and chargeable |
| Equity and filings live | 182 | 134 | Second asset class, symbology proven |
| **Options wedge live** | **244** | **196** | The differentiating product |
| Backtesting live | 322 | 274 | Quantitative platform begins |
| Calibrated forecasting live | 383 | 335 | Invariant I2 fully realized |
| Paper trading live | 445 | 397 | Complete research to decision loop |
| Live execution enabled | 491 | 443 | Full retail product |
| Enterprise ready | 575 | 527 | Second business |
| Ecosystem open | 616 | 568 | Third business |

---

## 16. Gate Review Procedure

Every phase closes with a written review, not a feeling. The review is a document in `docs/gates/phase-N.md` containing:

1. Each exit criterion, its verification artifact, and pass or fail.
2. Every waiver, with the reason, the risk accepted, and the phase by which it must be resolved. A waiver without a resolution phase is not a waiver, it is a silent scope cut.
3. Estimate versus actual per work item, which is the only way the estimates in this document improve.
4. Items deferred to a later phase with their original identifiers.
5. New risks discovered, added to the register in architecture §33.
6. A go or no go decision on the next phase, with the reasoning recorded.

**Recalibrate after Phase 0.** The estimates here are anchored on the author's judgment, not on measured throughput. Phase 0 produces the first real velocity data. Multiply every subsequent estimate by the Phase 0 actual over estimate ratio and update this document. A plan that is never recalibrated is a plan that is wrong for progressively longer.

---

## 17. Explicitly Not Now

Recording what is deliberately excluded is as valuable as recording what is included, because it prevents the same debate recurring quarterly.

| Item | Source | Not now because | Revisit |
|---|---|---|---|
| Causal market graph | v0.1 §37.1 | Research project, not product work | Post Phase 9 |
| Market digital twin | v0.1 §37.2 | Depends on microstructure data PRISMATIK does not have | Post Phase 9 |
| Federated strategy learning | v0.1 §37.3 | Requires multiple enterprise customers | Post Phase 8, demand driven |
| Confidential computing | v0.1 §37.4 | No customer has asked | Demand driven |
| Multi agent research council | v0.1 §37.7 | The single AI analyst is not yet proven | Post Phase 6 |
| Pine Script compatibility | v0.4 §12, Phase 0.4I | Provenance and licensing risk exceeds the value | Post Phase 9, if ever |
| TradingView Advanced Charts | v0.4 §8 | Separately licensed, not a foundation | Only if a customer requires it |
| NautilusTrader adoption | v0.4 §28 | LGPL implications unresolved and it owns owned boundaries | Reference only, permanently |
| Temporal orchestration | v0.2 §7.1 | In process task graph is sufficient below Phase 8 | Phase 8, ADR-0036 |
| Additional TSFM families | §6.7 | Three artifacts prove the registry; more is dilution | Phase 8 |
| Mobile client | not in corpus | No coherent mobile use case for a dense analytical workspace | Demand driven |

---

**End of PRISMATIK Phased Implementation Plan v1.0**

*Author: Aaron Stovall · Mythos Systems · 2026-07-24 · Version 1.0.0*
