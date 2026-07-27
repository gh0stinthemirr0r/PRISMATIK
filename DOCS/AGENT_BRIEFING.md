# PRISMATIK — Agent Briefing & Information Convergence Vision

**Audience:** an autonomous agent picking up the "Comprehensive Gap Audit, Refinement & Innovation" directive
**Date:** 2026-07-27
**Branch:** `feat/wave0-floor-continue`
**Status:** Part I is operational and binding. Part II is a proposal — nothing in it is scheduled, costed, or approved.

---

# PART I — OPERATIONAL BRIEFING

## 0. Read This Before You Touch Anything

You have been handed a directive written in the idiom of a mature web application: unhandled promise
rejections, redundant re-renders, glassmorphism filters, responsive viewport glitches.

**PRISMATIK is not that.** It is a 36-crate Rust workspace with a thin desktop shell. It is roughly
95% backend. If you execute the directive literally, you will spend your budget polishing surfaces
that do not exist while the workspace does not compile.

This document tells you what the directive should have said.

### Ground truth, verified 2026-07-27

| Gate | Result |
|---|---|
| `cargo test --workspace --lib` | **106 passing, 0 failing** |
| `cargo clippy --workspace --lib -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `cargo test --workspace` (all targets) | **RED** — 3 targets fail |

The three failing targets are `prismatik-audit` bench `audit_append`, `prismatik-identity` test
`identity_corpus`, and `prismatik-manifest` example `gen_goldens`.

### What the codebase actually is

```
crates/            36 Rust crates, strict 4-layer dependency order
  Layer 0          determinism, identity            (no workspace deps)
  Layer 1          calendar, audit, manifest, storage
  Layer 2          domain crates (market-data, features, strategy, risk, …)
  Layer 3          plugin-host, ai-tools, security
  Layer 4          application, cli, observability, renderer   (application is the ONLY
                                                                crate allowed to touch storage)
apps/desktop/      Tauri 2 + SvelteKit scaffold — early, thin
packages/          api-client, chart-contracts, design-tokens, ui — early
legacy-v0.1/       working SvelteKit dashboard + Axum server — DEPRECATED
services/          research sidecars (Python, isolated, untrusted-output)
DOCS/              the real specification. 8 wave plans, 13 spec docs, ADRs, gate records
```

The authoritative document is `PRISMATIK_Unified_Solution_Architecture_v1.0.md`. Read §2 (invariants),
§12 (determinism kernel), and §15 (data plane) before writing code.

---

## 1. Phase 0 — Blocking Work, Do This First

**Read `DOCS/ORPHANED_MODULE_REMEDIATION.md` and execute it.**

43 modules across 15 crates exist as `.rs` files that are never declared with `mod` in their crate's
`lib.rs`. Rust does not compile a file unreachable from the crate root, so this code is dead: never
compiled, never linted, never tested, invisible to CI.

You cannot audit, refactor, type-harden, or optimize code the compiler has never seen. **Every phase
of the directive is blocked on this.**

That document classifies all 43 by root cause, lists the exact missing symbol per crate, gives
dependency ordering, and provides a CI detector to prevent recurrence. It is self-contained.

Two rules from it that bear repeating:

- **Do not delete an orphaned module to make the build pass.** The orphaned module is the
  specification; the landed base type is what is behind. Extend the base type. Deleting silently
  discards real work.
- **Do not relax `#![warn(missing_docs, missing_debug_implementations)]` or `#![forbid(unsafe_code)]`.**
  If newly-wired public items lack docs, write the docs.

---

## 2. The Invariants Outrank the Directive

PRISMATIK has seven invariants (architecture §2), each with a named CI gate. They are not
aspirations; they are the product. The platform's entire commercial claim is:

> Data breadth is commodity. Chart quality is commodity. Models are commodity and given away by
> Amazon and Google. What cannot be bought is a platform where a backtest run in July 2026 can be
> re-executed byte-for-byte in July 2029, on a different machine, producing an identical signed
> manifest, with every input traceable to a provider endpoint and a retrieval timestamp.

| | Invariant | One-line meaning |
|---|---|---|
| I1 | Evidence precedes conclusion | No rendered conclusion without a resolvable evidence chain to raw records |
| I2 | Probability, never prophecy | Model output crosses IPC only with an uncertainty band and calibration record |
| I3 | Determinism under seed | Same inputs + seed + pinned artifacts → byte-identical output, any machine, any date |
| I4 | Structural capability enforcement | Runtime capability tokens, not documentation or code review |
| I5 | Point-in-time correctness | No input with `event_time` after the simulated clock — **including information laundered through a pretrained model's corpus** |
| I6 | Fail closed on staleness | Automation halts rather than degrades. Staleness is a hard deny |
| I7 | Tamper-evident audit | Append-only, hash-chained, Merkle-verifiable ledger |

### Directive items that violate these

Each of the following sounds reasonable and would break a core guarantee. **When the directive and
an invariant conflict, the invariant wins and you surface the conflict — you do not silently
resolve it.**

