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

# PART III — THE INSTRUMENT WORKSPACE, LONGITUDINAL INTELLIGENCE, AND THE ACCURACY CONTRACT

*Proposal. Same confidence markers as Part II.*

## 24. What Is Being Asked For, Stated Precisely

> *A user selects any US-listed business or any crypto asset and gets deep trend analysis over time.
> Data across all markets, over long history, because history is what makes metrics and predictions
> real. The system should predict accurately within a stated percentage, measure risk — all of it.*

Three of those four are straightforward engineering with enough history and enough discipline. One of
them — "predict accurately within a stated percentage" — needs to be restated before it can be built,
because the obvious reading of it cannot be delivered by anyone, and products that claim to deliver
it are lying. §29 restates it into something that is both achievable and **strictly more valuable
than what was asked for.** Read that section before costing anything else here.

The rest of Part III is the full angle sweep: the workspace, the history substrate, the trend engine,
the risk engine, backtest honesty, universe-wide analytics, the time machine, and the operator's own
calibration.

---

## 25. The Universal Instrument Workspace

**Confidence: H for the shell, M for full modality coverage. This is the product's front door.**

Already in the wave plan as `P2-EX-03` ("universal instrument workspace generalized across asset
classes"). Here is what generalized has to mean.

### 25.1 One entity model, all asset classes

The workspace is keyed on `AssetId` — the canonical, content-addressed, bitemporal identity from
`prismatik-identity`. Everything else is a modality that either exists for that asset or does not.

| Entity kind | Examples |
|---|---|
| Equity | AAPL, NVDA, MSTR, delisted names |
| Crypto spot | BTC, ETH, long-tail tokens, dead tokens |
| Crypto perp / futures | funding, basis, open interest |
| ETF / ETP | SPY, IBIT, sector and thematic funds |
| Option contract | OCC-symbol identity, adjusted contracts flagged |
| Index | SPX, NDX, crypto indices |
| Synthetic pair | BTC/MSTR, any user-defined ratio or spread |
| Basket / universe | user-defined, screen output, or peer group |

**The critical design rule: the workspace never renders a modality it does not have, and never
silently omits one either.** Every instrument view opens with a **capability matrix** — what data
exists for this asset, at what depth, at what quality, with what gaps. `prismatik-market-data`
already defines `BlindSpot` and the `Concludes` trait for exactly this.

A user looking at a long-tail token must see *"no options data, no institutional ownership, on-chain
from block 1, price history 14 months, two exchange sources, one of which failed in 2023"* — not a
layout that looks identical to AAPL's with empty panels. **Blind spots are content, not absence.**
This single decision is the difference between a tool that teaches a user what they don't know and a
tool that lets them believe a thin asset is a thick one.

### 25.2 Progressive disclosure — three depths

Complex data needs hierarchy or it becomes wallpaper. Three depths, each a deliberate step:

1. **Glance** (< 2 seconds) — what is this, what regime is it in, what changed, what is the current
   calibrated view and how much has that view been worth historically. One screen. No scrolling.
2. **Analysis** (minutes) — trend decomposition, risk panel, peer relative, factor exposure,
   narrative state, positioning, event calendar. Tabbed or panelled, user-arrangeable, layout saved
   per workspace.
3. **Forensics** (as long as it takes) — every number's evidence chain, the raw records, the
   retrieval timestamps, the manifest, the leave-N-out sensitivity, the point-in-time replay.

Depth 3 is not a debug view. **It is the product.** It is where a user goes when the answer matters,
and it is the thing no competitor can offer because they cannot reconstruct what they knew.

### 25.3 Cross-asset by construction

The workspace must accept a mixed selection — `{NVDA, BTC, IBIT, MSTR, SPX}` — and analyse them on
one lattice. That is the whole point of `prismatik-lattice` (Part II §9, and the cross-market
proposal §5). A user comparing BTC to NVDA on daily closes is comparing a 00:00 UTC snapshot to a
16:00 ET snapshot; the platform must reconcile that automatically and *say that it did*, with the
projection named on the surface.

### 25.4 Customization suite (directive Phase 3.2, kept)

Panel layout, telemetry toggles, correlation layer filters, saved workspace layouts, per-workspace
universe definitions. Two rules:

- **Layouts are versioned artifacts, not blobs.** A saved workspace is content-addressed and
  reproducible, so "the view I was looking at on Tuesday" is recoverable exactly.
- **A customization may hide a panel; it may never hide a disclosure.** Interval, evidence, regime,
  and blind-spot indicators are not user-suppressible. If a user could turn off the error bars, they
  would, and then the product is a lie with a preferences menu.

---

## 26. The Longitudinal Substrate — Deep History Is the Product

**Confidence: H, and this is where the real money and real time go. Underestimating it is the most
common way platforms like this fail.**

The user is right that history is what makes metrics real. But history is not a single thing you
either have or don't — it is six separate problems.

### 26.1 Depth targets by modality

Minimum viable, and what each unlocks:

| Modality | Target depth | Unlocks | Difficulty |
|---|---|---|---|
| US equity daily OHLCV | 25+ years | Multiple full cycles, 2000/2008/2020 regimes | M (licensing) |
| US equity intraday | 10+ years | Microstructure, gap studies, event windows | H cost, H volume |
| Crypto spot daily | since inception per asset | Full life-cycle; most assets are young | M |
| Crypto intraday / trades | 5+ years majors | Funding, basis, liquidation cascades | H volume |
| Options chains + IV surface | 10+ years | Vol regimes, skew history, dealer positioning | **Very high cost** |
| Fundamentals (as-reported **and** restated) | 20+ years | Honest fundamental backtests | H — vintages are the hard part |
| SEC filings full text | 1993→ (EDGAR full) | Narrative/filing divergence, 13F, Form 4 | M — free, but parsing is real work |
| Macro (FRED with vintages) | full ALFRED vintages | No macro look-ahead | M — free |
| COT | 1986→ | Positioning cycles | L — free |
| News corpus | 10+ years | Everything in Part II | **Hardest: licensing, not tech** |
| On-chain | genesis per chain | Crypto-native flow | M — node cost |
| Corporate actions | 25+ years | Read-time adjustment | M |
| Delisted / dead universe | **same depth as live** | §27 | M — and always skipped |

**The honest sequencing insight:** free-and-deep sources (EDGAR, FRED/ALFRED, COT, on-chain) should
be ingested *early and greedily*, because their value compounds with depth and their cost does not.
Expensive sources (options chains, news, tick data) should be bought late, narrow, and only after
§13's source-valuation machinery can justify the spend with a number.

### 26.2 Bitemporality is not optional

Every historical fact needs two time axes: **when it was true** and **when we knew it**. The
architecture already commits to this; here is what it buys at instrument level.

- The **as-of view**: what the platform believed on 2023-05-24, using only data observable then.
- The **current view**: what we now believe about 2023-05-24, with all subsequent corrections.

Both are legitimate and they answer different questions. Backtests must use the as-of view. Research
retrospectives may use the current view — **but the surface must label which one it is showing**, and
mixing them within one panel is a defect.

### 26.3 Corporate actions applied at read time, never destructively

Splits, dividends, spin-offs, mergers, ticker changes, and OCC option adjustments are applied as a
**read-time factor computed from an append-only ledger**. The raw series is never rewritten. This is
already `P2-DK-02`/`P2-DK-03` and Wave 2 DoD #2 (byte comparison of raw Parquet before and after a
split ingestion).

