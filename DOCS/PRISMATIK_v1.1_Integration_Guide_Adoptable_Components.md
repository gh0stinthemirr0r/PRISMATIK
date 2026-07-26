# PRISMATIK v1.1 — Integration Guide: Adoptable Components

**Companion to:** `PRISMATIK_v1.1_Reference_Harvest_and_Bleeding_Edge_Addendum.md`
**Scope:** the seven `reference/` projects usable **today** under their existing public licenses, with no additional permission required.
**Companion file:** `PRISMATIK_v1.1_Approval_Register.md` covers the fourteen that need a grant.
**Date:** 2026-07-26

---

## 0. Scope and Reading Order

This file answers "what exactly do I take, and how do I wire it in" for components you can act on immediately. Everything here is MIT or Apache-2.0, verified by reading the `LICENSE` file.

| # | Component | License | Mode | Phase | Effort |
|---|---|---|---|---|---|
| 1 | **Kronos** | MIT | C — isolated sidecar + model artifact | 5 | L |
| 2 | **QuantDinger** | Apache-2.0 | E ref → A for MCP shape | 1, 4 | M |
| 3 | **ai-trading-claude** | MIT | B — adapt directly | 2–3 | S |
| 4 | **adata** | Apache-2.0 | E — pattern only | 1 | S |
| 5 | **stockbot-on-groq** | Apache-2.0 | E — UX/contract ref | 2 | S |
| 6 | **ai-algotrading-agent** | MIT | E — conformance fixtures | 0–1 | S |
| 7 | **AIAlpha** | MIT | E — feature methodology | 4 | M |

**Attribution obligation, all seven:** MIT and Apache-2.0 both require the license text and copyright notice to ship with any distribution containing derived code. Apache-2.0 additionally requires a `NOTICE` file if upstream provides one, and requires you to state significant modifications. Your v0.4 §Part XI intake pipeline already generates notices — these are its first real inputs. Register each as a `ThirdPartyComponentManifest` before writing integration code, not after.

---

## 1. Kronos — Financial TSFM (MIT)

### 1.1 What it actually is

`reference/Kronos-master/model/kronos.py` contains two artifacts, and the distinction matters because most people only notice the first:

- **`KronosTokenizer`** — a hybrid encoder/decoder transformer that quantizes OHLCV bars using **Binary Spherical Quantization**, with hierarchical `s1_bits` (coarse "pre" token) and `s2_bits` (fine "post" token). This is the actual contribution. It is a *learned, financial-domain* discretization of bar data.
- **`Kronos`** — a decoder-only autoregressive model over that token space, plus a `KronosPredictor` wrapper. Both use `PyTorchModelHubMixin`, so weights load from HuggingFace by repo id.

The family spans 4.1M → 499.2M parameters, pretrained on 12B K-line records from 45 exchanges.

### 1.2 The three distinct uses — pick deliberately

Most integrations only take (a). For PRISMATIK, (b) and (c) are arguably worth more.

**(a) Point/quantile forecasting.** Standard TSFM use. Feeds the Forecast Fan surface in v0.4 §44. This is the *least* differentiated use — Chronos-2 competes directly here.

**(b) Generative path synthesis for the Monte Carlo lab.** Kronos reports +22% generative fidelity on synthetic K-line sequences. Because it is autoregressive over a learned financial token space, sampling it produces *regime-plausible* paths rather than the parametric or bootstrap paths in your §8 simulation plane. Your v0.4 §44 "Simulation Comparison" surface lists historical bootstrap / parametric / regime-switching / TSFM / ensemble — Kronos is the strongest available candidate for the TSFM row **and** it upgrades the whole panel, because it gives you a path generator whose failure modes differ from the parametric ones. Uncorrelated failure modes are the entire point of that comparison.

**(c) The tokenizer as a standalone embedding source.** `KronosTokenizer` encodes any OHLCV window into a discrete token sequence. That is exactly the embedding you need for the **Embedding Analog Map** (v0.4 §44) and for LanceDB analog search (v1.0 §15.4, gap G06). You can use the tokenizer *without* running the predictor — much cheaper, and it directly closes a load-bearing gap.