| Directive item | Violates | Why, and what to do instead |
|---|---|---|
| "Implement aggressive data caching" | **I5, I6** | A cache keyed on wall-clock time silently serves data the simulated clock must not see, and hides staleness. Cache keys must include `DeterminismContext` + `as_of` + artifact set. Staleness must hard-deny, never serve stale. |
| "State history and rollback capabilities" | **I7** | The audit ledger is append-only and hash-chained; rollback is the exact thing it exists to prevent. Use compensating entries. "Rollback" of *user workspace state* is fine — of the ledger, never. |
| "Automated anomaly detection" | **I2** | No detector output reaches the UI without a `CalibrationRecord`. There is deliberately no public constructor that omits one. Build the detector; wire it through `prismatik-calibration`. |
| "Intelligent defaults based on user patterns" | **I3** | Ambient, drifting, non-reproducible state. In a trading platform a default that silently moved is a liability event. If built: defaults must be explicit, versioned, diffable, and recorded in the manifest. |
| "1-click sample data loaders" (Phase 4) | **I1** | Synthetic data inside a platform whose entire claim is provenance. If built, sample data must be indelibly tagged synthetic and structurally barred from evidence chains and manifests. A demo dataset that can be mistaken for real is a defect. |
| "Export CSV / JSON / PDF summaries" | **I1, I2** | An export carrying a conclusion must carry its evidence chain and calibration disclosures. An export that drops them reintroduces exactly the dishonesty the platform exists to prevent. Design the export schema around the disclosure, not as an afterthought. |
| "Connect external data feeds, webhook pipelines" | §16 provider plane | Everything goes through the `Provider` port and the `BudgetGovernor` (GCRA rate limiting via `governor`). No `reqwest` in a domain crate. Sidecars cannot write to the data plane, hold a broker credential, or emit an audit event. |
| "Real-time data correlation" | I1, I2, I3 | Correlation is easy to compute and easy to compute dishonestly. See `DOCS/PRISMATIK_Cross_Market_Convergence_Engine_Proposal_v0.1.md` — it specs this with effective sample size, FDR control, and regime conditioning already worked out. Do not reinvent it naively. |

---

## 3. Scope Corrections to the Directive

### Retarget Phase 1.1 (code health)

"Missing null checks, uncaught exceptions, memory leaks, unhandled promise rejections" mostly do not
apply to safe Rust. The real equivalents in this codebase:

- `unwrap()` / `expect()` / `panic!` on fallible paths — especially anything reachable from an order
  intent or a provider response
- Blocking calls inside `async fn` (file I/O, `std::sync::Mutex` held across `.await`)
- Unbounded channels and unbounded retry loops
- **Non-deterministic iteration order.** `HashMap`/`HashSet` iteration is a determinism leak. The
  workspace provides `DetMap` for exactly this. Audit for raw `HashMap` in any path that feeds a
  manifest, hash, or ordered output.
- **Ambient time and entropy.** Any `SystemTime::now()`, `Instant::now()`, `rand::thread_rng()`
  outside `prismatik-determinism` is a hard error, not a warning. There is an `ALLOWLIST.txt` and a
  `grep_gate` test — extend its coverage rather than adding exceptions.
- Float non-determinism: parallel reductions under Rayon are not associative and produce
  run-to-run bitwise differences. Any parallel numeric reduction needs fixed chunking and a
  deterministic merge order.
- `cargo clippy --workspace --all-targets -- -D warnings` is already a CI gate. The `--all-targets`
  part is where the failures are.

### Mostly drop Phase 1.2 / 1.3 for now — and the reason matters

There is not yet enough frontend to audit. `apps/desktop` is a scaffold. Auditing "responsive layout
glitches across all viewport sizes" on a scaffold produces work that will be thrown away.

**Keep from these sections** (they are real and valuable, scoped to `apps/desktop` and
`packages/design-tokens`): design-token adherence, focus states and keyboard navigation, a `Cmd+K`
command palette, screen-reader labels, contrast ratios, empty states, contextual onboarding.

**A design principle specific to this product.** Beauty here is not decoration — it is the
consequence of showing uncertainty honestly. Every number carries its interval. Every conclusion
carries its evidence chain. Every regime-conditional statement carries its regime. Every surface has
a visible "what would change my mind" affordance. A UI that renders a confident number with no error
bar is not a prettier UI, it is a false one. Honest uncertainty has *shape*, and shape is what
renders well.

---

## 4. Traps

**`walkthrough.md` records only successes.** It contains a "all gates passed" entry that was not true
of the committed tree — the untracked work it described included test files referencing APIs that
never landed. If Phase 5 tells you to document there, record failures too. Treat existing entries as
claims, not verification.

**Beware pipeline exit codes.** `cargo test --workspace 2>&1 | tail -40` returns `tail`'s exit code,
not cargo's. This masked a real compile failure in this repo. Redirect to a file and check `$?`.