Why it matters for the instrument workspace specifically: a user asking "what did NVDA look like in
2015" must get a coherent answer whether they want split-adjusted, unadjusted, or
total-return-adjusted — and the toggle must be explicit, because the three tell different stories and
the default is always wrong for someone.

Crypto has its own version and it is under-modelled everywhere: token redenominations, chain splits,
migrations (ERC-20 → native), rebases, airdrop-adjusted returns, and exchange-specific pair
delistings. **Treat these as corporate actions in the same ledger.** No one else does, and a crypto
backtest that ignores a token migration is silently wrong in the same way an equity backtest that
ignores a split is.

### 26.4 Vintages for anything revised

GDP, CPI, employment, and company fundamentals are all revised, sometimes substantially, sometimes
years later. A backtest using the *current* value of Q2-2019 GDP is using a number that did not exist
in Q2 2019. ALFRED provides equity-side vintages for macro; fundamentals require storing every
as-reported filing and never overwriting.

`observation_delay` handles publication lag. **Vintages handle publication *revision*.** They are
different mechanisms and you need both.

### 26.5 History quality is heterogeneous — score it

A 2013 crypto backtest and a 2013 equity backtest are not comparable, and presenting them in the same
table without qualification is misleading. Early crypto data is thin, wash-traded, exchange-failure-
ridden, and often reconstructed. Early small-cap equity data has its own pathologies.

Introduce a per-series, per-window **History Quality Score** built on `DataQualityScore` (already in
`prismatik-domain`), composed of:

- Source count and cross-source agreement
- Gap density and largest gap
- Imputation fraction (**never impute silently — flag every filled bar**)
- Suspected wash/print anomalies
- Venue survivorship (did the exchange carrying this series later fail?)
- Reconstruction provenance (was this stitched from multiple venues?)

**Every backtest and every trend statistic carries the minimum quality score of the data it consumed.**
A Sharpe of 2.1 computed on quality-0.3 data should be rendered differently from one on quality-0.95
data. This is I1 applied to the substrate.

---

## 27. Survivorship — the Bias That Silently Invalidates Everything

**Confidence: H that it matters. M that it will actually get done, because it is unglamorous.
Do it anyway. This section is the highest ratio of correctness-per-effort in Part III.**

If your universe contains only assets that exist today, every backtest, every trend statistic, and
every "accuracy" number is wrong in the same optimistic direction — and no seed, hash, signature, or
calibration curve detects it.

### What must be retained at full depth

| Market | Dead things you must keep |
|---|---|
| US equity | Delisted, bankrupt, acquired, taken private, reverse-split-to-death, deregistered |
| ETF | Closed and liquidated funds (a large fraction of thematic ETFs) |
| Crypto | Dead tokens, rug-pulls, abandoned chains, delisted pairs, **failed exchanges** |
| Options | Expired series — the entire chain, not just survivors |
| Index | Historical constituent sets with join/leave dates, **as published then** |