> **Recommendation: adopt (c) first in Phase 4/5, then (b), then (a).** (c) closes G06 with a domain-appropriate embedding instead of a generic one. (a) is the crowded, least-defensible use.

### 1.3 How to wire it

**Mode C, isolated sidecar.** It is PyTorch; it does not enter the Rust core.

```text
prismatik-core (Rust)
    │  signed, typed gRPC   [tonic]
    ▼
prismatik-tsfm-worker (Python, containerized)
    ├── KronosTokenizer  →  encode(window) -> Vec<TokenId>
    ├── Kronos           →  sample(context, n_paths, horizon) -> PathSet
    └── no network egress, no credentials, read-only signed dataset mount
```

Contract sketch — put this in `prismatik-research-bridge`:

```rust
pub struct TsfmForecastRequest {
    pub model_ref: ModelArtifactId,      // registry id, resolves to a pinned digest
    pub series: BarWindow,                // point-in-time-correct, from the feature view
    pub horizon: usize,
    pub sample_count: usize,
    pub quantiles: Vec<f64>,
    pub seed: DeterministicSeed,          // from prismatik-determinism, never ambient
    pub temperature: OrderedFloat<f64>,   // pinned in the model manifest, not caller-chosen
}

pub struct TsfmForecastResponse {
    pub model_digest: ContentHash,        // MUST match the request's resolved digest
    pub tokenizer_digest: ContentHash,    // separate artifact, separately pinned
    pub quantile_paths: QuantileBands,
    pub sample_paths: Option<Vec<Path>>,
    pub calibration_ref: Option<CalibrationReportId>,
    pub out_of_domain: OutOfDomainVerdict,
    pub data_cutoff: OffsetDateTime,
}
```

**Two digests, not one.** The tokenizer and the predictor are separate HuggingFace artifacts with independent versions. A tokenizer change silently invalidates every cached embedding and every stored analog index. Pin and record both.

### 1.4 Adoption steps

1. Register `NeoQuasar/Kronos-*` tokenizer and predictor as **separate** model artifacts in the §17 registry. Record: MIT license, AAAI 2026 paper reference, **pretraining cutoff**, the 45-exchange/12B-bar corpus description, and a contamination note.
2. **Contamination review is mandatory and non-trivial here.** Pretraining covered 45 global exchanges. Any backtest whose window predates the model cutoff is contaminated for US equities and major crypto pairs. Your §17 governance must refuse to score a strategy on pre-cutoff data using Kronos, or must label the result as in-sample. This is the single most likely way to fool yourself with this model.
3. Build the sidecar: pinned base image, `torch` pinned, egress-denied, signed read-only dataset mount, no credentials. Reuse the §6.6 MAPIE sidecar pattern.
4. Determinism: seed `torch`, `numpy`, and Python `random` from `prismatik-determinism`; set `torch.use_deterministic_algorithms(True)`; pin temperature and top-p in the model manifest rather than accepting them per call. Record the response as a replayable effect — replay returns the recording, never re-invokes.
5. Conformance: benchmark against Chronos-2 (Apache-2.0), Chronos-Bolt, and a seasonal-naive / Auto-Theta / GARCH baseline, **per frequency band** (see addendum §4.1). Kronos is finance-specialized, so expect it to win on daily/hourly bars and lose to nothing much at all on secondly — but *verify*, do not assume.
6. `finetune/qlib_data_preprocess.py` shows the Qlib data path. Useful because it maps directly onto your §6.7 Phase 5 Qlib sidecar; you can reuse the shape of the preprocessing contract even if you replace the data source with PRISMATIK feature views.

### 1.5 Traps

- **`examples/` is heavily China-market oriented** (`prediction_cn_markets_day.py`, `get_akshare_date_*.py`, akshare dependency). Take `prediction_example.py`, `prediction_batch_example.py`, and `run_backtest_kronos.py` as API references; ignore the data plumbing entirely.
- `sys.path.append("../")` in `model/kronos.py` — the repo assumes a specific CWD. Repackage properly before containerizing.
- MIT covers the *code*. **Model weights on HuggingFace can carry separate terms.** Verify the weight repo's license independently; a permissive code license does not imply permissive weights. This is the exact trap your v1.0 §6.7 flags for Moirai.
- Autoregressive sampling cost scales with `sample_count × horizon`. Budget it in §28 performance budgets before it lands on an interactive surface.