**`legacy-v0.1/` is deprecated but still builds and still gets edited.** Do not invest in it without
an explicit decision. Do not port its patterns into the new core — its server has its own auth model
that predates the security architecture.

**Generated artifacts.** `cargo-cyclonedx` writes one `*.cdx.json` per member crate; 35 of them were
accidentally committed. `scripts/generate-sbom.{sh,ps1}` now sweep them into `artifacts/sbom/` and
`*.cdx.json` is gitignored. Do not re-add them.

**New third-party dependencies are a supply-chain decision.** The repo has `deny.toml`,
`supply-chain/audits.toml`, and a cargo-vet gate. `prismatik-storage` is currently blocked because it
needs `arrow` and `parquet`, which are not in the workspace. Do not add large dependency trees
unilaterally — flag them.

---

## 5. Suggested Resequencing

```
Phase 0   Orphan remediation                             BLOCKING — nothing else works first
Phase 1   Rust health audit + invariant gate coverage    determinism leaks, ambient time,
                                                          HashMap ordering, unwrap on fallible paths
Phase 2   Backend optimization under I3/I5/I6 constraints
Phase 3   Frontend / UX  ← REQUIRES A DECISION, see §6
Phase 4   Innovation — cross-reference the two proposal docs rather than inventing in parallel
Phase 5   Verification + honest documentation
```

## 6. Decisions Needed From the Owner Before Starting

An agent will guess at these and guessing wrong wastes an entire phase.

1. **Frontend target.** `apps/desktop` (greenfield Tauri 2 + SvelteKit) or reviving
   `legacy-v0.1/UI`? The directive assumes a mature UI that does not exist.
2. **`arrow` + `parquet` dependency approval** — blocks `prismatik-storage` and transitively
   `prismatik-application`.
3. **Scope of Part II below.** It is a large proposal. Which, if any, of it is in scope?
4. **Data licensing and regulatory posture** for anything in Part II — see §22. These are not
   engineering decisions and must not be made by an agent.

---

# PART II — THE GLOBAL INFORMATION CONVERGENCE & PREDICTION ENGINE

*Proposal. Not accepted, not scheduled, not costed. Confidence markers: **H** well-understood
engineering, **M** non-trivial but understood, **L** research-grade — prototype in a sidecar before
it earns a crate.*

## 7. Thesis

> **Everyone has the same news. Nobody has an honest record of what the news was worth.**

There is no alpha in knowing that a headline exists — it is on every terminal simultaneously. The
alpha is in six things almost nobody instruments:

1. **Novelty** — is this genuinely new information, or the forty-seventh restatement of a known fact?
2. **Diffusion** — how fast does it travel from origin to mass awareness, and through what topology?
3. **Lineage** — who reported it first, and who is merely echoing?
4. **Dispersion** — how much do sources *disagree* about what it means?
5. **Silence** — what is conspicuously *not* being said, by whom, right now?
6. **Track record** — when this source, this analyst, this firm said this before, what happened?

Every one of those is a *point-in-time* question. Every one is answerable only by a platform that can
prove what it knew and when. PRISMATIK is architecturally that platform and almost nothing else is.

## 8. The Unlock: Honest LLM Backtesting

**This is the single most defensible idea in this document.**

Every "AI news trading" product on the market is built on a foundation that quietly invalidates it:
they score historical news with a model whose pretraining corpus already contains the outcome. An
LLM asked in 2026 to judge the sentiment of a March 2020 headline *knows what happened next*. The
backtest is not optimistic, it is meaningless.

Invariant I5 names this explicitly — *"including information laundered through a pretrained model's
corpus."* The architecture already specifies `PretrainingRecord` and a contamination gate that hard-
denies a model whose pretraining cutoff overlaps the test window, and emits a security event on
denial (`P5-QM-06`).

**Consequence: PRISMATIK can honestly backtest news-driven strategies and its competitors
structurally cannot.** That is not a feature comparison. That is a claim nobody else can make,
enforced by a type system and a CI gate rather than by a promise.

Everything else in Part II is a delivery vehicle for that claim.

Additional discipline required, beyond the existing gate:

- **Point-in-time embeddings.** You cannot embed a 2019 article with a 2026 embedding model and
  compare it to a 2019 corpus. Embedding model + tokenizer codebook are already pinned, content-
  addressed artifacts (§12.5). Extend that to a per-vintage embedding index in LanceDB, whose
  dataset versioning makes the pin real rather than aspirational.
- **Archive mutation is real.** News outlets silently edit headlines and bodies after publication.
  Store raw verbatim payloads with `retrieved_at`, never rewrite, and create a *new* record with a
  supersession link on change. This is already the Raw-layer rule (§15.1); news makes it load-bearing.
- **Publication time ≠ observability time.** See §10.

## 9. Proposed Crate Topology

Four new Layer 2 domain crates, plus reuse. Follows existing rules: ports only, no storage backends,
no vendor SDKs, no HTTP clients in domain crates.