### The crypto-specific failure everyone commits

Crypto survivorship is *worse* than equity survivorship and almost universally ignored. The base rate
of total loss is high, the dead assets are disproportionately the ones that looked best before dying,
and the price history frequently disappears with the exchange that hosted it. **A crypto momentum
backtest on today's top-200 list is not optimistic — it is meaningless**, because the universe
definition itself encodes the outcome.

The mitigation: reconstruct point-in-time universe membership. What were the top 200 by market cap on
2018-01-15, *according to data available on 2018-01-15*? That is a hard, tedious data-engineering
problem and it is the difference between a real crypto backtest and a demo.

### Universe definitions are bitemporal artifacts

A universe is not a list. It is a **rule plus an as-of date**, resolved through the same bitemporal
symbology as everything else, pinned into the manifest so a reproduction resolves the same members.

**Gate:** `test_universe_survivorship` — construct a universe as-of a historical date, assert it
contains assets that have since died, and assert the count of dead members is non-zero for any window
older than five years. A universe with zero mortality over a decade is a bug, not a clean dataset.

---

## 28. The Trend Engine

**Confidence: M. This is what the user is asking for by name, and it is where most platforms ship
moving-average crossovers with a confident tone of voice.**

### 28.1 Trend is a latent state with uncertainty, not a line

Do not ship "the 50-day crossed the 200-day." Ship a **state-space estimate**:

- Local linear trend / structural time series decomposition → level, slope, seasonal, irregular, each
  with a posterior variance
- STL or wavelet multi-resolution decomposition for scale separation
- Kalman/particle filtering so the trend estimate updates online and its uncertainty is explicit

The rendered object is a **trend corridor**, not a trend line. Width is information: a strong trend
with wide uncertainty is a different trade from a weak trend with tight uncertainty, and a single
line erases that distinction.

### 28.2 Multi-scale by default

Trend is scale-dependent and a system that picks one scale has smuggled in an assumption. Compute and
display simultaneously: intraday, daily, weekly, monthly, secular. **Scale agreement is itself a
feature** — when all scales align, that is a different market state from when the daily fights the
weekly. Render the alignment explicitly (a small multi-scale ribbon, not five separate charts).

### 28.3 Trend persistence — the hazard function

**The most useful and least-implemented idea in this section.**

Users do not actually want to know "is there a trend." They want to know **"how much longer?"**
That is a survival-analysis question, and survival analysis answers it properly:

> *Trends with this character — this strength, this scale-alignment, this regime, this breadth —
> have historically had a median remaining life of 14 trading days, with a 25th percentile of 4 and
> a 75th of 41. The hazard rate rises sharply above day 30. n = 212 historical analogs.*

Fit a hazard model over the historical corpus of trend episodes, conditioned on regime and
characteristics. Output a **survival curve for the current trend**, not a binary "trend intact."

This reframes the entire interaction. It is honest (it is a distribution), it is actionable (it maps
directly to holding period and option tenor), and it is checkable (the hazard model is itself
calibratable against realized episode lengths).

### 28.4 Trend fragility

A trend supported by broad participation is structurally different from one held up by three names or
one exchange. Compute **fragility** from the correlation network and breadth:

- Breadth: what fraction of the universe/peer group participates?
- Concentration: what share of the move is attributable to the top contributors?
- Network support: is the trend's cluster cohesive (MST/partial-correlation, cross-market proposal §7)?
- Liquidity depth: could positions actually exit at these prices? (§30.5)
- Narrative saturation: is the story fully priced? (Part II §14.5)

Fragility and strength are orthogonal. Rendering them on two axes rather than collapsing to one score
is the whole insight — the dangerous quadrant is *strong and fragile*, and a single "trend score"
hides exactly that quadrant.

### 28.5 Change points on the trend, not the price

Run BOCPD (Part II / cross-market proposal §8) on the **trend state**, not raw price. Price is noisy
and change-point-rich; the underlying trend state is not. This produces far fewer, far more meaningful
alerts, and the run-length posterior gives "this trend is 14 days old with 70% probability" for free —
which feeds §28.3.

### 28.6 Regime conditioning, always

Every trend statistic is reported conditional on regime. "Momentum works" is not a statement with a
truth value; "momentum has a positive expected 20-day return in low-volatility, high-breadth regimes,
and a negative one in stressed regimes, with these coverage numbers" is.

---

## 29. The Accuracy Contract

**Read this section before promising anything to anyone. It is the most important section in Part III.**

### 29.1 The honest restatement

"Predict accurately within a certain percentage" has an intuitive reading — *"the system is 87%
accurate"* — that cannot be delivered by any system in any market, and every product that claims it
is either measuring something trivial, overfitting, or lying. Markets are non-stationary, partially
efficient, and reflexive. A fixed accuracy number is not a hard engineering target; it is a category
error.

**But the underlying need is completely legitimate and there is a rigorous way to serve it.** Invert
the promise:

> **You choose the confidence level. The system delivers an interval that provably contains the truth
> that often — and shows you its realized coverage so you can verify the claim yourself.**

That is **conformal prediction**, which the architecture has already committed to (§17.3: ACI as the
time-series default, Mondrian for regime-conditioning). It gives a *coverage guarantee* rather than an
accuracy claim:

- Ask for 80% → get an interval that contains the realization ~80% of the time.
- Ask for 95% → get a wider interval that contains it ~95% of the time.

Coverage is guaranteed by construction. **What varies with model skill is the interval's width.**

### 29.2 Sharpness subject to calibration — the actual measure of skill

This is the standard and correct framing, and it should be the product's spine:

> **Calibration is a constraint. Sharpness is the objective.**

Two systems both delivering 80% coverage are not equally good. The one with narrower intervals knows
more. So the headline metric is not accuracy — it is **interval width at a fixed coverage level,
compared against baselines**.

> *"90-day NVDA return, 80% interval: [−11%, +19%]. Realized coverage over the last 500 predictions:
> 79%. In high-volatility regimes: 74%. Baseline (unconditional historical) interval at the same
> coverage: [−19%, +27%]. Sharpness gain: 34%."*

That statement is honest, verifiable, comparable across assets and horizons, and vastly more useful
than "87% accurate." A user reading it understands something true. A user reading "87% accurate"
understands something false.

### 29.3 The metric suite

Point-accuracy metrics are the wrong family. Use **proper scoring rules** — they cannot be gamed by
hedging your stated confidence:

| Metric | Measures | Use |
|---|---|---|
| **CRPS** | Full distributional accuracy | Primary metric for continuous targets |
| **CRPSS** | CRPS skill vs baseline | The headline number. Positive = beats baseline |
| Pinball loss | Per-quantile accuracy | Where in the distribution is the model weak? |
| Log score | Sharpness-sensitive | Punishes overconfidence hard |
| Brier + decomposition | Binary events | Splits into reliability / resolution / uncertainty |
| Reliability diagram | Calibration curve | The user-facing honesty surface |
| **PIT histogram** | Distributional shape | Flat = well-calibrated. U-shaped = overconfident. Diagnoses *how* a model is wrong, not just that it is |
| Realized coverage vs nominal | The conformal guarantee | Per horizon, per regime, per asset |
| Interval width / sharpness | Skill at fixed coverage | The competitive number |
| Directional accuracy | Sign only | Report it — users want it — but **never alone** |

**Every one of these is reported per horizon, per regime, per asset class, and per data-quality
band.** Aggregate accuracy numbers hide exactly the heterogeneity a user needs.

### 29.4 Accuracy is heterogeneous — publish the map

The single most useful honesty feature: **the Accuracy Atlas.** A surface answering *"where is this
system actually good?"*

Realistically, and this should be stated plainly to users:

| Target | Realistic predictability |
|---|---|
| Realized volatility | **Genuinely predictable.** Vol clusters. Strong, durable skill available |
| Return *distribution* / interval | Achievable with honest calibration |
| Drawdown risk, tail behaviour | Achievable, especially regime-conditioned |
| Relative / cross-sectional ranking | Modest but real skill |
| Event *reaction* distributions | Achievable where n is sufficient |
| Directional return, short horizon | **Near the noise floor.** Tiny edges at best, mostly illusory |
| Precise price targets | Not achievable. Do not ship this |

Building the Atlas means occasionally telling a user *"we have no skill here, and here is the
evidence."* That will feel commercially painful. It is the single strongest trust-building act
available, and it is the only defensible position for a platform whose entire premise is provable
honesty.

### 29.5 The predictability ceiling

For each series and horizon, estimate an **information-theoretic ceiling** — entropy rate, permutation
entropy, or a similar complexity measure — to answer *"how much predictable structure is even present
here?"*

Then report skill **as a fraction of the achievable ceiling**, not in absolute terms. A model
capturing 60% of available structure in a near-random series is doing extraordinarily well; a model
capturing 60% in a highly structured series is underperforming. Absolute numbers conflate the two and
mislead in both directions.

This also prevents the most expensive research failure mode: pouring months into a target that
contains almost no extractable signal.

### 29.6 The promotion gate

No model reaches a user surface without passing, on **both discrimination and calibration**:

1. Beats the unconditional historical baseline (CRPSS > 0)
2. Beats the random-walk / momentum baseline
3. Beats the simplest domain heuristic
4. Realized coverage within tolerance of nominal, **per regime**
5. PIT histogram passes a uniformity test
6. Passes the pretraining-contamination gate (Part II §8)
7. Number of configurations searched is disclosed, and the deflated Sharpe / PBO check clears (§31)

Already normative in the architecture as the baseline ladder (§17.2). Part III's contribution is
making it the *accuracy contract* the product is sold on.

### 29.7 What is safe to promise

- ✅ "You pick the confidence level; we deliver calibrated intervals and show realized coverage."
- ✅ "Our intervals are N% narrower than baseline at the same coverage, on these assets, in these regimes."
- ✅ "Here is exactly where we have skill and where we don't."
- ✅ "Every prediction is in a tamper-evident ledger. Verify our record yourself, including the misses."
- ❌ "87% accurate."
- ❌ Any point price target without an interval.
- ❌ Any accuracy number not decomposed by regime and horizon.

---

## 30. The Risk Engine

