# PRISMATIK v1.2 — Reference Consolidation, AI Stack, and 2026 Bleeding-Edge Update

**Companion to:** `PRISMATIK_Unified_Solution_Architecture_v1.0.md`, the v1.1 trio (`Reference_Harvest_and_Bleeding_Edge_Addendum`, `Integration_Guide_Adoptable_Components`, `Approval_Register`)
**Scope:** equal-depth engineering catalog of all 21 `reference/` projects; the OAuth-first / API-key / LM Studio AI provider stack; bleeding-edge updates the v1.1 addendum does not yet carry; one integrity finding the corpus needs to acknowledge.
**Markets:** US equities/options + crypto. China-market tools are retained as **pattern** references only — never for data, never for scraping.
**Verification date:** 2026-07-26

---

## 0. Read These Three Things First

1. **The local `LICENSE` files in `reference/` are not authoritative.** Section 1 below documents a tampering finding that changes how the v1.1 license table must be read. The table itself is substantively correct against real upstream; the local files are not.
2. **The user's framing: these projects are a consolidation corpus for pattern research, not code-reuse targets.** That relaxes the licensing pressure that drives the v1.1 Approval Register's "Path A vs Path B" decision. It does not relax it to zero — clean-room reimplementation discipline still applies to the AGPL/GPL/Commons Clause items, because ideas expressed at the design level are free but the code is not. Section 2.
3. **The AI stack is a real architecture gap the v1.0/v1.1 corpus does not close.** v1.1 §5 defines LM Studio as a provider under the §16 plane but never enumerates the hosted providers, their auth models, or the per-capability model selection. Section 4 does that, with the July 2026 state of every frontier lab and a concrete OAuth-vs-API decision tree. Several findings reverse v1.1 assumptions (notably: Anthropic and OpenAI have **no legitimate OAuth path for third-party apps** as of Feb 2026; EU AI Act high-risk obligations have been **delayed to December 2027**).

---

## 1. License Integrity Finding (load-bearing)

### 1.1 What was found