```
prismatik-narrative      NEW   Document identity, story threading, novelty, lineage,
                               diffusion topology, silence detection.
  deps: prismatik-domain, prismatik-identity, prismatik-features, prismatik-determinism

prismatik-consensus      NEW   Analyst/firm/institution call ledger, track-record scoring,
                               per-source calibration curves, disagreement metrics.
  deps: prismatik-domain, prismatik-calibration, prismatik-audit

prismatik-lattice        NEW   Cross-venue + information temporal alignment. Shared with the
                               Cross-Market Convergence proposal — build it once.
  deps: prismatik-domain, prismatik-calendar, prismatik-determinism

prismatik-hypothesis     NEW   The prediction engine proper: falsifiable statement types,
                               baseline ladder, adversarial falsification, prediction ledger.
  deps: prismatik-calibration, prismatik-audit, prismatik-manifest, prismatik-narrative,
        prismatik-consensus
```

Existing crates gaining surface, not new crates: `prismatik-analog-store` (news/filing embeddings as
a modality — the `evidence_index` table is already specified for this), `prismatik-features` (new
views, §20), `prismatik-risk` (new pre-trade checks, §19), `prismatik-ai-router` (already implements
capability-routed providers with recorded effects — this is the LLM boundary, use it),
`prismatik-calibration` (per-source Mondrian categories).

Sidecars (Python, isolated, untrusted output): translation, NER/entity linking, embedding
generation, topic modelling, LLM inference. Rust owns the contracts and the online path.

## 10. The Information Arrival Lattice

**Confidence: H. Foundation. Build first.**

Every information event carries four timestamps, and conflating any two of them is a look-ahead bug:

| Timestamp | Meaning |
|---|---|
| `event_time` | when the thing described actually happened |
| `publication_time` | when a source first published it |
| `observation_time` | when *we* retrieved it (`retrieved_at`) |
| `diffusion_time` | when it reached mass awareness — modelled, with uncertainty |

The feature store's `observation_delay` field then does real work. Some concrete, frequently-wrong
delays:

| Source | Delay | Trap |
|---|---|---|
| Newswire | seconds | But *your* ingestion latency is not zero. Measure it; encode it. |
| SEC EDGAR filing | minutes to hours after acceptance | Acceptance timestamp ≠ dissemination timestamp |
| 13F holdings | **45 days** | Observable at *filing* date, never at period end. Wave 2 DoD #4. |
| Form 4 insider | 2 business days | |
| CFTC COT | 3 days | Tuesday data, Friday publication |
| FRED macro | series-specific + **vintage** | Must use the initial print, not the revised value |
| Sell-side research | hours to days | Often circulated to clients before public availability. If you only have the public version, your delay is wrong and unknowable — say so. |
| On-chain metrics | **chain finality depth** | A metric with delay 0 consumes blocks that may be orphaned. This is a look-ahead bug that only manifests in backtests spanning a reorg. Almost nobody handles it. |
| Social / Telegram / Discord | seconds, but | Deletion and edit are routine. Verbatim capture with `retrieved_at` is mandatory. |

**Gate:** property test — for a random view, entity, and instant, assert no returned value has
`event_time + observation_delay > as_of`. 10k cases.

## 11. The Narrative Graph — `prismatik-narrative`

### 11.1 Story threading (M)

Articles are not the unit of analysis; **stories** are. Cluster documents into threads by entity +
event type + time window, then track lifecycle:

```
emergence → amplification → saturation → decay → (dormancy | revival)
```

Price reaction differs enormously by phase. Trading emergence and trading saturation are *opposite*
trades, and a system that treats "positive news" as one signal is averaging them into nothing.

Each thread carries: first-observation timestamp, participating sources in order, entity set,
cumulative attention, and phase posterior. Phase is a *distribution*, never a hard label — an
`Indeterminate` phase is first-class and should be used freely. The existing flow-classification
doctrine applies verbatim: *a classifier that always decides is a classifier that is often wrong.*

### 11.2 Novelty scoring (M)

Semantic distance from everything already in the corpus **as of that timestamp**. A story that is 95%
recycled has no information content regardless of how many outlets carry it.

This is only computable with point-in-time embedding indices (§8). It is the single clearest example
of a feature that PRISMATIK can compute honestly and a competitor cannot compute at all.

### 11.3 Source lineage & diffusion topology (M)

Build a directed graph of who-reported-what-when. Derive:

- **Origination score** per source — how often first, weighted by subsequent confirmation
- **Echo ratio** — how much of a source's output is derivative
- **Diffusion topology** — the *shape* of the spread

That last one has a use nobody expects. Organic news diffuses through a scale-free network with a
characteristic heavy-tailed cascade. **Coordinated inauthentic amplification diffuses through a
suspiciously regular topology.** In crypto especially — pump campaigns, wash-narrative promotion,
paid placement — the topology signature is detectable before the price signature. This is both a
tradable signal and a user-protection feature.