**Confidence: H for standard measures, M for crypto-specific, L for model risk. "Measure risk — all
of it" is a large surface; this is the full sweep.**

Architecture already lists `P6-QM-02` (exposure, concentration, correlation, scenario analysis). This
expands it.

### 30.1 Distributional risk

- **VaR** at multiple horizons and confidences — but never alone. VaR is not subadditive and says
  nothing about the tail beyond the threshold. Shipping VaR as the headline risk number is
  malpractice.
- **CVaR / Expected Shortfall** — coherent, tail-aware. This is the headline.
- **EVT tail fitting** (peaks-over-threshold, GPD) — because the historical sample almost never
  contains the tail that matters, and empirical quantiles at 99.5% on 3 years of data are fiction.
- **Skew and kurtosis**, with the honest note that both are unstable estimators requiring long windows.

### 30.2 Path and drawdown risk

Terminal-value distributions hide the path, and users do not experience terminal values — they
experience drawdowns and quit during them.

- Maximum drawdown **distribution**, not the single historical max
- Time-under-water distribution — how long until recovery?
- Ulcer index, Calmar/MAR
- **Probability of ruin** and probability of hitting a personal pain threshold
- **Sequence-of-returns risk** — same returns, different order, materially different outcome under
  contributions or withdrawals

### 30.3 Structural risk

- Factor exposure decomposition (both markets, shared factors — cross-market proposal §9.2)
- **Hidden concentration**: a portfolio that looks diversified by name and is concentrated by factor.
  The most common real portfolio failure, and invisible to name-level checks. New pre-trade check.
- **Tail dependence (λ_L)** alongside correlation. Two assets at ρ = 0.3 with λ_L = 0.7 are a
  diversification illusion — independent on ordinary days, joined at the hip on the day that matters.
  **Report λ_L on every pair.** Nearly free once the lattice exists; enormous honesty payoff.
- Correlation instability: how much does the correlation matrix move between regimes?
- Beta stability, rolling and regime-conditioned

### 30.4 Liquidity risk

Chronically under-modelled and the reason paper results don't survive contact with size.

- ADV participation and **days-to-liquidate** at a given participation rate
- Spread cost and depth-at-price
- **Market impact** (square-root-law family), sized to the actual position
- Liquidity *regime* — depth evaporates precisely when you need it
- Crypto: per-venue depth fragmentation, and depth that is partially wash
- Options: open interest, spread width, and whether the strike trades at all

**Every backtest result should carry a capacity estimate.** A strategy with a Sharpe of 3 and a
capacity of $200k is a hobby, and the platform should say so before a user finds out with real money.

### 30.5 Crypto-specific risk

Structurally different from equity risk. Mostly absent from equity-first platforms.

| Risk | Instrumentation |
|---|---|
| Exchange counterparty | Venue solvency proxies, reserve attestations, withdrawal-latency anomalies, historical failure base rates |
| Custody | Self-custody vs exchange vs qualified custodian exposure split |
| Smart contract | Audit status, TVL-at-risk, time-since-deploy, upgrade-key centralization |
| Bridge | Cross-chain exposure — historically the highest-severity loss category |
| Stablecoin depeg | Peg deviation history, collateral composition, redemption-gate risk |
| Liquidation cascade | Aggregate leverage, funding extremes, liquidation-level clustering |
| Regulatory | Jurisdictional exposure, delisting precedent |
| Concentration | Whale/holder distribution, exchange-held supply fraction |
| Chain | Reorg depth, validator/miner concentration, halt history |

### 30.6 Equity-specific risk

Gap risk (overnight, and crypto is the overnight sensor — cross-market proposal §10), halt risk,
borrow cost and short-squeeze exposure, corporate-action risk, index rebalance flow, earnings-date
proximity, **adjusted option contracts** (already a HardDeny), and single-filing concentration.

### 30.7 Scenario and stress

- **Historical replay** — run the current portfolio through 1987, 2000, 2008, 2020, 2022, the May
  2021 and FTX crypto events
- **Hypothetical shocks** — user-defined factor moves with correlation-consistent propagation
- **Reverse stress testing** — *"what scenario breaks me?"* Solve for the shock that produces a
  target loss. Far more useful than forward stress testing and almost never offered
- **Regime-conditional stress** — correlations go to 1 in crises; stressing with calm-regime
  correlations understates loss, which is the exact error that made 2008 worse

### 30.8 Model risk — the risk that the risk model is wrong

Rarely instrumented; PRISMATIK already has the machinery.

- Drift detection: feature, calibration, embedding, performance (already `P5-QM-14`)
- **Out-of-distribution detection** — a model producing a *tight* forecast in a regime it has never
  seen is a defect, not a feature. Already doctrine in the architecture. OOD must widen intervals
  automatically and suppress beyond a second threshold, with no human in the loop
- **Ensemble disagreement as a risk metric** — when models that usually agree diverge, that is
  information about model risk, not just noise
- Estimation error on the risk numbers themselves: a VaR from 250 observations has a confidence
  interval, and it is wider than people expect. **Render it.**

### 30.9 Position sizing under uncertainty

- Kelly and fractional Kelly — with the parameter-uncertainty correction, because full Kelly on
  estimated parameters is reliably ruinous