---

## 2. QuantDinger — MCP Surface and Pipeline Shape (Apache-2.0)

### 2.1 The MCP tool surface — the highest-value artifact here

`mcp_server/src/quantdinger_mcp/server.py` exposes 22 tools. The full list is worth reproducing because it is a working answer to "what does a trading platform expose to an agent," and it de-risks your §20.3 `rmcp` work considerably:

**Read / identity:** `whoami`, `check_health`, `list_markets`, `search_symbols`, `get_klines`, `get_price`, `list_strategies`, `get_strategy`, `runtime_overview`, `list_indicators`, `get_indicator`

**Job lifecycle:** `get_job`, `list_jobs`, `wait_for_job`, `stream_job_until_done`

**Authoring loop:** `get_indicator_authoring_contract` → `validate_indicator_code` → `save_indicator`

**State-changing:** `stop_strategy`, `place_quick_order`

### 2.2 What to take

**Take the authoring loop wholesale — it is the best idea in this repo.** The agent first *requests the contract*, then writes code against it, then *validates before saving*. Three separate tools, in order. This is the correct shape for your §18 StrategyIR and your §20.4 wasm plugin host:

```text
get_strategy_authoring_contract  →  returns the StrategyIR schema, available
                                     indicators, capability envelope, and limits
        ↓  agent writes candidate
validate_strategy                →  parse → typecheck → capability check →
                                     determinism lint → dry-run on fixture data
        ↓  only if valid
save_strategy                    →  persists as an unsigned draft, never live
```

The agent never emits an executable artifact directly. Validation is a *tool*, not a post-hoc check — so failure is a normal, cheap, in-loop event the agent can iterate against rather than a hard error at the end. That materially improves small-model performance, which matters given your LM Studio decision.

**Take the job-lifecycle quartet.** `wait_for_job` and `stream_job_until_done` as *distinct* tools is a real insight: agents handle long-running work badly, and giving them an explicit blocking primitive beats letting them poll `get_job` in a loop and burn context. Your `/api/v1/jobs/:id` already has the shape; add the two waiting variants.

**Take the observability posture.** They ship `docker-compose.observability.yml` as a separate, always-present file rather than an afterthought. Adopt for §26.

### 2.3 What to explicitly reject

**`place_quick_order` as an MCP tool is precisely what your architecture forbids.** It gives a language model direct write authority over an order path. Your §19 invariant — and the 2026 literature consensus in the addendum §4.2 — is that the LLM proposes and deterministic code disposes. The correct PRISMATIK analogue is:

```text
propose_order_intent  →  returns a ProposalId + the full deterministic risk
                          evaluation (every gate, pass/fail, with reasons)
                          NOTHING is submitted. The agent cannot submit.
        ↓
[human or policy engine approves out-of-band, outside the agent's context]
        ↓
execution happens in the Rust core, which accepts only RiskApproved<OrderIntent>
```

The `RiskApproved<T>` newtype should be constructible **only** inside the risk kernel. Then "the agent cannot bypass risk" is a compile-time property rather than a policy statement. Same for `stop_strategy` — it is state-changing and belongs behind the guardian gate, though it is far more defensible than order placement since it is strictly risk-reducing. A reasonable rule: **agents may take risk-reducing actions directly; risk-increasing actions require the approval gate.**

### 2.4 Adoption steps

1. Read `server.py` and `security.py` as a design reference; implement in Rust with `rmcp` (§6.5/§20.3). It is Apache-2.0, so direct adaptation is permitted if you prefer — attribute and note modifications.
2. Split your tool registry into three capability classes — `ReadOnly`, `RiskReducing`, `RiskIncreasing` — and make the class a required field on every tool registration. Deny-by-default on the third.
3. Implement the authoring-contract triple for StrategyIR and for indicators.
4. Add `wait_for_job` / `stream_job_until_done` to the existing job API.
5. Adopt the standalone observability compose file.