### 11.4 The Silence Detector (L — and the most interesting idea here)

**Anomalous absence.** Nobody instruments this because absence is hard to represent — you cannot
embed a thing that was not said.

The construction: model each entity's *expected* information arrival rate conditional on its history
and its peer group, then flag significant negative deviations.

- A company that has pre-announced every quarter for six years and has not this quarter
- An analyst who publishes on a name every earnings cycle and has gone quiet
- A protocol whose dev channel volume has collapsed
- A regulator that responds to comparable events within N days and has not
- Sell-side silence in the window before downgrades

Absence of news is a signal with a genuinely different failure mode from presence of news, which is
exactly what makes it valuable in an ensemble. Treat expected-arrival-rate as a calibrated model in
its own right, subject to the same baseline ladder as everything else.

### 11.5 Cross-lingual arbitrage (M)

News breaks in Mandarin, Japanese, Korean, and German hours before it reaches English wires. Crypto
news breaks on Telegram, Discord, and X before it reaches anywhere. **Time-to-English-translation is
measurable, decaying alpha**, and non-English source coverage is a cheap and durable moat because
most competitors simply do not do it.

Instrument: per-language first-observation timestamps, translation latency distribution per
source-pair, and a "language lead time" feature per entity. Translation runs in a sidecar; the
original text is the raw record and the translation is a derived artifact with its own model
provenance — never overwrite the original.

## 12. The Consensus Cartography — `prismatik-consensus`

### 12.1 Disagreement, not sentiment (M)

Sentiment scores are close to worthless — they are a lossy projection of a rich object onto one
axis, and everyone computes the same one. **Dispersion of framing across sources is the signal.**

When Reuters, Bloomberg, a Chinese state outlet, a European business daily, and crypto-native media
describe the same event with maximally divergent framing, that divergence is itself the datum.
Geopolitical stress appears as framing divergence *before* it appears in price. Compute the
dispersion of the embedding cloud for a single story thread, not its centroid.

### 12.2 The Analyst Accountability Ledger (M — highest product value in Part II)

**Record every published call from every firm, analyst, and institution with its horizon. Then score
it against realization. Then publish the calibration curve.**

```
Firm X, 12-month equity price targets, last 5 years:
  n = 1,247 calls
  realized within stated target ± band:  31%
  directional accuracy:                  54%   (coin flip: 50%)
  median absolute error:                 22%
  conditional on high-volatility regime: 19%   ← the number that matters
```

This is exactly the Mondrian conformal machinery already committed to in the architecture (§17.3),
pointed at a new category dimension: **per-source, per-regime calibration**. The infrastructure
exists. Nothing new needs to be invented — it needs to be aimed.

Consequences:

- Sell-side research stops being noise and becomes a weighted input with a measurable prior.
- An analyst with a genuinely good record on a specific sector in a specific regime becomes
  discoverable — and that is a real, rare, monetizable finding.
- The consensus estimate can be *re-weighted by historical calibration* instead of equal-weighted.
  A calibration-weighted consensus is a strictly better prior than the published consensus, and it
  is computable from public data.
- It creates an accountability surface the industry does not currently have.

**Handle with care.** Publishing accuracy scores about named individuals and firms has defamation,
contract, and data-licensing exposure. See §22. The engineering is straightforward; the deployment
posture is a decision for counsel, not for an agent.

### 12.3 Institutional positioning as ground truth

News says X; the filing says Y. 13F, Form 4, 13D/G, and COT are *what institutions actually did*,
lagged but factual. Divergence between narrative and filed fact is a high-signal event class:
narrative bullish + insiders selling is a different world from narrative bullish + insiders buying.

This is Wave 2 work that already exists in the plan (`P2-EX-01`); the contribution here is joining it
to the narrative graph.

## 13. Source Value = Information, Not Volume

**Confidence: L. Elegant, and it converges two subsystems that have no obvious reason to meet.**

For each source, measure its **marginal contribution to predictive information, conditional on all
other sources** — transfer entropy to the price process, or incremental log-loss reduction in the
prediction engine.

Two consequences:

1. Sources that add nothing get automatically down-weighted. The provider set self-prunes.
2. **It directly justifies data spend.** A feed costing $40k/year that adds 0.3% conditional
   information reduction is a line item with a number attached to it.

That second point is the convergence: this makes signal value a first-class input to the
`BudgetGovernor` (§16.2). Cost governance and alpha measurement become the same subsystem. No
product I am aware of does this, and it is the kind of thing a CFO understands immediately.

## 14. The Prediction Engine — `prismatik-hypothesis`

This is where most products become dishonest. The discipline below is what prevents it.

### 14.1 Hypotheses, not scores

The engine does not emit "bullish 0.72." It emits **falsifiable statements with horizons, intervals,
and explicit falsification conditions**:

```rust
/// No public constructor omits the disclosure fields. Same enforcement pattern
/// as ModelPrediction / CalibrationRecord under I2 — the type system carries
/// the discipline, not the reviewer.
pub struct Hypothesis {
    pub statement: FalsifiableStatement,   // entity, direction, magnitude, horizon
    pub interval: ConfidenceInterval,      // never a scalar
    pub calibration: CalibrationRecord,    // realized coverage, per regime
    pub regime: RegimeLabel,               // conditional on state, always
    pub evidence: Vec<EvidenceRef>,        // resolves to raw records + retrieval timestamps
    pub falsifiers: Vec<FalsificationCondition>,   // what would kill this, stated up front
    pub baselines_beaten: BaselineLadderReport,    // §14.2
    pub novelty: NoveltyScore,             // is this conclusion itself recycled?
    pub dissent: Vec<DissentRecord>,       // §14.3
    pub pretraining_record: PretrainingRecord,     // §8 contamination gate
}
```

### 14.2 The baseline ladder is not optional

Already normative in the architecture (§17.2). Applied here: a news-driven model must beat, on
**both discrimination and calibration**, before it is promoted:

1. No-news baseline (price/vol only)
2. Momentum-only
3. Naive keyword count
4. Published analyst consensus, unweighted
5. Calibration-weighted consensus (§12.2)

Most "AI news alpha" does not beat rung 2. Making the ladder a promotion gate rather than a report is
what keeps the platform honest when a demo looks impressive.

### 14.3 Adversarial falsification and the correlated-error trap

The architecture already records the **AnalystScope correction**: shared discovery *worsens*
correlated error. Five agents agreeing after reading the same news summary is one opinion reported
five times. The fix is a hard **common-facts / private-evidence split** — the technical agent must
not see the news the sentiment agent sees. Only then is agreement genuine signal.

Add to that a dedicated **refutation agent** per hypothesis whose only job is to kill it, prompted to
default to "refuted" under uncertainty. Survivors carry higher confidence; the refutation attempt is
stored as a `DissentRecord` and rendered alongside the conclusion. **Showing the strongest case
against your own conclusion is the most trust-building thing a prediction surface can do**, and
almost no product does it.

### 14.4 The Prediction Ledger — the idea I would build the marketing on

**Every prediction is written to the append-only, hash-chained, Merkle-verifiable audit ledger at
issuance, before resolution is knowable.**

The consequence is not subtle: **the platform is cryptographically incapable of lying about its
track record.** Anyone can verify, with the standalone verifier and no PRISMATIK installation, that
a prediction existed at the claimed time and has not been edited since. Deleted predictions are
detectable as gaps in the chain. Retroactive edits are detectable without trusting the storage layer.

Publish the tree head periodically — the `public_verify` path already exists in `prismatik-cli`.

Every trading product in the world advertises its track record and every one of them is asking to be
taken at its word. This is a trading platform whose track record is *verifiable by an adversary*.
That is the strongest asset in this document and it is nearly free, because I7 and
`prismatik-manifest` already exist. It needs to be aimed, not invented.

### 14.5 Reflexivity and crowding

Narrative predictive power **inverts** at saturation. A trade that is fully consensus has already
been placed by everyone who was going to place it.

Intersect narrative saturation (§11.1) with positioning data (COT, perp funding rates, put/call
skew, ETF flows, dealer gamma). The intersection is a **crowded-trade detector** — high narrative
saturation plus extreme positioning is the classic setup for a violent unwind that has nothing to do
with fundamentals.

This is the feedback loop that makes the engine self-aware about its own signal degradation, which is
a property most systems lack entirely.

### 14.6 Automatic event studies

For any recognized event class, automatically run the historical study and return a *distribution*,
never an anecdote:

> *"The last 47 times a Fed governor used this phrasing within 48h of a CPI print, 5-day forward
> NASDAQ returns were: median −0.4%, IQR [−2.1%, +1.3%], n=47, and dropping the 3 nearest analogs
> flips the median sign."*

That last clause is the `leave_n_out_sensitivity` field, already mandatory and non-optional in
`AnalogResult`. Applied to event studies it answers the question that kills most such analysis:
*is this conclusion carried entirely by March 2020?*

## 15. Multi-Modal Fusion — the Market State Tensor

One object per lattice instant, spanning both markets and all modalities:

```
   assets  ×  modality  ×  horizon
              ├─ price / return
              ├─ realized & implied volatility
              ├─ flow            (options prints, exchange netflow, ETF create/redeem)
              ├─ positioning     (COT, funding, 13F, dealer gamma, insider)
              ├─ liquidity       (spread, depth, stablecoin net issuance)
              ├─ narrative       (thread phase, novelty, dispersion, silence)   ← Part II
              ├─ consensus       (calibration-weighted analyst prior)           ← Part II
              └─ macro           (FRED, vintage-correct)
```

Every slice carries its own `observation_delay`, so the tensor is point-in-time correct by
construction rather than by author discipline. This is the existing feature-store rule applied at
tensor scale, and it is the only reason a structure this heterogeneous is safe to build.