- Risk parity / equal risk contribution
- **Hierarchical Risk Parity** — uses the correlation-metric tree (cross-market proposal §7),
  avoids matrix inversion, far more robust out-of-sample than mean-variance
- Volatility targeting with a regime-aware target
- Explicit maximum-loss budgeting per position and per day

### 30.10 The Risk Ledger

Every risk number carries: estimation window, **effective sample size**, regime, data quality score,
and the model that produced it. A CVaR with no estimation window attached is a number with no meaning,
and rendering one is an I1 violation.

---

## 31. Backtest Honesty — Where Prediction Claims Live or Die

**Confidence: H. Mostly already in the wave plan; consolidated here because §29's claims are only
worth what this section enforces.**

| Discipline | Why |
|---|---|
| **Purged + embargoed CV** | Drop training samples whose label horizon overlaps the test window, plus a buffer after. Without both, overlapping labels leak and every OOS number is optimistic. Already `P4-QM-09` |
| **Combinatorial purged CV** | Multiple backtest paths → a *distribution* of Sharpe, not one number |
| **Deflated Sharpe Ratio** | Corrects for multiple testing, non-normality, and sample length |
| **Probability of Backtest Overfitting** | Explicit estimate that the selected strategy is a fluke |
| **Configurations-searched disclosure** | Deflated Sharpe requires N. Report it prominently — a Sharpe of 2.0 from 5 configs and from 50,000 are different objects. Already in the v0.1 walk-forward panel; keep it |
| **Minimum backtest length** | For a given Sharpe, there is a minimum history below which the result is not distinguishable from luck. State it |
| **Realistic costs** | Fees, spread, slippage, impact sized to position, borrow, funding. Frictionless backtests are fiction |
| **Capacity estimate** | §30.4 |
| **Survivorship-clean universe** | §27 |
| **Point-in-time features** | `observation_delay` + vintages |
| **Regime-stratified results** | A strategy that works only in one regime should show it, not average it away |
| **Walk-forward efficiency** | OOS performance ÷ IS performance. Low ratio = overfit |

**The multiple-testing disclosure is the differentiator.** As the v0.1 README already says: multiple-
testing bias is how these products quietly lie. Surfacing it is the whole posture.

---

## 32. Cross-Sectional and Universe-Wide Analytics

Instrument-level depth is necessary; universe-level breadth is what makes it actionable.

- **Screening with uncertainty.** Ranked screens must carry **rank intervals**, not point ranks. The
  #3 and #17 names are frequently statistically indistinguishable, and a bare ordered list implies a
  precision that does not exist. FDR control applies here exactly as in the correlation matrix
  (cross-market proposal §6.2) — screening 5,000 names on 20 metrics is 100,000 tests.
- **Data-driven peer groups.** GICS/sector labels are coarse and stale. Cluster on realized
  co-movement, factor loadings, and narrative co-occurrence. MSTR's real peer group includes BTC.
- **Relative value** — spreads, ratios, cointegration (with proper testing and a stationarity verdict,
  not eyeballed charts).
- **Breadth and dispersion** as first-class universe metrics; they feed regime detection (§28.4).
- **Factor exposure per name**, both markets, shared factor space.
- **Cross-sectional momentum/reversal** with the survivorship-clean universe from §27.

---

## 33. The Time Machine

**Confidence: M. The most striking UX expression of the entire architecture. Build it as the demo.**

A date scrubber on the instrument workspace. Drag it to any historical instant and the **entire
workspace re-renders to what the platform knew at that moment** — prices, features, news, analyst
consensus, positioning, regime label, and the predictions it was making.

> *"What did we say about NVDA on 2023-05-24, on what evidence, and what actually happened?"*

Nothing else on the market can do this honestly, because it requires bitemporal symbology,
`observation_delay`, vintage handling, versioned embedding indices, an append-only raw layer, and a
prediction ledger — all of which exist in this architecture and essentially nowhere else.

Two hard rules:

- The as-of view must be **visually unmistakable** — a persistent, high-contrast chrome state, not a
  small badge. A user who forgets they are in the past and trades on it has been actively harmed.
- Pair it with the **Prediction Ledger** (Part II §14.4): scrub to a date, see the prediction, scrub
  forward, see the resolution. The system grades itself in front of the user, including the misses.

This is also the most persuasive sales artifact available. It is one interaction that demonstrates
the entire invariant set at once.

---

## 34. The Operator Mirror — Calibrating the User

**Confidence: M. Uses `prismatik-journal`, which already exists and already has the right shape.**

The journal crate already specifies a three-layer memory loop and `TriggerWinRate`. Point the same
calibration machinery that scores models and analysts (Part II §12.2) at **the user's own decisions.**

- Every thesis logged with its reasoning, conviction level, horizon, and evidence
- Scored on realization, exactly like an analyst
- **The user's personal calibration curve**: *"when you say you're 80% confident, you're right 61% of
  the time — and 44% in high-volatility regimes"*
- Per-setup, per-sector, per-regime hit rates via the existing memory layers
- Behavioural pattern surfacing: overtrading after losses, conviction inflation after wins, holding
  losers past the stated thesis invalidation, systematically ignoring your own falsification criteria