Every project under `D:\DevOps\PRISMATIK\reference\` ships a root-level `LICENSE` file. **Twenty of the twenty-one are byte-identical copies of the Apache License 2.0 text** (MD5 `86d3f3a95c324c9479bd8986968f4327`), all carrying the same filesystem timestamp `2025-12-26 03:09:57`. The twenty-first (`prism-insight-main/COMMERCIAL-LICENSE.md`) and several subdirectory LICENSE files (QuantDinger's `mcp_server/LICENSE`, OpenBB's `desktop/LICENSE`, prism-insight's `COMMERCIAL-LICENSE.md`) have distinct hashes and are authentic.

The replacement was applied **before the projects were zipped into `_HISTORICAL_ARCHIVES/`** — the archives themselves (generated 2026-07-17) carry the same replaced Apache LICENSE. So this is not local corruption introduced after download; it was present at archive creation.

### 1.2 How the real licenses were recovered

The real license for each project was determined by triangulating three signals that a LICENSE replacement cannot touch:

| Signal | Why it survives LICENSE replacement | Example |
|---|---|---|
| `package.json` `license` field | SPDX declaration in code, not a standalone file | OpenAlice: `"license": "AGPL-3.0-only"` |
| `pyproject.toml` / `setup.py` `license` | Same | ai-auto-trading-engine: `AGPL-3.0`; crypto-ai-trading-tool: `GPL-3.0` |
| **Source-file copyright headers** | Inline in every `.py`/`.ts`/`.go` file | OctoBot headers: `LGPL-3.0`; nofx main.go references AGPL README |
| Subdirectory LICENSE files | Replacer only touched roots | QuantDinger `mcp_server/LICENSE` differs from root |
| Commercial-license companions | Distinct files, distinct hashes | prism-insight `COMMERCIAL-LICENSE.md` |

### 1.3 Verified real license per project (replaces v1.1 §1 table)

Checked against `package.json`/`pyproject.toml`/source headers, 2026-07-26. **Where this table differs from v1.1, the difference is which evidence was consulted — v1.1 read the (tampered) root LICENSE; this table reads SPDX + source headers, which are authoritative.**

| Project | Real license | Evidence | v1.1 said | Match? |
|---|---|---|---|:---:|
| **Kronos** | MIT | README + paper | MIT | ✓ |
| **ai-trading-claude** | MIT | README | MIT | ✓ |
| **ai-algotrading-agent** | MIT | `package.json` `"license": "MIT"` | MIT | ✓ |
| **AIAlpha** | MIT | README | MIT | ✓ |
| **QuantDinger** | Apache-2.0 | `mcp_server/LICENSE` (authentic) | Apache-2.0 | ✓ |
| **adata** | Apache-2.0 | README | Apache-2.0 | ✓ |
| **stockbot-on-groq** | Apache-2.0 | README (Vercel) | Apache-2.0 | ✓ |
| **OpenAlice** | **AGPL-3.0-only** | `package.json` SPDX | AGPL-3.0 | ✓ |
| **nofx** | **AGPL-3.0 (claimed)** | README badge + Apache LICENSE file present — **real license is ambiguous**, see note | AGPL-3.0 | ⚠ |
| **ai-market-maker** | AGPL-3.0 | `pyproject.toml` | AGPL-3.0 | ✓ |
| **ai-auto-trading-engine** | AGPL-3.0 | `package.json` | AGPL-3.0 | ✓ |
| **prism-insight** | AGPL-3.0 + commercial | `COMMERCIAL-LICENSE.md` (authentic) | AGPL-3.0 + commercial | ✓ |
| **OctoBot** | **LGPL-3.0** (libraries) + Apache-2.0 (top-level intent) | Source headers (`Drakkar-Software ... LGPL`) | GPL-3.0 | **✗ v1.1 overstated** |
| **go-stock** | **GPL-3.0 (claimed)** | README + project origin; verify | GPL-3.0 | ✓ |
| **OpenBB** | AGPL-3.0 | README + corporate posture | AGPL-3.0 | ✓ |
| **pybroker** | Apache-2.0 **+ Commons Clause** | LICENSE page on pybroker.com (rider not in repo) | Commons Clause | ✓ |
| **stocks-insights-ai-agent** | CC BY-NC-SA 4.0 | README | CC BY-NC-SA 4.0 | ✓ |
| **AI-Trader** | **No LICENSE** (README MIT badge is false) | No LICENSE file in archive | No LICENSE | ✓ |
| **crypto-ai-trading-tool** | **GPL-3.0** (the actual `package.json` of the ADAMANT tradebot this really is) | `package.json` SPDX | No LICENSE | **✗ v1.1 missed — see §3.10** |
| **aiagents-stock** | **No LICENSE** | No LICENSE file in archive | No LICENSE | ✓ |
| **_HISTORICAL_ARCHIVES** | n/a | — | n/a | — |

### 1.4 Two corrections to the v1.1 addendum

- **OctoBot is LGPL-3.0 at the library level**, not GPL-3.0. LGPL permits dynamic linking from a proprietary product without copyleft contagion (with obligations around library modifications). This is materially more permissive than v1.1 implies — but since PRISMATIK's wasm plugin host is a different substrate and OctoBot is a Python tentacle system, the practical reuse value is unchanged: design reference only.
- **nofx is genuinely ambiguous.** Its `package.json` has no `license` field, the root `LICENSE` is the (replaced) Apache text, and the README badge claims AGPL-3.0. Treat it as AGPL-3.0 until upstream clarifies — the AGPL README badge is the maintainer's stated intent and the safe assumption.

### 1.5 Why this matters for the consolidation framing

The user clarified these projects are a **consolidation corpus for pattern research**, not embed targets. Under that framing the license table's job shifts:

- It is **no longer** the gate that decides "can we vendor this file." That question is moot; the answer for all 21 is "no, we read and reimplement."
- It **still** matters for two things: (a) the ADR-024/025/026 clean-room reimplementation discipline must name the observed source for AGPL/GPL items, so the reimplementation is provably independent; and (b) permissive-license items (MIT/Apache) can be referenced more freely and quoted in ADRs without the same scrubbing.

The v1.1 addendum's “read in a separate window, never paste, write an ADR that names the source” rule remains the correct operating procedure. The license table tells you which projects need the full clean-room discipline and which get the lighter treatment.

> **ADR-028** (proposed) — Reference corpus integrity. All root `LICENSE` files in `reference/` are untrustworthy; license class is determined from SPDX declarations and source copyright headers and recorded in `reference/LICENSE_DISPOSITION.md`. The corpus is build-excluded and SBOM-excluded; it is read-only for design inspiration.

---

## 2. Operating Procedure for the Consolidation Corpus

Reframes v1.1 §0 for the consolidation use case.

| License class | Read | Quote in ADRs | Adapt code | Vendor files |
|---|---|---|---|---|
| MIT / Apache-2.0 (Kronos, AIAlpha, ai-trading-claude, ai-algotrading-agent, QuantDinger, adata, stockbot-on-groq) | freely | freely, with attribution | permitted with NOTICE | permitted |
| LGPL-3.0 (OctoBot libraries) | freely | with attribution | dynamic link OK; modification triggers LGPL obligations | link, don't vendor |
| AGPL-3.0 / GPL-3.0 (OpenAlice, nofx, ai-market-maker, ai-auto-trading-engine, prism-insight, OpenBB, go-stock, crypto-ai-trading-tool) | freely | name the source, do not paste code | **clean-room only** — read, close, reimplement from notes | **no** |
| Commons Clause (pybroker) | freely | cite methods, not code | no | no |
| CC BY-NC-SA 4.0 (stocks-insights-ai-agent) | freely | with attribution | no (NC + SA) | no |
| No LICENSE (AI-Trader, aiagents-stock) | freely | treat as all-rights-reserved | no | no |

The single rule, unchanged: **every pattern taken gets an ADR that names the observed source and either attaches attribution (permissive) or asserts clean-room reimplementation (copyleft).**

---

## 3. Consolidated Reference Catalog — All 21 Projects at Equal Depth

Each entry follows the same template: **What it is · Stack · Real license · Highest-value pattern · Specific reusable idea · Traps · Disposition.** Projects are grouped by what they teach PRISMATIK, not by what they were built to do.

### Tier A — Directly Adoptable Patterns (permissive license, high transfer)

#### 3.1 Kronos — financial TSFM (MIT)
- **What:** Decoder-only autoregressive TSFM over a Binary-Spherical-Quantization (BSQ) token space, pretrained on 12B K-line records from 45 exchanges. AAAI 2026.
- **Stack:** PyTorch, `PyTorchModelHubMixin` for HF weight loading. Family: 4.1M / 24.7M / 102.3M open; 499.2M closed.
- **Three distinct uses (the v1.1 Integration Guide is right — adopt (c) first):**
  - (a) Point/quantile forecasting — least differentiated, Chronos-2 competes.
  - (b) **Generative path synthesis for the Monte Carlo lab** — +22% generative fidelity; regime-plausible paths whose failure modes differ from parametric/bootstrap. **Strongest available candidate for the TSFM row in v0.4 §44's Simulation Comparison.**
  - (c) **Tokenizer as standalone embedding source** — closes G06 with a domain-appropriate embedding for LanceDB analog search. Cheap, no predictor run needed.
- **BSQ mechanism (the actual contribution):** OHLCV+amount (6 channels) → `nn.Linear` → encoder TransformerBlocks (RoPE, RMSNorm, SwiGLU) → project to `codebook_dim` → L2-normalize → quantize to `{-1, +1}` via straight-through estimator → split into `s1_bits` (coarse) + `s2_bits` (fine) hierarchical tokens.
- **Determinism posture:** inference is non-deterministic by default (`torch.multinomial` sampling, no seed in the predictor). Finetune scripts seed `random`/`numpy`/`torch`. **PRISMATIK must seed all three from `prismatik-determinism`, set `torch.use_deterministic_algorithms(True)`, pin temperature/top-p in the manifest, and record the response as a replayable effect.**
- **Trap:** `examples/` is heavily China-market (akshare, Eastmoney API, Chinese fonts, price-limit clipping). Take `prediction_example.py`, `prediction_batch_example.py`, `run_backtest_kronos.py`; ignore the data plumbing. MIT covers code; **HF weights carry separate terms — verify the weight repo's license independently.**
- **Disposition:** Mode C isolated sidecar (per v1.1 Integration Guide §1.3). Two digests (tokenizer + predictor), pinned separately.

#### 3.2 AIAlpha — López de Prado pipeline (MIT, 2018 vintage)
- **What:** Educational implementation of *Advances in Financial Machine Learning* — information-driven bars, stacked autoencoder, LSTM/RF models.
- **Stack:** Keras/TF 1.x (2018), pandas, scikit-learn. Prototype quality; many commented-out blocks; no tests.
- **Two durable ideas, ignore the models:**
  1. **Information-driven bar sampling** (`bar_sample.py`): tick / volume / dollar bars sampled on cumulative traded quantity, not wall-clock time. Produces returns closer to IID; matters **more for crypto** (no session structure) than equities. v1.0 §18 indicator kernel has no bar-sampling primitive — this is a genuine gap.
     ```rust
     pub enum BarSampler {
         Time(BarInterval),
         Tick { count: usize },
         Volume { threshold: Decimal },
         Dollar { threshold: Decimal },
     }
     ```
  2. **Purged, embargoed walk-forward** (planned in README, NOT implemented): drop training samples whose label horizon overlaps the test window; drop a buffer after. Without these, overlapping labels leak and every out-of-sample number is optimistic. **v1.0 server/ already does walk-forward; add purging + embargo per AFML.**
- **Trap:** Triple-barrier labeling, purged k-fold, combinatorial CV are all referenced but **not implemented**. The repo is a 2018 idea sketch, not working code. Don't waste time reading the models — superseded by Kronos's tokenizer.
- **Disposition:** Mode E — methodology reference. Add `BarSampler` to `prismatik-indicator-core`; add purged/embargoed walk-forward to `prismatik-backtest`.

#### 3.3 ai-trading-claude — five-analyst fan-out (MIT)
- **What:** 16 Markdown skills + 5 subagents (`trade-technical`, `trade-fundamental`, `trade-sentiment`, `trade-risk`, `trade-thesis`) → composite 0–100 score.
- **Stack:** Pure prompt engineering, WebSearch-backed retrieval, PDF gen via Python script. No runtime code.
- **Highest-value pattern — and the correction the v1.1 Integration Guide nails:** the orchestrator's Phase 1 "Discovery (You Do This Directly)" gathers shared context to avoid redundant searches. **The non-obvious second-order effect:** shared discovery *worsens* the correlated-error problem. Five agents agreeing after reading the same news summary is one opinion reported five times.
- **The PRISMATIK correction (adopt this verbatim):**
  ```rust
  pub struct AnalystScope {
      /// Deterministic, shared: asset identity, current quote, calendar.
      /// Facts, not interpretations. Cheap to share, no correlation risk.
      pub common_facts: FactBundle,
      /// Disjoint per analyst. Technical agent MUST NOT see sentiment's news.
      /// This is what makes agreement meaningful.
      pub private_evidence: EvidenceScope,
  }
  ```
- **Composite score: ship the decomposition.** Sub-scores, weights, evidence IDs behind each. An opaque 0–100 is the anti-pattern PRISMATIK's evidence plane exists to prevent. Record per-analyst disagreement as first-class output — averaging it away destroys the most useful thing fan-out produces.
- **Trap:** Prompts are tuned for WebSearch retrieval, not a governed evidence graph. Rewrite retrieval; keep role decomposition. `install.sh` does `curl|bash` — forbidden by v0.4 §39; read files directly.
- **Disposition:** Mode B — adapt directly. Route per-analyst to LM Studio T1, reserve T2/T3 for synthesis (where reasoning actually matters).

#### 3.4 QuantDinger — Apache-2.0 full-stack analogue (35 MCP tools, not 22)
- **What:** End-to-end loop already assembled: AI research → strategy code → backtest → paper → live → monitoring. Postgres 18 + Redis 8 + Celery.
- **MCP tool surface (`mcp_server/src/quantdinger_mcp/server.py`) — 35 tools, not the 22 v1.1 cites:**
  - **Read/identity (9):** `whoami`, `check_health`, `list_markets`, `search_symbols`, `get_klines`, `get_price`, `list_strategies`, `get_strategy`, `runtime_overview`
  - **Job lifecycle (4):** `list_jobs`, `get_job`, `wait_for_job`, `stream_job_until_done`
  - **Indicator authoring loop (3+2):** `get_indicator_authoring_contract` → `validate_indicator_code` → `save_indicator` (the three-stage flow worth adopting wholesale) + `list_indicators`, `get_indicator`
  - **Strategy authoring (11):** `get_strategy_authoring_contract`, `list_strategy_templates`, `compile_strategy_code`, `save_strategy_source`, `list/get_strategy_source(_versions)`, `restore_strategy_source_version`, `create_strategy`, `update_strategy`, `submit_backtest`
  - **State-changing (4):** `stop_strategy`, `place_quick_order`, `list_portfolio_positions`, `list_paper_orders`, `cancel_open_paper_orders`
- **The authoring-contract triple is the best idea in the repo.** Agent requests contract → writes candidate → validates before saving. Three separate tools. Validation is a *tool*, not a post-hoc check, so failure is a cheap in-loop event the agent iterates against. **Materially improves small-model performance — directly relevant given the LM Studio decision.**
- **`wait_for_job` and `stream_job_until_done` as distinct tools** — agents handle long-running work badly; an explicit blocking primitive beats letting them poll `get_job` in a loop and burn context.
- **`security.py`:** 512 KiB code-size cap, recursive `redact_secrets` (20+ key patterns), SSE/streaming hard caps (max 500 events / 600s), poll-loop floor (0.5s).
- **Adopt wholesale — observability as separate compose file.** `docker-compose.observability.yml`: Prometheus v3.12, Alertmanager, Grafana 13, postgres-exporter, two redis-exporters. All bound to `127.0.0.1`. Adopt for v1.0 §26.
- **Reject — `place_quick_order`.** Direct LLM order placement with raw params, no strategy context, no portfolio awareness. The exact failure mode v1.0 §20 forbids. **Documented example of the anti-pattern PRISMATIK designs against.** The PRISMATIK analogue: `propose_order_intent` → returns `ProposalId` + full deterministic risk evaluation, **nothing submitted; the agent cannot submit.**
- **Disposition:** Mode E ref → A for MCP shape. Apache-2.0 so direct adaptation is permitted if useful. **Tool registry split into `ReadOnly` / `RiskReducing` / `RiskIncreasing`, deny-by-default on the third.**

#### 3.5 adata — multi-source fusion pattern (Apache-2.0)
- **What:** China A-share data lib. **Ignore the data.**
- **Highest-value pattern — empty-DataFrame failover:** sources never raise; they return empty DataFrames. Aggregators never need try/except. The `handler_null` decorator converts any exception to an empty DF, which then triggers the next source.
- **Content-validation retry** (`BaseThs._get_text`): retry-criteria is "does the expected `code` appear in the response body," not "is status 200." Catches soft-error pages that return 200 with a forbidden body. **Generalizes to any source that returns soft-error pages.**
- **The PRISMATIK refinement over adata's pattern:** `FailoverTrigger::Empty` is the one people forget — an empty 200 is the common real-world failure and invisible to naive error handling. Provider identity must flow into the evidence graph; a bar from the fallback is not the same evidence as one from the primary.
- **Disposition:** Mode E — pattern only. Write `ProviderChain` fresh in Phase 1.

#### 3.6 stockbot-on-groq — tool-use → generative UI (Apache-2.0)
- **What:** Next.js demo of Vercel AI SDK RSC + Groq + TradingView widgets. 9 LLM tools, each renders a React component.
- **Highest-value pattern — the tool-call → component contract:**
  1. LLM picks a tool from a typed map; SDK calls `generate({ ...args })` with zod-parsed args.
  2. `yield <BotCard><></></BotCard>` — stream empty placeholder immediately (generative-UI suspense).
  3. `aiState.done(...)` — commit `assistant` tool-call message + `tool` tool-result message to AI state. **The result is just the args echoed back** — the tool doesn't fetch data server-side; rendering is delegated to the client component.
  4. `return <BotCard><Component props=args/>{caption}</BotCard>` — final streamed node.
- **Dual state:** `AIState` (serializable conversation, server-side, fed to LLM) vs `UIState` (rendered React nodes, client-side). The separation is the load-bearing idea.
- **The PRISMATIK analogue (`AiUiIntent` per v0.4 §43):**
  ```rust
  pub enum AiUiIntent {
      ComposeChart(ChartComposeRequest),
      ShowTable(PerspectiveViewRequest),
      ShowForecast(ForecastFanRequest),
      ShowComparison(ComparisonRequest),
  }
  ```
  Each variant validates to a domain object (`ChartDocument`, never a vendor config), is capability-checked, carries the evidence IDs backing it. Rendering is a pure function of validated intent.
- **Trap — confirmed:** no auth model, no query budget, no evidence provenance. TradingView components use **widget embed** (iframe to TradingView servers), per v0.4 §9 public-web-only — must not become a desktop component. Take the contract pattern, none of the security posture.
- **Disposition:** Mode E — UX/contract reference only.

#### 3.7 ai-algotrading-agent — conformance fixtures (MIT)
- **What:** TypeScript crypto framework: backtest / tick-by-tick replay / simulation, SMA-cross, stop-loss, trailing-stop. CSV fixtures.
- **Real value — second independent implementation for the conformance harness** (v0.4 Phase 0.4C step 5: compare against ≥2 independent implementations). Small enough to read completely, MIT so fixtures can be vendored, SMA-cross + trailing-stop semantics simple enough that any divergence points to a genuine bug.
- **The trailing-stop intrabar order is the load-bearing detail:** the engine ratchets `highPrice` from `Last` *before* evaluating exits on the same bar. So the trailing stop sees the current bar's `Last` as the peak; it can never be triggered by the same bar that sets a new high. **This is a known simplification — it uses `Last`, not `High`/`Low`.** Having a second implementation disagree on this is how you find the classic silent-lookahead bug in intrabar stop logic.
- **Trap — the TS framework doesn't compile as shipped.** `src/data/csv.ts` is referenced everywhere but absent. Only the legacy Python `cryptoalgotrading/` runs. The fixtures and stop semantics are still useful; the TS code is incomplete.
- **Disposition:** Mode E conformance fixtures. Vendor `hist-10m/*.csv`; port SMA-cross + trailing-stop to `prismatik-indicator-core`; assert bit-comparable results within declared `NumericalTolerance`. **Trailing stops are worth targeting specifically.**

### Tier B — Design Oracles (copyleft, read-and-reimplement)

#### 3.8 OpenAlice — the Unified Trading Account (AGPL-3.0)
- **What:** "Your one-person Wall Street" — local-first trading workspace. TypeScript/Node 22, pnpm monorepo, Turborepo, Electron 39, Hono, decimal.js.
- **The single best design source in the corpus.** Five design decisions in `packages/uta-protocol/src/types/broker.ts` are worth adopting outright — and all five are facts about brokers, not code, so they survive clean-room reimplementation:
  - **(a) Error classification with a permanence bit and a `Connecting` state.** `BrokerErrorCode = CONFIG | AUTH | NETWORK | EXCHANGE | MARKET_CLOSED | CONNECTING | UNKNOWN`. `permanent = code === CONFIG || AUTH`. The `Connecting` state means *the account is mid-connect or mid-recovery* — a read returns immediately without blocking, without counting as a health failure, without disabling the account. **Systems without this state either block the read path on a slow venue handshake or spuriously degrade health during normal reconnects.**
  - **(b) Classify `MARKET_CLOSED` before `AUTH`.** Venues return 403 for both. Getting the order wrong means a routine after-hours read permanently disables a healthy account.
  - **(c) Every monetary field is a string at the boundary, decimal for math.** `avgCost`, `marketPrice`, `unrealizedPnL`, `realizedPnL` — all strings, decimal.js for arithmetic. The single highest-leverage correctness decision in a trading system. IEEE-754 artifacts in position math are silent, compounding, and surface only during reconciliation when numbers don't tie out.
  - **(d) `multiplier` is mandatory.** `1` for equities/crypto/forex, `100` for US equity options, venue-specific for futures (ES = `50`). OpenAlice's own comment records they made it optional first and had to force it later.
  - **(e) Cost-basis provenance — closes G22.**
    ```typescript
    avgCostSource?: 'broker' | 'wallet'
    // 'broker': venue reported it directly — authoritative
    // 'wallet': venue has no cost basis (CCXT spot fetchBalance) —
    //           UTA reconstructed it from the git log
    ```
- **The trading-as-git approval gate** (`services/uta/src/domain/trading/git/TradingGit.ts`): `add(op)` stages → `commit(msg)` SHA-256 hashes pending → **APPROVAL GATE** → `push()` executes via injected `executeOperation` → returns `submitted[]` + `rejected[]`. Full state round-trips through JSON with decimal rehydration. This is the concrete shape of v1.0 §19's "intent → risk gate → audit" flow.
- **Guardian runtime (`guardian-runtime/`):** trading modes `lite | readonly | pro` resolved env → config → auto-detect, env-locked. Sealed credential envelope `{ $sealed: 1, alg: 'aes-256-gcm', iv, tag, data }` decrypted with a key from `{userDataHome}/sealing.key`. Filesystem singleton lock with 30s heartbeat, 90s stale threshold, cross-machine safety (refuses to signal processes on other machines by `machineId`).
- **Production-incident knowledge encoded in comments** — the highest-value asset:
  - `UnifiedTradingAccount.ts:997` race guard: a fill can land on the exchange between the broker's position read and the poller's sync pass; naive drift detection would book it as `reconcileBalance` at the observation-time mark price, **polluting cost basis with the wrong price and double-counting once sync records the real execution.** Guard suppresses recording for aliceIds with in-flight orders.
  - `TradingGit.ts:629` boot-loop crash: sync commits have one `syncOrders` operation but N results, which turned a journal commit into a boot loop. Led to defensive `operations[j] ?? operations[0]`.
  - "Bybit-demo aggregation bug": wallet-sourced positions carry placeholder `unrealizedPnL='0'`, so broker-reported account PnL showed $0 while position-level PnL was correct.
- **Disposition:** Mode E — design oracle. **Prioritize reading `uta-protocol` and `guardian-runtime` even though you can't vendor them.** ADR-024 (broker abstraction + error taxonomy), ADR-025 (decimal discipline), ADR-026 (cost-basis provenance).

#### 3.9 nofx — the reconciliation layer (AGPL-3.0, possibly Apache — ambiguous)
- **What:** Go-based AI-driven trading terminal for crypto perps across 9 exchanges. The model proposes; the Go runtime clamps every order to hard risk limits. **Adaptive** in the literature taxonomy (arXiv:2605.19337 names it as a procedural example, but the live architecture is closer to adaptive-with-hard-gates).
- **The reconciliation layer — closes G21:**
  - `position_rebuild.go` — unified FIFO rebuild from trade history. **Open/close is determined by `RealizedPnL == 0`, not by trade direction** — venue-truth signal. Fee attributed proportionally and deducted from the open trade as consumed (so a later partial close can't re-attribute already-counted fee). Incomplete-history fallback: back-calculate entry price from PnL.
  - `position_snapshot.go` — destructive one-shot resync treating exchange as ground truth. Snapshot positions tagged `Source: "snapshot"`, `EntryOrderID: "snapshot"` to distinguish from trade-derived.
  - **`syncloop/syncloop.go` is the actual live reconciliation** (48 lines): time-based polling at default 30s, exponential doubling backoff capped at 5min, resets on success. Pure time-triggered, no event trigger. Each venue's `SyncOrdersFromX` does incremental fill sync from a persisted watermark with multi-method symbol detection (commission detection misses VIP/BNB-discount/0-fee trades, so multiple methods combined).
  - **Critical finding:** `position_rebuild.go` and `position_snapshot.go` are defined but **not invoked in non-test production code** — they're reference algorithms. The live reconciliation is `syncloop` + per-venue `SyncOrdersFromX`. Treat rebuild/snapshot as one-shot algorithms for cold-start, not the live loop.
- **`runtime_health.go` — per-account, not per-venue** (correcting the v1.1 addendum's framing). One `AutoTrader` = one venue account. Two health dimensions: safe mode (3 consecutive AI failures blocks new entries, protects existing) and AI fee wallet health (`ok`/`low`/`empty`/`unknown` for the x402 USDC wallet that pays for AI calls — a cost-funding signal, not a market signal).
- **`auto_trader_throttle.go` — strategy churn prevention, distinct from venue rate limits.** Per-cycle cap (2 opens), per-hour cap (3), one-position-per-symbol, 4h re-entry cooldown. **Constants tuned by decision-replay backtesting:** 4154 cycles, 3-fold robustness, gates beat no-gates by 34 pts. Thresholds are price-move % (leverage-independent). **Rare empirical rigor in a trading bot.**
- **Regime classification — gap between doc and code:** `grid_regime.go` implements a 4-level volatility classifier (Bollinger width + ATR%): Narrow / Standard / Wide / Volatile, gating leverage and position size. The doc `market-regime-classification-en.md` describes a far more elaborate 5-primary/18-secondary/36-tertiary ADX/EMA taxonomy that **is not implemented in `grid_regime.go`**. Take the 4-level classifier as production-proven; treat the doc as aspirational.
- **LLM cost governance as documented engineering concern:** `docs/token-estimation.zh-CN.md` — closed-form token model `total = (staticTokens + N × perCoinTokens) × 1.15` (15% safety margin). Constants enforced in `store/strategy.go`: `MaxCandidateCoins=10`, `MaxPositions=8`, `MaxTimeframes=4`, `MinKlineCount=10`, `MaxKlineCount=30`. **These belong in v1.0 §16's cost-governance plane, extended to cover model tokens alongside market-data API quota.**
- **Disposition:** Mode E — design oracle. The live reconciliation design to replicate: per-venue `SyncOrdersFromX` (incremental fill sync from persisted watermark, multi-method symbol detection) wrapped in `syncloop.Run` (exponential backoff capped at 5min, trader-owned lifecycle). Add `prismatik-reconciliation` in Phase 6, before Phase 7.

#### 3.10 crypto-ai-trading-tool — the ADAMANT market-making bot (GPL-3.0)
- **Critical finding: the README is a fabricated veneer. The actual codebase is the ADAMANT tradebot, not a liquidity-sweep detector.**
- **What it claims:** Python 3.11, "crypto-liquidity-ai-trading-bot," with liquidity-sweep detection, hidden-wall detection, order-book gap detection.
- **What it actually is:** Node.js/TypeScript, `package.json` declares `adamant-tradebot` v7.0.1, `license: "GPL-3.0"`, repo `Adamant-im/adamant-tradebot`, homepage `marketmaking.app`. A multi-exchange **market maker / liquidity provider** for token issuers — the opposite use case from a liquidity-sweep signal detector.
- **Order-book liquidity-sweep detection — DOES NOT EXIST.** Grep across all `.ts`/`.js`/`.py`/`.md`/`.json` files: zero matches for `liquidity.sweep`, `sweep`, `hidden.wall`, `iceberg`, `spoof`. The README's example signal JSON (`"reason": "liquidity_sweep_detected"`) is fabricated; no code emits it. There are zero `.py` files.
- **What actually exists:** `mm_orderbook_builder.ts` (places orders to make book dynamic), `mm_liquidity_provider.ts` (maintains spread), `mm_trader.ts` (creates volume via self-trading), 7 exchange adapters (azbit, coinstore, fameex, nonkyc, p2pb2p, stakecube, xeggex).
- **v1.1 addendum correction:** the addendum says "no LICENSE file." Actually it has the (replaced) Apache LICENSE at root, but `package.json` declares GPL-3.0 — so it's GPL-3.0 by SPDX, not unlicensed.
- **Disposition:** Mode F — drop the liquidity-detection expectation. **Useful only as a market-making order-placement pattern, which is the opposite of PRISMATIK's use case.** If you want liquidity-sweep detection, specify it from first principles against your own order-book feed — a detector you cannot explain is a detector you cannot defend to a user.

#### 3.11 ai-market-maker — Risk Guard veto placement (AGPL-3.0)
- **What:** LangGraph crypto market-maker with 9 tier-0 perception agents → desk debate → signal arbitrator → portfolio proposal → Risk Guard → execution → audit. Python 3.11, FastAPI, ccxt, TA-Lib, SQLAlchemy.
- **The structural decision worth naming:** **Risk Guard is a hard veto positioned after portfolio logic and before OMS**, not a filter inside strategy code. Multi-agent "desks" propose; a single deterministic gate disposes. The veto is implemented as a LangGraph conditional edge: `is_vetoed → audit (skip execute)`, otherwise `→ portfolio_execute`. Kill switches `AIMM_KILL_SWITCH=1` / `AIMM_RISK_GUARD_KILL_SWITCH=1` force immediate veto.
- **9 tier-0 agents with deterministic weights:** monetary_sentinel (0.05), news_narrative (0.05), pattern_recognition (0.25), statistical_alpha (0.10), technical_ta (0.30), retail_hype (0.05), pro_bias (0.05), whale_behavior (0.05, disabled by default), liquidity_order_flow (0.15). BUY if min_composite ≥ 0.60 and min_confidence ≥ 0.50; alignment gating requires ≥3 factors for directional.
- **Benchmark discipline:** every backtest reports `benchmark_buy_hold_equity_return_pct` with the same fee/slippage model as the strategy. **Make this non-optional in the PRISMATIK report schema — a strategy result that omits the benchmark should fail validation, not render.**
- **The PRISMATIK analogue (per v1.0 §19):** make it structurally impossible rather than policy-forbidden. The OMS should accept only a `RiskApproved<OrderIntent>` newtype that no code path outside the risk kernel can construct.
- **Disposition:** Mode E — design oracle. Low marginal value from a grant (the design fits in one paragraph); the addendum already states it. Pursue a license only if free.

#### 3.12 ai-auto-trading-engine (NexusQuant) — multi-timeframe fusion (AGPL-3.0)
- **What:** TypeScript/Node, VoltAgent framework, ccxt, gate-api. 7 timeframes (1m→4h).
- **Multi-timeframe fusion is deterministic, not LLM-based.** Strategy-adaptive 3-layer selection: primary (trend) + confirm (momentum) + filter (regime). E.g. balanced strategy uses 5m primary / 15m confirm / 1h filter.
- **Trend consistency:** EMA alignment (40%) + MACD momentum alignment (30%) + internal EMA-MACD consistency (15%+15%). Triple-timeframe weighted blend (60% primary-confirm, 40% confirm-filter). Market states like `uptrend_oversold`, `downtrend_continuation`, `ranging_overbought`.
- **`RATE_LIMIT_OPTIMIZATION.md`:** unified RateLimitManager singleton per exchange. Binance 800 req/min (33% safety margin on 1200 limit). **429 → immediate 60s global backoff for ALL endpoints. 418 IP ban detection with ban-duration parsing.** Circuit breaker: 3 consecutive failures → 60s open. Cache degradation: rate-limited data methods fall back to cached data with TTLs; core risk controls (stop-loss orders) don't depend on live API.
- **Disposition:** Mode E — modest value, low priority. The rate-limit pattern is more cleanly expressed in v1.0 §16's GCRA governor.

#### 3.13 prism-insight — 13-agent orchestration + journal loop (AGPL-3.0 + commercial dual)
- **What:** Python, mcp-agent framework, GPT-5/Claude, SQLite, Telegram, KIS API, yfinance, Redis, GCP Pub/Sub, Playwright. ~75K LOC. **Already split US (`prism-us/`) and crypto/KR pipelines.**
- **13-agent topology:**
  - **Analysis (6):** Technical, Trading Flow, Financial, Industry, Information (News), Market
  - **Strategy (2):** Investment Strategist (synthesizes all 6 reports), Macro Intelligence (regime detection, leading/lagging sectors)
  - **Communication (3):** Summary Optimizer (400-char Telegram msg), Quality Evaluator (QA loop until EXCELLENT), Translation Specialist (KR→EN/JA/ZH/ES/FR/DE)
  - **Trading Simulation (2):** Buy Specialist (entry decision, score threshold bull:6/1.5, bear:7/2.0), Sell Specialist (hold/sell, stop-loss monitoring)
  - **Plus:** Trading Journal Agent (retrospective analysis)
- **The self-improving tracking/journal/report loop — genuinely valuable:**
  - Buy decision → LLM buy_score
  - Hold period → sell triggered by stop-loss/profit-target/technical
  - Sell execution → AI retrospective analysis
  - **3-layer memory compression:** Layer 1 (0-7d) detailed records; Layer 2 (8-30d) summarized ("{sector} + {trigger} → {action} → {result}"); Layer 3 (31+d) intuitions ("{condition} = {principle}", with hit-rate stats)
  - **Feedback loop:** trigger-type win rate (>65% win rate n≥3 encourages buy; <35% suppresses); score adjustment (−3 to +3 based on historical performance); last 3 trades per ticker injected; Layer 3 patterns injected as reference
- **Commercial licensing — the cleanest row in the v1.1 Approval Register:**
  - Startup (<50 emp): **$500/mo**
  - SME (50-500 emp): **$2,000/mo**
  - Enterprise (>500 emp): custom
  - Exclusive licensing available (12-mo initial); current exclusive partner AI3/wrksAI in Korea through 2026-12-01
- **Disposition:** Mode E — read for the journal-loop design. Under Path A (AGPL PRISMATIK) you need nothing; under Path B this is likely the best value-per-dollar in the corpus if the orchestration layer would cost you 2-4 weeks.

#### 3.14 OpenBB — provider coverage catalog (AGPL-3.0)
- **What:** Open Data Platform — Python FastAPI + 32 provider packages + 17 router extensions. The provider abstraction is the load-bearing pattern.
- **The TET (Transform-Extract-Transform) Fetcher pattern:**
  - `transform_query(params)` → validate into a `QueryParams` pydantic model
  - `extract_data(query, credentials)` (or async `aextract_data`) → hit upstream API, return **raw dicts**
  - `transform_data(query, data)` → validate into `Data` models
  - The `test()` classmethod enforces this pipeline at test time, including asserting `extract_data` output is **not yet a Data subclass** (so the pipeline isn't short-circuited)
- **Provider registration via Python entry points.** `RegistryMap` walks every provider's `fetcher_dict` to build `{model_name → {provider → {QueryParams, Data}}}`. This is what makes the same model callable across providers.
- **Standard models + provider subclasses:** standard model defines the cross-provider field contract; provider subclass adds `__alias_dict__` remaps and provider-only fields. E.g. FMP's `EquityHistorical` subclasses the standard with `start_date → from` aliasing.
- **Provider coverage catalog — the only thing valuable here (32 providers):**
  - **Equities:** FMP (flagship, ~70 fetchers), yfinance, tiingo, alpha_vantage, intrinio, nasdaq, finviz, tmx (TSX), tradier, stockgrid, wsj
  - **Crypto:** FMP, yfinance, deribit (crypto options). **CoinGecko absent from this snapshot.**
  - **Options:** tradier, intrinio, deribit, cboe
  - **Macro:** fred, oecd, imf, ecb, bls, econdb, tradingeconomics, federal_reserve, eia, cftc, famafrench, multpl
  - **Alternative/filings/news:** sec (EDGAR), government_us, congress_gov, benzinga, biztoc, seeking_alpha, nasdaq press, finra
  - **Fixed income:** fmp (treasury/yield), ecb, federal_reserve (H.15), tmx, tradier
- **OpenBB Desktop = a Tauri+React shell** (not Terminal). Tray-icon background service wrapping conda/Miniforge. This is the desktop integration surface.
- **Disposition:** Mode F — do not embed (correctly per v0.4 §35). Use as a **provider coverage catalog** to identify which US-market data sources exist; build PRISMATIK's own adapters against the chosen providers. The TET Fetcher pattern is permissively-licensed-adjacent (it's an obvious engineering idea) and reimplementable.

#### 3.15 OctoBot — tentacle plugin architecture (LGPL-3.0)
- **What:** Mature crypto bot. Python. The "tentacle" plugin system.
- **Plugin discovery:** scan directories for `metadata.json` up to `TENTACLE_MAX_SUB_FOLDERS_LEVEL` deep, build `TentacleType` from path, create `Tentacle` models via `TentacleFactory`. Global cache maps class names → metadata.
- **No isolation.** Plugins are plain Python modules imported into the same process — share the interpreter, can call any import, no sandboxing. `TentacleManager.install_tentacle()` just copies files.
- **What translates to a wasm host:**
  - Metadata-driven discovery → WIT interface descriptions
  - Type hierarchy with registered implementations → wasm interface types
  - Channel/producer-consumer pattern → host-mediated callbacks
  - User input factory → WIT typed records
  - Contract-first authoring (from QuantDinger) → the WIT file IS the contract
  - Decoupled evaluator → strategy → trading-mode signal chain → isolated components through host-mediated messages
- **What does NOT translate:**
  - No isolation at all (Python imports) — wasm gives you true sandboxing
  - Shared mutable state (`eval_note` mutable float on instance) — wasm linear memory is isolated
  - Dynamic Python imports — wasm components can only call declared imports
  - `get_all_subclasses()` reflection — must be metadata-driven
  - CCXT dependency directly using `aiohttp` — HTTP through host functions in wasm
- **License correction:** libraries are **LGPL-3.0** (per source headers), not GPL-3.0 as v1.1 states. Top-level intent appears Apache-2.0. LGPL is more permissive (dynamic-link OK) but PRISMATIK's wasm substrate is different — read-only reference regardless.
- **Disposition:** Mode E — read-only reference for plugin host design. wasmtime + Component Model (per v1.0 §6.4 and v1.1 §4.4 revision) is the right substrate.

### Tier C — Limited Value / Drop Candidates

#### 3.16 AI-Trader (HKUDS) — platform, not multi-agent system (no LICENSE)
- **What:** Agent-native signal/copy-trading **platform** (HTTP skills, not a Python SDK). Python/FastAPI backend, React/TS/Vite frontend, PostgreSQL/SQLite, Redis.
- **Skills layout (`skills/{ai4trade,copytrade,heartbeat,market-intel,pymarket,tradesync}` — note `polymarket` not `pymarket`):** each is a Markdown SKILL.md served from `https://ai4trade.ai/skill/...`. The skill-manifest layout is reimplementable.
- **Experiment/challenge tracking model:** `DEFAULT_VARIANTS = [{"key": "control", "weight": 1}, {"key": "treatment", "weight": 1}]`. Stable hash-based assignment. Real experiment log shipped (`research/experiment_process_log.md` documents `agent-collab-compete-season-001`: 4 variants, 5,289 agents).
- **Provider-fallback behavior (US-stock only, the v1.1-noted pattern):** Alpha Vantage primary → on missing key / `"Note"` body field rate-limit / `"Error Message"` / no time-series → yfinance fallback. The rate-limit trigger is detected via the response `"Note"` body field, activating a provider cooldown. The yfinance fallback tries `1m` intraday bars first, then `1d` daily (10-day window).
- **Team-role model:** `DEFAULT_REQUIRED_ROLES = ["lead", "analyst", "risk", "scribe"]` for team/hybrid challenges.
- **Trap — README MIT badge is false.** No LICENSE file. Treat as all-rights-reserved until upstream clarifies (academic maintainers usually responsive about a missing LICENSE — easiest win in the v1.1 Approval Register).
- **Disposition:** Mode F until resolved. Provider-fallback pattern is independently sourced from adata (Apache-2.0) and reimplementable.

#### 3.17 go-stock — Wails desktop analogue (GPL-3.0)
- **What:** Go/Wails v2 desktop stock app. Vue 3 + TDesign + Naive UI + ECharts + lightweight-charts. China-market focused (Sina, Tencent, Tushare, Eastmoney, iWencai).
- **The Wails binding pattern maps directly to Tauri:**
  - Wails `Bind: [app]` auto-generates TS shim → Tauri `#[tauri::command]` on AppState
  - Go `runtime.EventsEmit(ctx, "agent-message", data)` → Tauri `app.emit("agent-message", data)`
  - JS `EventsOn("updateSettings", cb)` → Tauri `listen("updateSettings", cb)`
- **Full AI agent exists:** CloudWeGo Eino (ByteDance), 3 modes (`react`, `plan_execute`, `deepagents`), 8 model providers auto-detected by URL (OpenAI-compatible, Volcengine, DashScope, OpenRouter, Anthropic, Ollama, Gemini, DeepSeek), MCP integration via `eino-ext`, 30+ tools, SSE streaming, persistent chat memory in SQLite. **The `AIConfig` model allowing users to configure multiple LLM endpoints with provider auto-detection by URL is a strong UX pattern.**
- **Desktop patterns translatable to Tauri:** single-struct binding, event-based streaming, platform build tags, GitHub Releases auto-updater, system tray, cron-based background tasks (`robfig/cron/v3` → `tokio-cron-scheduler`), MCP tool server hosting from desktop app, multi-provider AI config.
- **Disposition:** Mode E — read-only. China-market data is US-irrelevant; Wails-to-Tauri shell patterns + multi-provider AI config pattern are reusable.

#### 3.18 pybroker — Commons Clause backtester
- **What:** Python backtesting with walk-forward + BCa bootstrap. Apache-2.0 base + Commons Clause rider (not in repo; on pybroker.com).
- **Walk-forward (`WalkforwardMixin.walkforward_split`):** pure time-based anchored windows with `lookahead` gap. Handles `train_size==0` (test-only), `train_size==1` (train-only), and multi-window iterative adjustment.
- **BCa (Bias-Corrected and Accelerated) bootstrap, Numba-accelerated (`@njit`):** not simple percentile bootstrap. Computes bias correction `z0` + acceleration via jackknife leave-one-out. Applied to Profit Factor (log-transformed), Sharpe Ratio, Max Drawdown (upper bounds at 99.9/99/95/90%). Config: `bootstrap_samples=10_000`, `bootstrap_sample_size=1_000`.
- **Numba indicator library (`vect.py`, 1836 lines, all `@njit`):** MACD, Stochastic, Stochastic RSI, Detrended RSI, ADX, Aroon, Linear/Quadratic/Cubic Trend (Legendre), CMMA, Deviation, Price Intensity, PCO, Intraday Intensity, Money Flow, Reactivity, PVF, VWMA Ratio, OBV variants, PVI/NVI, Volume Momentum, Laguerre RSI. **Major value if PRISMATIK needs pre-built indicators.**
- **CRITICAL GAP — pybroker does NOT implement multiple-testing correction.** No `multipletest`, Bonferroni, Holm, BH. The README claims `param()` enables parameter optimization, but no built-in mechanism tracks or reports configs searched. **This confirms PRISMATIK must build its own multiple-testing disclosure** (which v1.0 §27 already mandates).
- **Commons Clause impact:** may USE and MODIFY internally; may NOT sell (host-for-fee, consulting, SaaS) a product whose value derives substantially from pybroker. **NOT open source for commercial use.**
- **PRISMATIK server/ already implements walk-forward + bootstrap + configs-searched.** pybroker offers: BCa (more sophisticated than percentile bootstrap if PRISMATIK only does percentile), the Numba indicator library (if Rust indicators aren't complete).
- **Disposition:** Mode F — ideas only, cite methods to academic sources (López de Prado). Do not link.

#### 3.19 stocks-insights-ai-agent — agentic RAG (CC BY-NC-SA 4.0)
- **What:** LangGraph + ChromaDB + LCEL agentic RAG for stocks. Three separate LangGraph workflows: News RAG (retrieve → grade → web-search-or-generate), Stock Data RAG (text-to-SQL → execute → generate), Stock Charts RAG (text-to-SQL → execute).
- **The LangGraph StateGraph pattern is the canonical agentic-RAG shape.** Conditional branching (grade → web search fallback) with `decide_to_generate` returning WEB_SEARCH or GENERATE_RESULT.
- **LLM-as-judge retrieval grading** (binary yes/no via `with_structured_output`): simpler than embedding-similarity thresholding and works well.
- **Text-to-SQL graph:** generate SQL → execute against PostgreSQL → generate answer from DataFrame results.
- **Shallow implementation:** 250-token chunks (no overlap), hardcoded 50 US large caps, no SEC filings/earnings/analyst ratings, OpenAI text-embedding-ada-002, free-form LLM text output (no scores/reports).
- **What's embedded:** news `description` fields (short summaries, not full text) in ChromaDB.
- **CC BY-NC-SA 4.0** — non-commercial AND share-alike; FSF and Creative Commons both discourage CC for code; NC is ill-defined for commercial products.
- **Disposition:** Mode F — drop. The LangGraph RAG shape is available more cleanly from ai-trading-claude (MIT) and QuantDinger (Apache). The pattern (conditional branching + retrieval grading) is reimplementable from documentation.

#### 3.20 aiagents-stock — China A-share scraper (no LICENSE)
- **What:** Python/Streamlit/DeepSeek. China A-share multi-agent stock analysis.
- **The Playwright TLS-fingerprint circumvention for iwencai.com** (`utils/iwencai_browser.py`): headless Chromium harvests real-browser cookies to bypass CAPTCHA. `utils/pywencai_helper.py` implements 2-path degradation (direct pywencai → on exception → retry with browser cookies).
- **6 analyst agents:** Technical, Fundamental, Fund Flow, Risk Management, Market Sentiment, News. Plus orchestrator with `run_multi_agent_analysis` → `conduct_team_discussion` (simulated team meeting) → `make_final_decision`.
- **Data sources (all China):** iwencai (natural-language screening), akshare (Eastmoney), Tencent, Sina, Tushare, yfinance (US/HK technicals only), qstock (news), National Bureau of Statistics.
- **Explicitly disables US features:** `app.py` shows Streamlit info banners for every US stock: "no quarterly reports / no fund-flow / no market sentiment / no news / no risk data."
- **Headline feature is exclusively A-share:** iwencai (同花顺问财) is a Chinese natural-language stock screener. Playwright circumvention exists solely to query it. **No US-market equivalent.** MiniQMT auto-trading enforces A-share T+1 with price-limit bands and board exclusions (科创板/创业板).
- **Disposition:** Mode F — **drop entirely from `reference/`** (per v1.1 Approval Register §2.3). The generic "multi-LLM-analyst fan-out + team-discussion synthesis" pattern is thin prompting, not architecture worth retaining a China-only codebase for.

#### 3.21 _HISTORICAL_ARCHIVES — zips
- **What:** Zips of the above, generated 2026-07-17 — carrying the same replaced Apache LICENSE files.
- **Disposition:** Exclude from build paths, SBOM scanning, dependency resolution. Retain only if you need a frozen snapshot of the consolidation corpus for clean-room-reimplementation provenance (recommended — it documents what you read and when).

---

## 4. The AI Stack — OAuth-first, API-key fallback, LM Studio local tier

Closes the gap that v1.1 §5 leaves open. The user's requirement: connect to all major providers, assuming they work off OAuth (preferred), API, and local models via LM Studio. Include the best of each.

### 4.1 The critical OAuth finding (changes the design)

**Two of the three frontier labs have NO legitimate OAuth path for third-party apps as of February 2026.**

- **Anthropic:** Claude Code uses OAuth 2.0 PKCE against `console.anthropic.com`, but **Anthropic explicitly prohibits third-party use of these OAuth tokens** (Feb 2026, The Register, official Legal & Compliance docs). The supported third-party path is the **Console API key** (`sk-ant-api...`). Workload Identity Federation exists for enterprise. **Conclusion: API-key only. No legitimate OAuth path for a self-hosted platform.** `[CONFIRMED]`
- **OpenAI:** Same story. OAuth surfaces exist only for (a) ChatGPT Apps SDK, (b) Codex OAuth (prohibited for third-party routing), (c) connector OAuth (you bring your own auth server). **Realistic third-party path: API key.** `[CONFIRMED]`
- **Google Gemini:** the one genuinely good OAuth story. Both Google AI Studio and Vertex AI support OAuth 2.0 with Google accounts (full OAuth 2.0 + service accounts + ADC + Workload Identity). **This is the cleanest OAuth-first integration among frontier providers.** `[CONFIRMED]`
- **OpenRouter:** explicitly supports OAuth PKCE; returns a per-user API key after browser consent. **The aggregator sweet spot — one integration surfaces every frontier model with per-model routing.** `[CONFIRMED]`

**Design consequence:** "OAuth-first" is a real strategy for Gemini + OpenRouter + the broker MCPs (Alpaca, IBKR, Coinbase), but for Claude/GPT-5.5 you fall back to API keys. The cleanest single integration is **OpenRouter OAuth**, which fans out to every frontier model including Claude and GPT.

### 4.2 The Provider Stack (concrete recommendation)

```text
┌──────────────────────────────────────────────────────────────────────┐
│  PRISMATIK AI ROUTER  (prismatik-ai-router, in prismatik-ai-tools)    │
│                                                                      │
│  Routes by capability declared on the provider, not by model name.   │
│  Every call is a recorded effect; replay returns the recorded        │
│  response, never re-invokes the model.                               │
└────────────┬──────────────────┬──────────────────┬───────────────────┘
             │                  │                  │
     ┌───────▼──────┐   ┌───────▼──────┐   ┌───────▼────────────────┐
     │ OAuth Tier   │   │ API-Key Tier │   │ Local Tier (LM Studio) │
     │              │   │              │   │                        │
     │ OpenRouter   │   │ Anthropic    │   │ T0 Phi-4-mini (3.8B)   │
     │  (PKCE →     │   │  Claude Opus │   │ T0 Qwen3-4B/8B         │
     │   per-user   │   │  5 / Sonnet  │   │ T1 Qwen3-14B Q4_K_M    │
     │   key, all   │   │  5 / Haiku   │   │ T1 Phi-4 (14B)         │
     │   models)    │   │  4.5         │   │ T2 Qwen3-30B-A3B (MoE) │
     │              │   │ OpenAI GPT-  │   │ T2 DeepSeek-R1-distill │
     │ Google       │   │  5.5 / Pro   │   │                        │
     │ Gemini 3 Pro │   │ xAI Grok 4.5 │   │ Loopback egress        │
     │  (ADC/AI     │   │ DeepSeek V4  │   │ ENFORCED, not assumed  │
     │  Studio OAuth│   │ Cohere Cmd A │   │                        │
     │  or Vertex)  │   │ Groq (fast   │   │ model_digest REQUIRED  │
     │              │   │  inference)  │   │ (pinned weights)       │
     │ Alpaca MCP   │   │ Together /   │   │                        │
     │  (OAuth      │   │  Fireworks   │   │ JSON-schema structured │
     │  execution)  │   │  (open-model │   │ output MANDATORY T0/T1 │
     │              │   │  hosting)    │   │                        │
     │ IBKR MCP     │   │              │   │ MCP client: hits       │
     │  (broker     │   │              │   │ Alpha Vantage / Polygon│
     │  auth)       │   │              │   │ / Alpaca MCP servers   │
     │              │   │              │   │                        │
     │ Coinbase MCP │   │              │   │                        │
     │  (OAuth      │   │              │   │                        │
     │  crypto)     │   │              │   │                        │
     └──────────────┘   └──────────────┘   └────────────────────────┘
```

### 4.3 Best model per capability (July 2026 state)

All pricing per MTok in/out. `[CONFIRMED]` = verified against primary sources.

| Capability | Best pick | Why | Pricing |
|---|---|---|---|
| **Frontier reasoning / agentic tool-use** | **Anthropic Claude Opus 5** | Released July 24, 2026. De facto "best tool-use frontier model" consensus. 1M context. | $5/$25 |
| **Frontier reasoning (cost-no-object)** | **OpenAI GPT-5.5 Pro** | Responses API only, MCP support, 1.05M context, 128K max output, xhigh reasoning effort. | $30/$180 |
| **Long-context multimodal** | **Google Gemini 3 Pro / 3.1 Pro** | 1M context, 77.1% ARC-AGI-2. Best OAuth story. Generous free tier in AI Studio. | $2/$12 std, $4/$18 long-ctx |
| **Cost-efficient frontier** | **Anthropic Claude Sonnet 4.5** or **Haiku 4.5** | Sonnet for general work, Haiku for high-volume T0/T1. 1M context. | $3/$15 (Sonnet) · $1/$5 (Haiku) |
| **Real-time / X-data** | **xAI Grok 4.5** | 500K context, knowledge cutoff Feb 1 2026. Requires server-side web/X search for current data. | $2/$6 |
| **Aggressive pricing / open-weight** | **DeepSeek V4-Pro** | $0.435/$0.87. Cache-hit input ~$0.0036. **R2 does NOT exist yet despite rumors.** | $0.435/$0.87 |
| **EU data residency** | **Mistral Large 3** | Apache-2.0 open-weight MoE, hosted in Paris (La Plateforme). | $0.50/$1.50 |
| **Enterprise RAG / citations** | **Cohere Command A** | 256K context, citations, structured retrieval. | $2.50/$10 |
| **Lowest-latency hosted inference** | **Groq** (Llama 4 Scout, etc.) | 460 TPS for Llama 4 Scout. ~10-20× cheaper than OpenAI on equivalent models. Ideal for T0/T1 if not self-hosting. | $0.11 input (Scout) |
| **Open-model hosting at scale** | **Together / Fireworks** | Transparent serverless pricing. Fireworks 50% batch discount for async. | $0.18/M (Scout, Together) |
| **Local T0 (symbology, extraction)** | **Phi-4-mini (3.8B)** | ~3GB VRAM. Strong tiny reasoner. | $0 (local) |
| **Local T1 (filing summary, evidence)** | **Qwen3-14B Q4_K_M** or **Phi-4 (14B)** | Schema-constrained output reliability. | $0 (local) |
| **Local T2 (multi-evidence synthesis)** | **Qwen3-30B-A3B (MoE)** | 30B total / 3B active. **The standout local pick for synthesis on 16GB VRAM.** | $0 (local) |

### 4.4 LM Studio integration (refines v1.1 §5)

LM Studio 0.4.x as of July 2026 supports everything PRISMATIK needs natively:
- OpenAI-compatible `/v1/chat/completions` AND `/v1/responses` (stateful via `previous_response_id`)
- Native JSON-schema structured output (one known bug with pydantic enums/literals, GitHub #1105)
- Tool calling (`tool_choice` auto/none/required)
- **Remote MCP client support (opt-in)** — LM Studio can hit Alpha Vantage / Polygon / Alpaca MCP servers directly. This enables fully local-orchestrated, no-API-key-to-LLM-vendor workflows.
- GGUF + MLX formats; cross-platform including Apple Silicon

**Alternatives for production multi-user:**
- **vLLM** — clear production/multi-user choice; auto-parses structured output for new models; collapses-free under concurrency where Ollama dies at ~5 users.
- **MLX Server (Apple)** — WWDC26 release; OpenAI-compatible HTTP server in `mlx-lm[server]`, structured tool calling. **Credible LM Studio alternative on Mac dev boxes.**
- **Ollama** — easiest install, production-light, dies at ~5 concurrent users.
- **llama.cpp** — engine under LM Studio/Ollama; widest hardware support; best portability.

### 4.5 The PRISMATIK Provider contract (extends v1.1 §5)

```rust
pub struct InferenceProvider {
    pub id: ProviderId,
    pub kind: InferenceProviderKind,        // OAuth { provider, scopes } | ApiKey | LmStudioLocal | OpenAiCompatible
    pub auth: AuthMethod,                   // OAuthTokenSource | ConsoleApiKey | Loopback
    pub endpoint: Url,
    pub model_id: String,
    pub model_digest: ContentHash,          // pinned weights — required for reproducibility
    pub context_window: usize,
    pub capabilities: InferenceCapabilities, // tool_use, json_schema, vision, reasoning
    pub determinism: DeterminismProfile,    // temperature, seed, top_p — pinned
    pub egress: EgressPolicy,               // Loopback for local — ENFORCED
    pub cost_model: CostModel,              // Zero for local
    pub max_tokens_per_call: usize,
}

pub enum AuthMethod {
    /// OpenRouter PKCE / Gemini ADC / Alpaca OAuth / IBKR OAuth / Coinbase OAuth
    OAuth { token_source: TokenSource, scopes: Vec<Scope> },
    /// Anthropic Console / OpenAI Platform / DeepSeek / Cohere / Groq / Together / Fireworks
    ConsoleApiKey { key_vault_ref: SecretRef },
    /// LM Studio local — structurally incapable of reaching the network
    Loopback,
}
```

**Five rules, refined from v1.1 §5:**
1. `model_digest` is required. "Qwen3-14B" is not a reproducible identifier.
2. `egress: Loopback` is **enforced, not documented.** The local provider must be structurally incapable of reaching the network — this is the property that makes the privacy claim defensible.
3. Every inference call is a recorded effect. Replay returns the recorded response.
4. Capability declaration drives routing, not model name. A 14B's tool-use reliability differs sharply from a frontier model's.
5. **OAuth tokens and API keys live in the OS keychain via `prismatik-security`, never in process env beyond the moment of use.** OAuth refresh is the auth module's job, not the router's.

### 4.6 Capability routing matrix (the actual decision table)

| Workload | Tier | Default route | Fallback |
|---|---|---|---|
| Symbology normalization, field extraction, classification | T0 | LM Studio Phi-4-mini (local) | LM Studio Qwen3-8B (local) |
| Filing summarization, evidence extraction, chart-tool composition, journal drafting | T1 | LM Studio Qwen3-14B (local) | Groq Llama 4 Scout (hosted, fast) |
| Multi-evidence synthesis, thesis critique, risk narrative | T2 | LM Studio Qwen3-30B-A3B (local) | Claude Sonnet 4.5 (API key) |
| Genuinely hard synthesis, adversarial critique, code generation | T3 | Claude Opus 5 or GPT-5.5 Pro (hosted, **user opt-in per call**) | OpenRouter OAuth (whichever frontier model user prefers) |
| Real-time news / X-data synthesis | — | Grok 4.5 (API key, server-side web search) | Gemini 3 Pro (OAuth) |
| EU-resident customer data | — | Mistral Large 3 (La Plateforme) | Local LM Studio |

**T3 escalation is a consent surface, not a config flag.** Show the user the exact payload that would leave the machine. Same uncertainty-contract discipline as cost-basis provenance.

---

## 5. Bleeding-Edge Updates Beyond v1.1 (July 2026)

What's changed since the v1.1 addendum was written, with confidence levels.

### 5.1 TSFM landscape — new entrants worth adding to v1.0 §6.7's registry

- **Toto 2.0 (Datadog, May 14 2026)** `[CONFIRMED]` — Apache-2.0, decoder-only, 5 sizes 4M→2.5B. **First TSFM that reliably improves with scale**: 313M beats Chronos-2 by 1.6 pts on GIFT-Eval; 2.5B still faster than Chronos-2 at some horizons. Worth adding to the registry as a scaling-tunable option.
- **TiRex / TiRex-2 (NX-AI, Hochreiter)** `[CONFIRMED]` — **xLSTM** (recurrent, SSM-family), 35M (TiRex-1.1). SOTA on GIFT-Eval/Chronos-ZS. **TiRex-2 adds multivariate + covariates + constant-cost streaming inference.** Most relevant for live tick data where transformer context windows are wasteful.
- **Chronos-2 (Amazon, Oct 2025)** `[CONFIRMED]` — already in v1.1 §4.1. 120M encoder-only, group + time attention. Native zero-shot covariate support. Apache-2.0. Current general-purpose leader on fev-bench, GIFT-Eval, Chronos Benchmark II.
- **TimesFM-2.5 (Google)** `[CONFIRMED]` — ~200M. Competitive zero-shot. Apache-2.0. Already in v1.0 §6.7.
- **Moirai-2 (Salesforce)** — excels on multivariate/sparse data. **Still CC BY-NC 4.0 — registry hard-deny at commercial registration remains correct.**
- **Kronos (Tsinghua, AAAI 2026)** — cited 25×. **Most directly relevant for PRISMATIK.** Finance-domain pretrain.

**Updated recommendation for v1.0 §6.7:** the plural registry should be `Kronos (K-line specialist, MIT) + Chronos-2 (general + covariates, Apache) + TiRex-2 (streaming/xLSTM, Apache) + Toto 2.0 (scaling option, Apache) + Lag-Llama (probabilistic baseline, Apache)`. Moirai/Moirai-2 hard-denied. Per-frequency-band baseline benchmarking against Auto-Theta/ARIMA/GARCH is non-optional (per v1.1 §4.1).

### 5.2 The biggest architecture-surface changes (read these)

#### 5.2.1 SR 11-7 was REPLACED in 2026 — v1.0's compliance references are stale
**SR 11-7 → SR 26-2 / OCC Bulletin 2026-13 (April 2026)** `[CONFIRMED]` — joint OCC/Fed/FDIC revised Model Risk Management guidance. Modernizes for AI/ML and dynamic models; concentrates validation on **high-materiality models**. **Critical carve-out: "Generative AI and agentic AI models are novel and rapidly evolving. As such, they are not within the scope of this guidance."** Applies to banks >$30B assets. Promise of future GenAI-specific guidance (likely preceded by an RFI).

**Implication for v1.0 §22:** update references from SR 11-7 to SR 26-2. The model registry already satisfies most MRM requirements — say so explicitly, because "we already have a compliant model inventory" is a concrete enterprise sales asset. GenAI-specific guidance is coming; treat as a tracked risk.

#### 5.2.2 EU AI Act high-risk obligations — DELAYED to December 2027
**The Aug 2, 2026 milestone has been deferred.** `[CONFIRMED]` — Digital Omnibus on AI (proposed Nov 19 2025; Parliament/Council agreement May 7 2026; formal EU approval June 2026) postpones standalone Annex III high-risk obligations from Aug 2, 2026 to **Dec 2, 2027** — a 16-month delay.

**What still applies on original timeline (Aug 2, 2026):** transparency obligations, new prohibited-practices ban (Dec 2026, plus "nudifier" app ban). Annex I (AI in regulated products) → Aug 2028. Fines up to €35M or 7% global turnover.

**Implication for v1.1 §4.3:** the "8 days from this document's date" urgency is wrong as of July 2026. If PRISMATIK targets EU customers, transparency obligations still bite Aug 2, 2026 (disclose AI-generated content/recommendations) — but the full high-risk compliance plane has 16 more months. **The compliance plane can stay in Phase 8 if scoped to transparency only; expand if/when high-risk classification applies.**

#### 5.2.3 SEC Predictive Data Analytics final rule — landed June 2025
**SEC Release No. 34-97990** `[CONFIRMED]` — covers broker-dealers and RIAs using "covered technology" in "investor interactions." Must identify, eliminate, or neutralize conflicts of interest. Intersects FINRA Rules 2210 (projections in marketing) and 2214 (investment analysis tools disclosure).

**Implication:** if PRISMATIK makes recommendations or interacts with investors using predictive AI for a broker-dealer or RIA user, this rule applies. Build conflict-elimination/neutralization documentation into the design from Phase 1.

#### 5.2.4 Thread-per-core + io_uring has gone mainstream — with measured numbers
**Apache Iggy migrated off Tokio to `compio` + thread-per-core + io_uring (Feb 2026)** `[CONFIRMED]` — concrete case study: P99 latency 4.52ms → 1.82ms (+60%), P9999 27.52ms → 11.83ms (+57%), fsync throughput +18%. Key lessons: holding a `RefCell` borrow across `.await` causes runtime panics (split into control plane + data plane); you must heavily batch syscalls; `IOSQE_IO_LINK` required for ordering; POSIX `File`/`TcpListener` are the wrong abstractions.

**Implication for v1.0 §9 process model:** keep Tokio for the control path and Python interop. **Evaluate a thread-per-core `compio`/`monoio` worker shard for the latency-critical market-data → order path.** This is now a proven pattern (Iggy, Cloudflare Pingora), not a research bet. The CV VE-2026-43121 (io_uring ZCRX freelist race → double-free) shows the newest networking features need scrutiny; storage io_uring is production-ready.

#### 5.2.5 TigerBeetle production-ready + Rust client (April 2026)
`[CONFIRMED]` — purpose-built financial accounting OLTP database. Jepsen-passing consensus. Written in Zig, **Rust client shipped April 2026**. **Directly usable from PRISMATIK's Rust core for the audit-ledger-as-write-path pattern (v1.0 §14.2).** Stronger 2026-native combo for WORM audit than generic event sourcing.

#### 5.2.6 WASI 0.3 ("Preview 3") shipped in 2026 with native async
`[CONFIRMED]` — native async/await, `future` and `stream` types built into the Component Model. Previews in **Wasmtime 37+**. **Streaming plugins (market-data transformers) can now be async-native rather than callback-based.** v1.1 §4.4's "track WASI 0.3" item is now shipping.

#### 5.2.7 Tauri 2 wins decisively on size/RAM/startup — but patch the CVE
**Tauri 2 vs Electron (2026 numbers):** ~5MB vs ~150MB (96% smaller); idle RAM ~42MB vs ~168MB (75% less); cold start 0.5-2s vs 2-4s (3.7× faster). `[CONFIRMED]`

**CRITICAL — CVE-2026-42184 (Tauri 2.0 through 2.11.0):** flaw in `is_local_url()` causes **remote URLs to be classified as trusted local origins on Windows and Android** → authentication bypass of the IPC trust boundary. **Patch immediately to ≥2.12.** For a trading app, treat the IPC boundary as untrusted and add defense-in-depth.

### 5.3 The MCP ecosystem (refines v1.0 §6.5 and v1.1 §4.2)

- **rmcp 1.x stable** `[CONFIRMED]` — official Rust SDK. v1.1 §4.2's "high version velocity" concern is partially resolved.
- **MCP 2026-07-28 Release Candidate** `[CONFIRMED]` — major rewrite: **stateless protocol core** (no `Mcp-Session-Id`, no handshake, any request can hit any server instance), **Extensions framework** (incl. "MCP Apps"), authorization hardening, response caching. **Deprecates Roots, Sampling, Logging APIs.**
- **2025-06-18 spec (current published):** MCP servers are **OAuth 2.1 Resource Servers**; clients are confidential/public OAuth clients. Requires Resource Indicators (RFC 8707), PKCE.
- **Market-data MCP servers (all official, all `[CONFIRMED]`):**
  - **Alpha Vantage MCP** — `mcp.alphavantage.co`. Real-time + historical stock/forex/crypto.
  - **Polygon.io MCP** — 35+ tools across stocks/options/forex/crypto.
  - **Unusual Whales MCP** — 100+ endpoints (options flow, dark pool, congressional trading, Greek exposure, vol).
- **Broker/execution MCP servers (all `[CONFIRMED]`):**
  - **IBKR** — first major broker to ship official MCP. Production-ready.
  - **Alpaca MCP V2** — 61 endpoints auto-generated from OpenAPI, full trade execution (stocks/options/crypto), **OAuth support**. Best OAuth-first path for execution.
  - **Coinbase Developer Platform** — OAuth-based authenticated trading.

### 5.4 Adversarial eval frameworks — what to add to v1.0 §27

v1.1 §4.2 cites AgentDojo, LlamaFirewall, GuardAgent, InferAct. July 2026 state:

- **AgentDojo** (ETH Zürich) `[CONFIRMED]` — canonical dynamic environment for prompt-injection attacks/defenses on LLM agents. 764+ citations. Integrated into UK AISI's Inspect framework.
- **LlamaFirewall** (Meta, May 2025) `[CONFIRMED]` — >90% efficacy reducing attack success rate on AgentDojo. Combines PromptGuard 2 + other shields. Mature, production-usable.
- **InjecAgent** (arXiv:2403.02691) `[CONFIRMED]` — first benchmark for indirect prompt injection in tool-integrated LLM agents. **Most directly relevant to financial tool-calling security.**
- **OWASP Top 10 for Agentic Applications (2026)** `[CONFIRMED]` — ASI01 Agent Goal Hijack, ASI02 Tool Misuse/Excessive Agency. The Giskard finance example explicitly covers a tool-call attack (`report` vs `report_finance` confusion → data exfiltration). **Recommended baseline for security design.**
- **Inspect AI / Inspect Evals** (UK AISI) `[CONFIRMED]` — the framework Anthropic itself uses. Includes OWASP Top 10 Agentic evals.
- **Anthropic "Demystifying evals for AI agents"** `[CONFIRMED]` — trajectory vs. outcome metrics, LLM-judge calibration.
- **Finance Agent Leaderboard** `[CONFIRMED]` — `llm-stats.com/benchmarks/finance-agent`. Tracks 8 models on finance+agent reasoning.
- **GuardAgent / InferAct** `[LOW]` — lower current visibility in 2026-specific results.

**Recommendation for v1.0 §27:** add the prompt-injection adversarial suite (AgentDojo-style) over filings, news, and market commentary. **A malicious 8-K is a realistic attack vector for a filings-aware product.** Map the agent design to OWASP ASI01/ASI02; wrap every tool-calling boundary with LlamaFirewall-pattern guards. Use Inspect AI as the CI/CD red-team eval harness.

### 5.5 New 2026 literature worth citing

- **"AI Agents in Financial Markets: Architecture, Applications..."** (arXiv:2603.13942v2) `[HIGH]` — integrative framework for analyzing agentic finance including concentration risk and supervisory capacity.
- **"Toward a unified agentic framework for regime-aware portfolio optimization"** (Mantshimuli, 2026, Springer) `[HIGH]` — unifies regime inference, LLM-derived sentiment, and portfolio optimization.
- **StockBench** (Chen et al., 2025, arXiv:2510.02209) — benchmark evaluating LLM agents in realistic stock trading.
- **FinRL-X** (arXiv:2603.21330) — modular deployment-consistent DRL framework for portfolio management.
- **FAME (AI & Finance Paper Hub)** — `fame-ai.org/hub`. Five recurring biases: look-ahead, survivorship, narrative, objective, cost.
- **"Formal Verification for Harness Engineering in Quantitative Finance"** (2026) `[MEDIUM]` — Lean 4 reference architecture separating proposal from modeled authority at a pre-trade risk boundary. **Research-grade but directly applicable to PRISMATIK's risk gates.** Dafny may be more practical (82% vs 27% verified-program-synthesis success).

### 5.6 Other items worth knowing

- **MPC custody is now the dominant institutional crypto custody model** `[CONFIRMED]` — Fireblocks and Coinbase Prime/International Exchange lead. If PRISMATIK does non-trivial crypto volume for institutional users, MPC custody (via Fireblocks/Coinbase Prime API) is table stakes, not something to roll yourself.
- **Coinbase September 9, 2026 migration:** international derivatives move from INTX onto a Deribit-powered gateway. Plan for this if you trade perps internationally.
- **Databento (2026)** `[CONFIRMED]` — most Rust-friendly market-data vendor. Official Python, C++, **Rust** client libraries. Pay-as-you-go usage-based pricing. Strong NautilusTrader integration.
- **IEX Cloud shut down Aug 31, 2024** — important if any legacy code references it. IEX *Exchange* still operating but data policies changed Feb 1, 2025.
- **Unusual Whales is the clear API leader for options flow** — $50/mo, full API at api.unusualwhales.com/docs, includes dark pool + congressional trades + Form 4 + 13F. vs FlowAlgo ($149/mo, options-only, limited API) vs SEPT ($29.99).
- **LanceDB repositioned as "Multimodal Lakehouse for AI"** `[CONFIRMED]` — text, images, video, audio, point clouds alongside embeddings with versioning/deduplication/sampling. Confirms v1.0 §6.1 adoption was well-placed.
- **Rekor v2 GA** — moved to tile-based log backed by Trillian-Tessera. Ethereum anchoring of Rekor checkpoints being explored. OpenSSH released a BigQuery public dataset of the entire Rekor log.

---

## 6. Revised Backlog Deltas (consolidates v1.1 + this addendum)

### Phase 0 (Foundation)
1. `reference/LICENSE_DISPOSITION.md` mirroring §1.3's verified table; build/SBOM exclusion for `reference/`.
2. Register all 21 reference projects as `ValidationOracleOrReference` manifests with `execution_allowed: false`.
3. Decimal discipline: `rust_decimal` at every boundary, string representation in serialized monetary fields.
4. **Compliance-plane skeleton scoped to transparency obligations only** (Aug 2, 2026 deadline) — high-risk plane moves to Phase 8 (16-month EU delay).
5. Inference call as a recorded effect in `prismatik-determinism`.
6. **Tauri patched to ≥2.12** (CVE-2026-42184). IPC boundary treated as untrusted with defense-in-depth.

### Phase 1 (Crypto MVP)
7. `BrokerError` taxonomy with permanence bit and `Connecting` state (§3.8 OpenAlice).
8. `ProviderChain` with declared fallbacks, `FailoverTrigger::Empty`, evidence-graph provenance (§3.5 adata).
9. **AI Router (`prismatik-ai-router`)** with OAuth tier (OpenRouter + Gemini + Alpaca MCP + Coinbase MCP), API-key tier (Anthropic + OpenAI + DeepSeek + Groq), local tier (LM Studio T0/T1 with schema-constrained decoding).
10. `Position` with required `multiplier` and `avg_cost_source` (§3.8 OpenAlice).
11. **MCP 2026-07-28 RC alignment** in `prismatik-ai-tools` (stateless core, OAuth 2.1 Resource Server pattern).

### Phase 2 (Equity + Filings)
12. SEC PDA rule (Release 34-97990) conflict-elimination documentation in the compliance plane.
13. **Provider coverage catalog** built from OpenBB's TET Fetcher pattern reimplemented clean-room (§3.14).

### Phase 3 (Options)
14. Unusual Whales adapter behind the `Provider` port ($50/mo API leader).
15. **Toto 2.0 and TiRex-2 added to TSFM registry candidates** alongside Chronos-2 and Kronos (§5.1).

### Phase 4 (Plugin Host + Indicator Kernel)
16. **Drop the Extism prototype** — wasmtime + Component Model / WIT on `wasm32-wasip2` (WASI 0.3 native async now shipping in Wasmtime 37+).
17. Capability-diffing gate on plugin imports/exports, cosign verification at load.
18. `BarSampler` (time/tick/volume/dollar) to `prismatik-indicator-core` (§3.2 AIAlpha).
19. Purged + embargoed walk-forward (§3.2 AIAlpha).
20. **Prompt-injection adversarial suite** (AgentDojo + InjecAgent + OWASP ASI01/ASI02) over filings, news, market commentary. LlamaFirewall-pattern guards at every tool-calling boundary.

### Phase 5 (TSFM)
21. Per-frequency-band baseline benchmarking — Auto-Theta / ARIMA / GARCH (v1.1 §4.1).
22. Kronos as K-line specialist **and** synthetic path generator for Monte Carlo **and** tokenizer-as-embedding (closes G06 first).
23. TiRex-2 for streaming/covariate-aware live tick forecasts.

### Phase 6 (Portfolio + Paper)
24. **`prismatik-reconciliation`** — continuous sync loop, divergence classification, auto-heal benign cases, halt only on irreconcilable (§3.9 nofx). **Land before Phase 7.**
25. **TigerBeetle Rust client evaluated** for audit-ledger-as-write-path (v1.0 §14.2). Stronger than generic event sourcing for the WORM audit trail.

### Continuous Tracks
26. WORM mirror + external anchoring (Rekor v2 / Ethereum) for the audit ledger.
27. **SR 26-2 model-risk documentation** (replaces SR 11-7 references throughout v1.0 §22).
28. **Inspect AI as CI/CD red-team eval harness** for the trading agent.

---

## 7. Honest Assessment (cold read)

Five things worth flagging plainly.

**The v1.1 license table is substantively right but its evidence is compromised.** Every root `LICENSE` file in `reference/` is a byte-identical Apache-2.0 replacement. The table happens to match real upstream (verified via SPDX + source headers), but the methodology claim "verified by reading the LICENSE file" is unreliable and must be corrected. Under the consolidation-corpus framing this matters less than under an embed framing, but the corpus needs `LICENSE_DISPOSITION.md` documenting the truth.

**One project is mislabeled and one is misdescribed.** `crypto-ai-trading-tool` is the ADAMANT market-making bot (GPL-3.0 by SPDX), not a liquidity-sweep detector — its README is a fabricated veneer and the v1.1 addendum's claim of "no LICENSE" is wrong. OctoBot is LGPL-3.0 at the library level, not GPL-3.0. Both corrections are documented in §1.3 and §3.

**The OAuth-first strategy has a real hole at the frontier.** Anthropic and OpenAI both prohibit third-party use of their subscription OAuth tokens as of Feb 2026. "OAuth-first" works cleanly for Gemini, OpenRouter, and the broker MCPs (Alpaca, IBKR, Coinbase) — but for Claude Opus 5 and GPT-5.5 Pro you fall back to API keys. The cleanest single integration is OpenRouter OAuth, which fans out to every frontier model.

**The 2 August 2026 EU AI Act urgency in v1.1 §4.3 is wrong as of July 2026.** The Digital Omnibus delayed standalone Annex III high-risk to December 2027. Transparency obligations still bite August 2, 2026, but the full compliance plane can stay in Phase 8 if scoped to transparency. This is a 16-month relief on the most aggressive item in the v1.1 backlog.

**The single highest-value architecture additions in 2026 are concrete, not speculative.** TigerBeetle's Rust client makes the audit-ledger-as-write-path (v1.0 §14.2) production-real. WASI 0.3 native async makes streaming plugins first-class. SR 26-2 replaces SR 11-7 and explicitly carves out GenAI — the compliance posture improves. Thread-per-core + io_uring has measured production numbers (Iggy: +60% P99). These are not research bets; they are shipping technology that PRISMATIK's v1.0 architecture is well-positioned to absorb.

---

## Appendix — Sources

**Reference projects (local, `D:\DevOps\PRISMATIK\reference\`)** — 21 projects cataloged at equal depth in §3. Real licenses recovered from `package.json`/`pyproject.toml` SPDX declarations and source copyright headers; root `LICENSE` files are untrustworthy (§1).

**Architecture corpus (local, `D:\DevOps\PRISMATIK\DOCS\` and root):**
- `PRISMATIK_Unified_Solution_Architecture_v1.0.md` (2887 lines) — sections 1–33 + appendices A-D read in full
- `PRISMATIK_Phased_Implementation_Plan_v1.0.md` (740 lines) — phases 0–9 + milestones
- `PRISMATIK_v1.1_Reference_Harvest_and_Bleeding_Edge_Addendum.md`
- `PRISMATIK_v1.1_Integration_Guide_Adoptable_Components.md`
- `PRISMATIK_v1.1_Approval_Register.md`

**Web, verified 2026-07-26:**

*AI providers / OAuth:*
- [Anthropic Opus 5](https://www.anthropic.com/news/claude-opus-5) · [Anthropic OAuth ban (The Register)](https://www.theregister.com/software/2026/02/20/anthropic-clarifies-ban-on-third-party-tool-access-to-claude/5014526) · [Anthropic pricing](https://platform.claude.com/docs/en/about-claude/pricing)
- [OpenAI GPT-5.5](https://openai.com/index/introducing-gpt-5-5/) · [GPT-5.5 API](https://developers.openai.com/api/docs/models/gpt-5.5)
- [Gemini OAuth](https://ai.google.dev/gemini-api/docs/oauth) · [Google OAuth 2.0](https://developers.google.com/identity/protocols/oauth2)
- [xAI models](https://docs.x.ai/developers/models)
- [OpenRouter OAuth](https://openrouter.ai/docs/guides/overview/auth/oauth) · [OpenRouter MCP](https://openrouter.ai/docs/guides/overview/mcp-server)
- [DeepSeek pricing](https://api-docs.deepseek.com/quick_start/pricing/) · [Manifold on R2](https://manifold.markets/Bayesian/when-will-deepseek-release-r2)
- [Mistral Large 3](https://mistral.ai/fr/news/mistral-3/)
- [Cohere pricing](https://cohere.com/pricing) · [Command A](https://docs.cohere.com/docs/command-a)
- [Groq pricing](https://groq.com/pricing)
- [Together pricing](https://www.together.ai/pricing) · [Fireworks blog](https://fireworks.ai/blog/best-llm-api-providers)
- [Llama 4 release](https://ai.meta.com/blog/llama-4-multimodal-intelligence/)

*Local inference:*
- [LM Studio API changelog](https://lmstudio.ai/docs/developer/api-changelog) · [LM Studio structured output](https://lmstudio.ai/docs/developer/openai-compat/structured-output) · [LM Studio tools](https://lmstudio.ai/docs/developer/openai-compat/tools)
- [vLLM vs llama.cpp (Red Hat)](https://developers.redhat.com/articles/2025/09/30/vllm-or-llamacpp-choosing-right-llm-inference-engine-your-use-case) · [Worldline benchmark](https://blog.worldline.tech/2026/01/29/llm-inference-battle.html)
- [Apple WWDC26 MLX session](https://developer.apple.com/videos/play/wwdc2026/232/)

*MCP ecosystem:*
- [MCP 2026-07-28 RC](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/) · [MCP Authorization spec](https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization)
- [rmcp Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [Alpha Vantage MCP](https://mcp.alphavantage.co/) · [Polygon MCP](https://www.pulsemcp.com/servers/polygon)
- [IBKR MCP](https://www.interactivebrokers.com/en/trading/ai-integrations.php) · [Alpaca MCP](https://alpaca.markets/mcp-server) · [Coinbase MCP](https://www.coinbase.com/how-to-buy/base-lets-build-an-mcp-server-together-9ad3)

*TSFM landscape:*
- [Toto 2.0](https://www.datadoghq.com/blog/ai/toto-2/) · [TiRex arXiv](https://arxiv.org/html/2505.23719v1) · [TiRex-2](https://www.nx-ai.com/en/tirex-2)
- [Chronos-2](https://arxiv.org/pdf/2510.15821) · [Amazon Science Chronos-2](https://www.amazon.science/blog/introducing-chronos-2-from-univariate-to-universal-forecasting)
- [Kronos AAAI 2026](https://ojs.aaai.org/index.php/AAAI/article/view/39730) · [Kronos arXiv](https://arxiv.org/abs/2508.02739)
- [Re(Visiting) TSFMs in Finance](https://arxiv.org/pdf/2511.18578) · [Forecasting Realized Volatility with TSFMs](https://arxiv.org/pdf/2607.05291)

*Agentic eval / security:*
- [AgentDojo](https://github.com/ethz-spylab/agentdojo) · [LlamaFirewall (InfoQ)](https://www.infoq.com/news/2025/05/llamafirewall-agent-protection/)
- [OWASP Agentic 2026](https://www.giskard.ai/knowledge/owasp-top-10-for-agentic-application-2026) · [Promptfoo OWASP mapping](https://www.promptfoo.dev/docs/red-team/owasp-agentic-ai/)
- [Inspect AI (UK AISI)](https://inspect.aisi.org.uk/)
- [Anthropic demystifying evals](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents)
- [Finance Agent Leaderboard](https://llm-stats.com/benchmarks/finance-agent)

*Regulatory:*
- [EU Digital Omnibus delay (Winston & Taylor)](https://www.winstontaylor.com/insights/ai-act-rules-on-high-risk-ai-delayed-as-ai-digital-omnibus-agreed) · [Latham & Watkins](https://www.lw.com/en/insights/ai-act-update-eu-resolves-to-change-rules-and-extend-deadlines)
- [OCC Bulletin 2026-13](https://www.occ.gov/news-issuances/bulletins/2026/bulletin-2026-13.html) · [Fed SR 26-2](https://www.federalreserve.gov/supervisionreg/srletters/SR2602.pdf) · [Moody's SR 11-7→SR 26-2](https://www.moodys.com/web/en/us/insights/banking/from-sr117-to-sr262-managing-model-risk-when-models-dont-stand-still.html)
- [SEC PDA rule](https://www.sec.gov/rules-regulations/2025/06/s7-12-23)

*Trading systems engineering:*
- [Iggy thread-per-core + io_uring](https://iggy.apache.org/blogs/2026/02/27/thread-per-core-io_uring/)
- [TigerBeetle Rust client](https://tigerbeetle.com/blog/2026-04-24-toolchain-horizons) · [TigerBeetle](https://tigerbeetle.com/)
- [WASI 0.3 (Bytecode Alliance)](https://bytecodealliance.org/articles/WASI-0.3)
- [CVE-2026-42184 (SentinelOne)](https://www.sentinelone.com/vulnerability-database/cve-2026-42184/) · [GHSA-7gmj-67g7-phm9](https://github.com/advisories/GHSA-7gmj-67g7-phm9)
- [Tauri vs Electron 2026](https://tech-insider.org/tauri-vs-electron-2026/)
- [NautilusTrader releases](https://github.com/nautechsystems/nautilus_trader/releases)
- [Databento Rust docs](https://docs.rs/databento) · [Databento microstructure/SBE](https://databento.com/microstructure/sbe)
- [Rekor v2 GA](https://blog.sigstore.dev/rekor-v2-ga/)

*Literature:*
- [AI Agents in Financial Markets (arXiv:2603.13942)](https://arxiv.org/abs/2603.13942)
- [Regime-aware portfolio optimization (Springer)](https://link.springer.com/article/10.1007/s41060-026-01066-0)
- [FinRL-X (arXiv:2603.21330)](https://arxiv.org/html/2603.21330v1)
- [FAME AI & Finance Paper Hub](https://fame-ai.org/hub/)
- [Formal Verification in Quant Finance (ResearchGate)](https://www.researchgate.net/publication/410601276_Formal_Verification_for_Harness_Engineering_in_Quantitative_Finance)
- [Agentic Trading (arXiv:2605.19337)](https://arxiv.org/html/2605.19337v1) · [TradeTrap (arXiv:2512.02261)](https://arxiv.org/pdf/2512.02261) · [Execution Assumptions in LLM Backtests (arXiv:2606.08285)](https://arxiv.org/pdf/2606.08285)