### 2.5 Traps

- Apache-2.0 §4(b): if you adapt files, you must carry the license and **state significant changes**. Your notice generator needs a modifications field.
- Their stack is Postgres 18 + Redis 8 + Celery. You have Postgres + ClickHouse + DuckDB and no Celery. Take the *contracts*, not the runtime.
- `place_quick_order` exists in a shipping product. Its existence is not an endorsement; treat it as a documented example of the failure mode you are designing against.

---

## 3. ai-trading-claude — Agent Fan-Out Shape (MIT)

### 3.1 What it is

Sixteen skills and five subagents, all markdown, MIT-licensed. `skills/trade-analyze/SKILL.md` is a three-phase orchestrator: **Discovery → parallel fan-out → synthesis.** The five agents are `trade-technical`, `trade-fundamental`, `trade-sentiment`, `trade-risk`, `trade-thesis`.

### 3.2 The one non-obvious design decision

Phase 1 is *"Discovery (You Do This Directly)"* — the orchestrator gathers shared foundational data **before** launching any agent, with the stated reason: *"This prevents 5 agents from redundantly searching for the same basic information."*

That is the right instinct with a cost-saving justification, but it has a **more important second-order effect that the source does not name**: shared discovery makes the correlated-error problem *worse*, not better. If all five analysts read the same discovery bundle, their "independent agreement" is not independent. Five agents agreeing on a thesis derived from one shared news summary is one opinion reported five times, and it will read to a user as high confidence.

**PRISMATIK's correction — do both, and make the split explicit:**

```rust
pub struct AnalystScope {
    /// Deterministic, shared by all analysts: asset identity, current quote,
    /// calendar context. Facts, not interpretations. Cheap to share, no
    /// correlation risk because there is nothing to interpret.
    pub common_facts: FactBundle,
    /// Disjoint per analyst. The technical agent MUST NOT see the news the
    /// sentiment agent sees. This is what makes agreement meaningful.
    pub private_evidence: EvidenceScope,
}
```

Then agreement across analysts is genuine signal, and you can report it honestly: *"4 of 5 analysts converged on disjoint evidence"* is a defensible claim. *"4 of 5 agents agreed after reading the same article"* is not. Given that your entire moat is a governed evidence graph with an uncertainty contract, this distinction is directly load-bearing for the product's central claim.

### 3.3 Adoption steps

1. Adapt the five agent role definitions as the initial `AnalystRole` set. MIT — attribute in `NOTICE`.
2. Implement `AnalystScope` with the common/private split above. Enforce it at the evidence-query layer so an analyst *cannot* retrieve outside its scope, rather than being asked not to.
3. Composite score: keep the 0–100 shape, but **ship the decomposition** — sub-scores, weights, and the evidence IDs behind each. An opaque composite is the anti-pattern your evidence plane exists to prevent.
4. Record per-analyst disagreement as a first-class output. Divergence is information; averaging it away destroys the most useful thing the fan-out produces.
5. Route to LM Studio T1 (7B–14B) per analyst with schema-constrained output; reserve T2/T3 for the synthesis pass, which is where reasoning actually matters.

### 3.4 Traps

- These are prompts, not code — quality varies and they are tuned for WebSearch-backed retrieval, not for a governed evidence graph. Rewrite retrieval; keep role decomposition.
- The disclaimer language in `SKILL.md` exists for a reason. Your §22 compliance plane needs equivalent, and for EU high-risk classification, stronger.
- `install.sh` does `curl | bash`. Explicitly forbidden by v0.4 §39. Do not run it; read the files directly.

---

## 4. adata — Multi-Source Failover (Apache-2.0)

**Ignore all China A-share data.** The transferable asset is one architectural commitment, stated in the README: *多数据源融合切换* — multi-source fusion with automatic switching, adopted explicitly to guarantee availability. `adata/common/base/base_req.py` and `base_ths.py` carry the request/failover plumbing.