Detail in `DOCS/PRISMATIK_Cross_Market_Convergence_Engine_Proposal_v0.1.md` §9.1, which also covers
the crypto↔equity clock problem, Hayashi–Yoshida covariance for asynchronous pairs, FDR control
across correlation matrices, and the bridge instruments (spot ETFs, COIN/MSTR, CME basis) that
transmit between the two markets. **Read that document before building anything in this section —
the two proposals share `prismatik-lattice` and should not be built twice.**

## 16. Product Surfaces

Named, so they can be argued about. The `prismatik-renderer` wgpu layer already exists for this class
of visual, with mandatory Canvas/CPU fallback.

| Surface | What it is |
|---|---|
| **The Newsroom** | Story threads as living objects, not a feed. Each thread shows phase, novelty, source lineage as a diffusion tree, and dispersion. You watch a story *form*. |
| **The Ledger** | Public prediction track record. Every past prediction, its interval, its resolution, the running coverage curve. Verifiable by an adversary. This is the trust surface. |
| **The Silence Board** | Entities whose expected information arrival has anomalously stopped, ranked by deviation. Nothing else on the market shows this. |
| **Consensus Cartography** | Analysts and firms positioned by historical calibration, not by AUM or brand. Reweights visibly as records update. |
| **The Attribution Ribbon** | Today's move decomposed into named factors with the **unexplained residual rendered at equal weight**. A factor model that always explains everything has too many factors. |
| **Contagion Replay** | Current cross-market + narrative state → nearest historical analogs → forward path distribution, with the leave-N-out sensitivity as a **slider the user can drag**. Watching your conclusion evaporate as you drag it is the most honest interaction in the product. |
| **The Night Watch** | 09:29 ET. Overnight crypto path + overnight global news flow → conformally calibrated equity open-gap interval, with the realized coverage curve beside it. The product grades itself in public every weekday. |
| **Market Cartogram** | Force-directed correlation-metric embedding of both markets; MST skeleton; edges by *partial* correlation; arrows by information flow. Scrub time and watch clusters collapse into one knot during stress. |

## 17. Onboarding — Directive Phase 4, Done Right

The directive's onboarding asks are good and I would keep them nearly as written, with three
product-specific additions:

- **Teach the epistemics, not the buttons.** The first-run experience should teach a user to read a
  coverage curve. If they leave onboarding believing "90% confidence" means "90% sure," the product
  has failed at the only thing that makes it different.
- **The Getting Started checklist should be provenance-shaped**: connect a provider → verify your
  first manifest → run the standalone verifier against it. A user who has personally verified one
  manifest understands the product. That is the activation moment, not "you made a chart."
- **Empty states are teaching surfaces.** An empty Prediction Ledger should say *"no predictions yet
  — and when there are, you'll be able to verify every one of them independently, including the
  wrong ones."*

## 18. Risk Integration

New pre-trade checks for the §19.1 catalog. The engine must feed the risk plane, not sit beside it.

| Check | Severity | Rule |
|---|---|---|
| `narrative_crowding` | SoftWarn | Intent aligns with a narrative at saturation phase + extreme positioning (§14.5) |
| `unverified_claim` | **HardDeny** | Intent cites a hypothesis whose evidence chain contains an unresolvable `EvidenceRef` |
| `source_contamination` | **HardDeny** | Intent cites a model whose pretraining cutoff overlaps the evidence window (I5) |
| `hypothesis_stale` | **HardDeny** | Cited hypothesis was issued under a regime since superseded — I6 applied to *structure*, not ticks |
| `silence_anomaly` | SoftWarn | Position entity is currently flagged by the Silence Detector |

`hypothesis_stale` is the sharp one. I6 currently governs *data* staleness. Structural staleness is
different and worse: every input can be milliseconds old while the relationship the position depends
on stopped holding last Tuesday.

## 19. Feature Views to Add

Non-exhaustive; every row is a look-ahead bug if the delay is wrong. See §10 for the delay table.
Each needs a `FeatureView` with an explicit `observation_delay`, a `lineage_transform`, and coverage
in the point-in-time property test.

`news_thread_phase`, `news_novelty`, `source_dispersion`, `silence_deviation`,
`language_lead_time`, `analyst_calibrated_consensus`, `analyst_dispersion`,
`filing_narrative_divergence`, `diffusion_topology_anomaly`, `narrative_saturation`.

## 20. Sequencing

| Stage | Content | Wave | Conf |
|---|---|---|:---:|
| 1 | `prismatik-lattice` — shared with the convergence proposal. Build once. | 1.5 | H |
| 2 | Raw news ingestion behind the `Provider` port; verbatim capture; supersession links | 2A | H |
| 3 | Point-in-time embedding index in LanceDB; pinned per-vintage embedding artifacts | 2A | M |
| 4 | `prismatik-narrative`: threading, novelty, lineage | 2B | M |
| 5 | `prismatik-consensus`: analyst call ledger + track-record scoring | 3 | M |
| 6 | Per-source Mondrian calibration wired into `prismatik-calibration` | 3B | M |
| 7 | `prismatik-hypothesis`: statement types, baseline ladder, **prediction ledger** | 4 | M |
| 8 | Adversarial falsification + common-facts/private-evidence split | 4 | M |
| 9 | Silence detector | 4 | L |
| 10 | Diffusion topology / inauthentic amplification detection | 5 | L |
| 11 | Source value = transfer entropy → BudgetGovernor convergence | 5 | L |