- **Counterfactual sizing**: *"with your actual signals and flat sizing, you'd be up 14% instead of
  3% — your sizing is subtracting alpha"*

This is the single most valuable thing a serious individual operator can receive, nobody offers it,
and the infrastructure is already specified. It also completes the platform's philosophical arc: it
calibrates its models, it calibrates the analysts, and it calibrates you — with the same machinery
and the same honesty.

Handle with care in the UI. This information is genuinely useful and genuinely uncomfortable.
Frame it as instrumentation, never as judgment.

---

## 35. Alerting That Doesn't Become Noise

- Alert classes: regime transition, trend hazard spike (§28.3), calibration decay, silence anomaly
  (Part II §11.4), risk-limit proximity, correlation-structure break, liquidity degradation, thesis
  invalidation (from the user's own stated falsifiers), data-quality degradation.
- **Alerts are predictions and must be calibrated like predictions.** Track per-alert-type precision:
  *"regime-transition alerts have preceded an actual transition 41% of the time."* An uncalibrated
  alert stream trains users to ignore it, which is worse than no alerts.
- Adaptive thresholds by regime — a fixed vol threshold fires constantly in stress and never in calm.
- Every alert links to its evidence chain and its "what would falsify this."

---

## 36. Data Quality as a First-Class Surface

Not a backend concern — a rendered one.

- Live gap detection, provider disagreement, outlier and stale-quote flagging
- **Never impute silently.** Every filled value is flagged, and the fill method is part of lineage
- Provider divergence as a signal in its own right — when two feeds disagree about a print, that is
  information
- **Every analysis carries the minimum quality score of its inputs** (§26.5). *"This backtest ran on
  data that was 3% imputed and drew from a venue that failed in 2023"* is a sentence the platform
  should be capable of generating automatically

---

## 37. Performance and Scale

The directive's Phase 2.1 asks, made concrete for this workload:

- Arrow end-to-end, zero-copy to sidecars. Never serialize a large panel to JSON
- DataFusion predicate/projection pushdown into partitioned Parquet
- Incremental materialization: recompute only the affected windows, never full rebuilds
- Content-addressed caching keyed on `(inputs, artifacts, seed, as_of)` — a cache key that includes
  the determinism context is safe under I5; a wall-clock key is not (§2)
- Query budgets with graceful degradation: a screen that would scan 20 years × 5,000 names should
  narrow with disclosure, never silently sample
- GPU (wgpu) for the heavy visuals — surfaces, path clouds, correlation graphs, heatmaps — with the
  mandatory Canvas/CPU fallback
- Deterministic parallel reduction throughout (fixed chunking, fixed merge order, compensated
  summation). **Non-associative float addition under work-stealing silently breaks I3 while the
  numbers still look plausible.**
- Latency targets: glance view < 300 ms warm; analysis panel < 2 s; full backtest async with progress
  and cancellation

---

## 38. Consolidated New Crates

Across Parts II and III, and shared with the cross-market proposal:

```
prismatik-lattice        Temporal alignment: venues, sessions, information arrival.   SHARED — build once
prismatik-dependence     Correlation, lead-lag, tail dependence, information flow
prismatik-regime         Change points, state inference, regime artifacts
prismatik-convergence    Factor attribution, network structure, market state tensor
prismatik-narrative      Story threading, novelty, lineage, diffusion, silence
prismatik-consensus      Analyst/firm call ledger, track-record calibration
prismatik-hypothesis     Falsifiable statements, baseline ladder, prediction ledger
prismatik-trend          Trend state estimation, multi-scale, hazard/survival, fragility
prismatik-riskmetrics    VaR/CVaR/EVT, drawdown, liquidity, capacity, stress, model risk
prismatik-universe       Bitemporal universe definitions, survivorship-correct membership
```

All Layer 2, ports only, no storage backends, no vendor SDKs, no HTTP clients. Heavy or research-grade
estimation goes to sidecars first and earns a crate only after out-of-sample validation.

---

## 39. Consolidated Definition of Done

| # | Criterion | Verified by |
|---|---|---|
| 1 | No conclusion renders without interval, evidence chain, regime, and ESS | Type-level + `test_no_orphan_conclusions` |
| 2 | Realized coverage within tolerance of nominal, per regime, per horizon | Rolling coverage report, 500+ observations |
| 3 | PIT histograms pass uniformity | Automated per model per promotion |
| 4 | Every model beats the full baseline ladder on discrimination **and** calibration | Promotion gate |
| 5 | Configurations-searched disclosed on every backtest; deflated Sharpe computed | Backtest result type |
| 6 | Universe as-of a historical date contains since-dead assets | `test_universe_survivorship` |
| 7 | No feature returned where `event_time + observation_delay > as_of` | Property test, 10k cases |
| 8 | Corporate actions applied at read time; raw bytes unchanged | Byte comparison before/after split ingestion |
| 9 | Correlation matrix bit-identical across 100 runs and 2 machines | DST replay + cross-machine harness |
| 10 | Every backtest carries a capacity estimate and a minimum data-quality score | Result type |
| 11 | Tail dependence reported on every rendered pair | UI review |
| 12 | Time Machine as-of state visually unmistakable | UX review + user test |
| 13 | Predictions written to the audit ledger at issuance, externally verifiable | `prismatik-cli verify` on a published tree head |
| 14 | Accuracy Atlas published, including regions of no skill | Content review |
| 15 | Blind spots rendered as content on every instrument view | UI review |
| 16 | Alert types carry their own realized precision | Alert config surface |

---

## 40. Sequencing for Part III

| Stage | Content | Depends on | Conf |
|---|---|---|:---:|
| 1 | `prismatik-universe` + survivorship-correct history ingestion | Bitemporal symbology (Wave 2) | H |
| 2 | History Quality Score wired into every result type | Stage 1 | H |
| 3 | Instrument workspace shell + capability matrix + blind spots | Stage 1 | M |
| 4 | `prismatik-trend`: state-space estimation, multi-scale | `prismatik-lattice` | M |
| 5 | `prismatik-riskmetrics`: distributional, drawdown, structural, liquidity | Stage 1 | M |
| 6 | Accuracy metric suite (CRPS/CRPSS/PIT/coverage) + promotion gate | `prismatik-calibration` | M |
| 7 | Backtest honesty consolidation (DSR, PBO, combinatorial purged CV, capacity) | Stage 5 | M |
| 8 | The Accuracy Atlas | Stage 6 | M |
| 9 | Trend hazard / survival model | Stage 4 | L |
| 10 | Crypto-specific risk suite | Stage 5 | M |
| 11 | The Time Machine | Stages 1–3 + prediction ledger | M |
| 12 | Operator Mirror on `prismatik-journal` | Stage 6 | M |
| 13 | Reverse stress testing, HRP sizing | Stage 5 | L |

**Stages 1, 2, and 6 are the load-bearing three.** Survivorship-correct history, quality scoring, and
the accuracy metric suite are what make every other number in the product mean something. Skipping
them produces a platform that is faster at being wrong.

---

## 41. What I Would Cut, and the Honest Risks

**Cut order:** trend hazard/survival (L — needs a large, cleanly-labelled corpus of trend episodes and
the labelling is subjective), reverse stress testing (L — the optimization is fiddly and the output is
easy to misread), predictability-ceiling estimation (L — entropy-rate estimators on short financial
series are themselves noisy), smart-contract and bridge risk scoring (L — largely qualitative, hard to
keep current).

| Risk | Reality |
|---|---|
| **History acquisition is the real cost and the real timeline** | Options chains, tick data, and licensed news are expensive and recurring. Deep history is the moat and it is bought, not coded. Model this before committing to Part III |
| Survivorship reconstruction for crypto is genuinely hard | Point-in-time top-N membership from 2017 may be partially unrecoverable. Where it is, say so and bound the analysis rather than quietly using today's list |
| The Accuracy Atlas will show weak areas | That is the feature. It must survive the first commercial conversation where someone asks to hide a red cell. Decide now, in writing, that it will not be hidden |
| Calibration guarantees assume near-exchangeability | ACI relaxes this and is the correct default for time series, but no conformal method survives a true structural break. Per-regime coverage makes the failure visible rather than silent — which is the best available outcome, not a fix |
| Risk numbers invite false precision | A CVaR rendered to four significant figures implies certainty that a 250-observation estimate does not have. Render estimation error alongside every risk figure or round aggressively |
| Scope | Part III is comfortably multiple engineer-years in full. Stages 1, 2, 6, and the workspace shell are the defensible minimum and are worth doing alone |

---

## 42. Closing — What This Adds Up To

Most platforms in this space are built on an implicit promise of *certainty delivered attractively*.
They render a number, they render it beautifully, and the number is not accountable to anything.

The architecture already in this repository makes the opposite bet, and Parts II and III are that bet
carried to its conclusion:

- **Deep, survivorship-correct, bitemporal history**, because a metric computed on a universe that
  encodes its own outcome is not a metric.
- **Trend as a state with uncertainty and a hazard function**, because the real question was never
  "is there a trend" but "how much longer, and how fragile."
- **Calibrated intervals instead of accuracy claims**, because coverage is provable and accuracy is
  not — and because sharpness-subject-to-calibration is a harder, more honest, and more competitive
  target than any percentage.
- **Risk measured across distribution, path, structure, liquidity, venue, and model**, with every
  number carrying its estimation window and effective sample size.
- **An accuracy map that admits where there is no skill**, because a system that is never wrong in
  public is a system nobody should trust.
- **A ledger that makes the platform cryptographically incapable of lying about its record**,
  including the misses.
- **A time machine that lets a user stand in any past moment** and see exactly what was known and what
  was said.
- **And a mirror that calibrates the operator** with the same machinery it uses on its models and on
  Wall Street's analysts.

The differentiator was never going to be a better prediction. Everyone's predictions converge toward
the same noisy ceiling, and the ones that don't are usually overfit. The differentiator is being the
only system that can prove what it said, why it said it, what it knew at the time, how often it has
been wrong, and exactly where its skill runs out.

That is a defensible product. It is also, not incidentally, an honest one.

---

*Part I is operational. Parts II and III are proposals — no crates created, no wave commitments made,
no work started. §21 (licensing, defamation, adviser regulation) and §41 (history acquisition cost)
must be resolved by a human before implementation.*