### 4.1 What to build

Your §16 provider plane governs cost and entitlements. It must also govern **degradation**:

```rust
pub struct ProviderChain {
    pub capability: DataCapability,        // e.g. DailyBars, OptionsChain, Filings
    pub primary: ProviderId,
    pub fallbacks: Vec<ProviderId>,        // ordered
    pub agreement_policy: AgreementPolicy, // NofM { n: 2, m: 3 } for critical reads
    pub divergence_action: DivergenceAction, // Halt | PreferPrimary | FlagAndContinue
    pub failover_trigger: FailoverTrigger, // RateLimited | Empty | Stale | Error | Timeout
}
```

**The rule that makes this more than a retry loop: provider identity flows into the evidence graph.** A bar from the fallback is *not the same evidence* as one from the primary. A backtest that silently mixed sources is not reproducible, and your reproducibility manifest (Appendix C) must record the per-request provider actually used, not the configured chain.

`FailoverTrigger::Empty` is the one people forget. AI-Trader's changelog documents exactly this in a US context — Alpha Vantage primary, yfinance fallback, triggered *"when Alpha Vantage is missing, rate-limited, or returns no usable price."* An empty 200 is the common real-world failure and it is invisible to naive error handling.

### 4.2 Adoption

Phase 1, small. Write it fresh — you need none of adata's code, only the commitment. Suggested US/crypto chains: crypto bars `Coinbase → CCXT public worker → CoinGecko`; equity bars `Alpaca → fallback`; crypto identity `CoinGecko` canonical (v0.4 §30) with no fallback, because identity divergence must halt rather than silently resolve.

---

## 5. stockbot-on-groq — Tool-Use → Generative UI (Apache-2.0)

Next.js app where LLM tool calls render live interactive financial components. `components/stocks/` and `components/tradingview/` hold the rendered surfaces; `lib/chat/` holds the tool wiring.

**What to take:** the contract shape that makes your v0.4 §43 `chart.compose` tool concrete. The invariant it demonstrates — and which your architecture already mandates — is that **the model emits a typed, validated component request, never markup or script.**

```rust
pub enum AiUiIntent {
    ComposeChart(ChartComposeRequest),   // → ChartDocument, backend-agnostic
    ShowTable(PerspectiveViewRequest),   // → governed query plan, budgeted
    ShowForecast(ForecastFanRequest),
    ShowComparison(ComparisonRequest),
}
```

Each variant validates to a domain object (`ChartDocument`, never a vendor config), is capability-checked, and carries the evidence IDs backing it. Rendering is a pure function of the validated intent.

**Traps:** it is a demo — no auth model, no query budget, no evidence provenance. Take the interaction pattern and none of the security posture. Their TradingView components use the *widget* embed, which per v0.4 §9 is public-web-only and must not become a desktop component.

---

## 6. ai-algotrading-agent — Conformance Fixtures (MIT)

Small, clean TypeScript crypto framework: backtest / tick-by-tick replay / simulation, with entry-exit strategies, stop-loss, and trailing stops, plus CSV fixtures in `hist-10m/` and `hist-10s/`.

**Its real value is as a second independent implementation for your conformance harness** (v0.4 Phase 0.4C step 5: *"compare against at least two independent implementations"*). It is small enough to read completely, MIT so you can vendor its fixtures, and its trailing-stop and SMA-cross semantics are simple enough that any divergence points to a genuine bug rather than a definitional difference.