**Stages 1–3 are worth building even if nothing after them ships.** They are the honest-backtesting
foundation (§8), and without them every later stage is decoration on an invalid claim.

**If only one thing from Part II is built, build the Prediction Ledger (§14.4).** It is small, it
reuses `prismatik-audit` and `prismatik-manifest` almost entirely, and it is the single most
defensible claim available to this product.

## 21. Hard Constraints — Not Engineering Decisions

**An agent must not decide any of these.** Escalate.

| Constraint | Reality |
|---|---|
| **News content licensing** | Reuters, Bloomberg, Dow Jones terms prohibit most redistribution and derivative display. Storing verbatim raw payloads — which I5 *requires* — may conflict with a provider's terms. Resolve licensing **before** ingestion architecture, not during. |
| **Copyright** | Summaries must be substantially shorter than and different from the source. Never reconstruct an article from stored excerpts across surfaces. |
| **Publishing accuracy scores about named firms and individuals** (§12.2) | Defamation and contract exposure. Factual, sourced, methodologically disclosed scoring is materially safer than editorializing, but this needs counsel before it ships publicly. Internal use is a different risk profile from published use. |
| **Investment adviser regulation** | A system emitting personalized, actionable predictions may constitute investment advice in multiple jurisdictions. This is a licensing question with criminal exposure in some regimes. Not an engineering decision. |
| **Market manipulation** | A platform that publishes predictions *and* has users trading on them has a reflexivity problem that is also a regulatory one. |
| **Cost** | Global multi-language news, options flow, and analyst research are materially expensive and recurring. §13 exists partly to make that spend defensible; model it before Wave 2 opens. |
| **Social/chat data** | Telegram, Discord, X terms and privacy law. Personal data in market context has GDPR implications. |

## 22. What I Would Cut First, and Honest Risks

**Cut order:** diffusion topology / inauthentic amplification detection (L — fascinating, hard to
validate, easy to fool yourself with). Then transfer-entropy source valuation (L — estimator is
data-hungry and noisy on daily data). Then the silence detector (L — the expected-arrival-rate model
is the whole game and it may not be estimable well enough to be actionable).

| Risk | Reality |
|---|---|
| Most of §11–13 is research-grade | Prototype in a sidecar. Do not give an estimator a Rust crate until it has survived out-of-sample validation on real data. |
| News→price relationships are **non-stationary at the meta level** | The relationship in 2021 (retail liquidity, meme dynamics) is not the one in 2026. Long backtests of this engine are structurally suspect and the surfaces must say so. |
| Entity linking is harder than it looks | "Apple" the company, the fruit, the record label; ticker collisions across venues; crypto projects that rename. This is where a news pipeline silently rots. The bitemporal symbology work (Wave 2) is the mitigation and it must land first. |
| LLM cost and latency at corpus scale | Do not run an LLM over every article. Use cheap filters (novelty, entity match, source tier) to gate expensive inference. `prismatik-ai-router` already has the cost-governance surface for this. |
| The engine will be wrong regularly | That is fine and expected — **but only if the coverage curve is displayed as prominently as the prediction.** Ship it with the curve or do not ship it. |
| Prediction ledger cuts both ways | It makes a bad model publicly, permanently, verifiably bad. That is the point. It is also a commercial risk that must be accepted deliberately before launch, not discovered after. |

## 23. The One-Paragraph Version

Everyone has the same news; nobody has an honest record of what it was worth. PRISMATIK's
determinism kernel, pretraining-contamination gate, and point-in-time feature store make it the only
architecture on the table that can *honestly backtest* news-driven strategies — competitors score
2019 headlines with models that already know how 2019 ended, and their results are not optimistic,
they are meaningless. Build on that: thread news into stories with lifecycle phases, score novelty
against a point-in-time corpus, measure dispersion rather than sentiment, detect anomalous silence,
and keep a calibration curve for every analyst and firm so consensus can be weighted by track record
instead of by brand. Then make the prediction engine emit falsifiable statements with intervals,
falsification conditions, and the strongest case against itself — and write every one to the
append-only hash-chained ledger at issuance, so the platform is cryptographically incapable of lying
about its own record. The differentiator was never better prediction. It is being the only system
that can prove what it said, why it said it, what it knew at the time, and how often it has been
wrong.

---

*Part I is operational. Part II is a proposal — no crates created, no wave commitments made, no work
started. §21 must be resolved by a human before any of Part II is implemented.*