**Adopt in Phase 0/1:** vendor `hist-10m/*.csv` as golden inputs; port SMA-cross + trailing-stop to `prismatik-indicator-core`; assert bit-comparable results within a declared `NumericalTolerance`. **Trailing stops are worth targeting specifically** — the intrabar update order (does the stop ratchet before or after the bar's low is tested?) is the classic silent lookahead bug, and having a second implementation to disagree with is how you find it.

---

## 7. AIAlpha — Feature Methodology (MIT)

2018-era code, durable ideas. It implements the *Advances in Financial Machine Learning* pipeline: **information-driven bars** (`bar_sample.py` — tick/volume/dollar bars instead of time bars), feature engineering, a stacked autoencoder for dimensionality reduction (`pca_auto.py`), then LSTM regression / random-forest classification.

**Take two things, ignore the models:**

1. **Information-driven bar sampling.** Dollar bars sample on cumulative traded value rather than wall-clock time, which produces returns closer to IID and far better behaved statistically than time bars. This belongs in `prismatik-indicator-core` as a first-class `BarSampler` alongside time bars — and it matters *more* for crypto than equities, since crypto has no session structure to make time bars meaningful. Your v0.4 Phase 0.4C core indicator list has no bar-sampling primitive at all; this is a genuine gap.

```rust
pub enum BarSampler {
    Time(BarInterval),
    Tick { count: usize },
    Volume { threshold: Decimal },
    Dollar { threshold: Decimal },
}
```

Every sampler must be deterministic, streaming-capable, and point-in-time safe, and the choice must be recorded in the reproducibility manifest — resampling changes results, so it is a first-class experiment parameter, not a preprocessing detail.

2. **Purged, embargoed walk-forward.** Your `server/` already does walk-forward with bootstrap and reports configurations-searched for multiple-testing bias — good, and ahead of most. The AFML additions worth taking: **purging** (drop training samples whose label horizon overlaps the test window) and **embargo** (drop a buffer after the test window). Without them, overlapping labels leak across the split and every out-of-sample number is optimistic. Add to §27 and to the walk-forward panel.

**Ignore:** the specific LSTM/autoencoder architectures are superseded — that role is Kronos's tokenizer (§1.2c) or a modern encoder. Take the pipeline, not the models. The repo is explicit that it is educational and not live-trading ready.

---

## 8. Consolidated Backlog Delta

**Phase 0**
- Register all seven as `ThirdPartyComponentManifest`; generate `NOTICE` with Apache-2.0 modification statements.
- Vendor ai-algotrading-agent fixtures; stand up the two-implementation conformance harness (§6).
- Add `BarSampler` (time/tick/volume/dollar) to the indicator kernel spec (§7).

**Phase 1**
- `ProviderChain` with `FailoverTrigger::Empty` and per-request provider provenance in the evidence graph (§4).
- MCP tool registry split into `ReadOnly` / `RiskReducing` / `RiskIncreasing`, deny-by-default on the third (§2).
- `RiskApproved<OrderIntent>` newtype, constructible only inside the risk kernel (§2.3).

**Phase 2–3**
- `AnalystScope` with common-facts / private-evidence split; decomposed composite score; divergence as first-class output (§3).
- `AiUiIntent` typed component-request enum (§5).

**Phase 4**
- Authoring-contract triple: `get_authoring_contract` → `validate` → `save` for StrategyIR and indicators (§2.2).
- Purged + embargoed walk-forward (§7).

**Phase 5**
- Kronos sidecar. **Tokenizer-as-embedding first** (closes G06), then generative path synthesis, then forecasting (§1.2).
- Per-frequency-band baseline benchmarking; contamination policy refusing pre-cutoff scoring.

---

## 9. Attribution Checklist

Before any release containing work derived from these:

- [ ] `NOTICE` carries MIT text + copyright for Kronos (ShiYu, 2025), AIAlpha (Vivek Palaniappan, 2018), ai-algotrading-agent (Ivo Petiz, 2018), ai-trading-claude (Zubair Trabzada, 2026)
- [ ] `NOTICE` carries Apache-2.0 text + any upstream `NOTICE` for QuantDinger (Open Byte Inc.), adata, stockbot-on-groq (Vercel Inc., 2023)
- [ ] Apache-2.0 modification statements present for every adapted file
- [ ] Kronos **model weights** license verified independently of the code license
- [ ] QuantDinger trademark policy checked — the repo ships `TRADEMARKS.md`, and Apache-2.0 §6 grants no trademark rights
- [ ] Every component has a signed manifest with pinned `commit_sha` and `source_archive_hash`
- [ ] `reference/` excluded from build paths, SBOM scanning, and dependency resolution
