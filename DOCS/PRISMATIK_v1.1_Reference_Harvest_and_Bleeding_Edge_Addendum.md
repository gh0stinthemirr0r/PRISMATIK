# PRISMATIK v1.1 — Reference Harvest and Bleeding-Edge Addendum

**Companion to:** `PRISMATIK_Unified_Solution_Architecture_v1.0.md`, `PRISMATIK_Phased_Implementation_Plan_v1.0.md`
**Scope:** disposition of the 21 on-disk reference projects in `reference/`, plus 2026 state-of-the-art findings not present in v1.0
**Markets:** US equities/options + crypto. China-market tooling is retained for *pattern* value only, never for data.
**Verification date:** 2026-07-26

---

## 0. The One Thing to Read First

I inspected every project in `reference/` and checked its actual `LICENSE` file rather than its README badge. **The result changes how you can use most of them.**

Fourteen of the twenty-one reference projects are AGPL-3.0, GPL, Commons Clause, CC-BY-NC, or carry **no license file at all**. Under your own v0.4 §34 license policy, every one of those is **Red — do not embed**. They are architecture reading, not source material.

This is not a setback. The highest-value thing in these repos was never the code — it was the *hard-won operational knowledge*: the broker error taxonomy that took OpenAlice a year of production incidents to get right, nofx's position-reconciliation state machine, the risk-veto placement in ai-market-maker. Those are facts about the domain, not expressions of them. You can reimplement them freely. What you cannot do is copy files.

The practical rule for this repo:

> **Read `reference/` in a separate window. Never paste from it. Every pattern you take gets written up as an ADR that names the observed source and asserts clean-room reimplementation.**

Add `reference/` to a build-excluded path and to your `cargo deny` / SBOM ignore list so nothing there can ever be linked accidentally.

---

## 1. Verified License Disposition of `reference/`

Checked by reading `LICENSE` head, 2026-07-26. Note the mismatches: `AI-Trader` shows an MIT badge in its README but ships **no LICENSE file**; `crypto-ai-trading-tool` and `aiagents-stock` likewise.

| Project | Actual license | Class | Mode | Value to PRISMATIK |
|---|---|---|---|---|
| **Kronos** | MIT | Green | **A — model artifact + finetune ref** | Financial TSFM. Already in v1.0 §6.7. Highest-value item here. |
| **ai-trading-claude** | MIT | Green | **A / B — harvestable** | 5-agent parallel analyst → composite Trade Score. Directly reusable shape for the AI plane. |
| **ai-algotrading-agent** | MIT | Green | **E — reference** | Tick-replay + trailing-stop semantics; small, clean, good conformance fixtures. |
| **AIAlpha** | MIT | Green | **E — reference** | López de Prado pipeline: dollar/volume bars, triple-barrier thinking, stacked autoencoder features. Dated code, durable ideas. |
| **QuantDinger** | Apache-2.0 | Green | **E — architecture ref, C for MCP** | Cleanest end-to-end reference: research → strategy code → backtest → paper → live → monitoring. Has an MCP server and an observability compose file. |
| **adata** | Apache-2.0 | Green | **E — pattern only** | China A-share. **Ignore the data.** Take the multi-source fusion/failover pattern (§3.6). |
| **stockbot-on-groq** | Apache-2.0 | Green | **E — UX reference** | Tool-use → generative UI → live chart. Informs the `chart.compose` AI tool contract in v0.4 §43. |
| **OpenAlice** | **AGPL-3.0** | Red | **E — design oracle, no code** | *Richest design source in the set.* UTA broker protocol, guardian runtime, trading-as-git. See §3.1. |
| **nofx** | **AGPL-3.0** | Red | **E — design oracle, no code** | Go. Multi-venue live trading with real reconciliation and regime logic. See §3.2. |
| **ai-market-maker** | **AGPL-3.0** | Red | **E — design oracle** | LangGraph desks + hard Risk Guard veto + benchmark-vs-buy-and-hold discipline. |
| **ai-auto-trading-engine** (NexusQuant) | **AGPL-3.0** | Red | **E** | Multi-timeframe LLM signal fusion; rate-limit optimization notes. |
| **prism-insight** | **AGPL-3.0 + commercial dual** | Red | **E** | 13-agent orchestration, tracking/journal/report loop. Dual-license means embedding requires a paid license. |
| **OctoBot** | **GPL-3.0** | Red | **E** | Mature crypto bot; tentacle/plugin architecture is instructive for your wasm plugin host. |
| **go-stock** | **GPL-3.0** | Red | **E** | Wails desktop app — a Go/Wails analogue of your Tauri shell. UI patterns only. |
| **OpenBB** | **AGPL-3.0** | Red | **F — do not embed** | Already correctly dispositioned in v0.4 §35. Provider-coverage catalog only. |
| **pybroker** | **Commons Clause** | Red | **F — ideas only** | Walk-forward + bootstrapped metrics. You already ship this in `server/`. Do not link. |
| **stocks-insights-ai-agent** | **CC BY-NC-SA 4.0** | Red | **F** | Non-commercial *and* ShareAlike. Agentic RAG shape only. |
| **AI-Trader** | **No LICENSE file** | Red | **F until resolved** | Agent-native skills layout is interesting; treat as all-rights-reserved until upstream clarifies. |
| **crypto-ai-trading-tool** | **No LICENSE file** | Red | **F** | Order-book liquidity-sweep detection. Concept is public; code is not licensed. |
| **aiagents-stock** | **No LICENSE file** | Red | **F** | China A-share, scraping-dependent (Playwright to defeat TLS fingerprinting on iwencai). Explicitly excluded by v0.4 §Mode F "undocumented scraping". |
| **_HISTORICAL_ARCHIVES** | n/a | — | — | Zips of the above. Exclude from all tooling. |

### Recommended action

Add `reference/LICENSE_DISPOSITION.md` mirroring this table, and a `reference/README.md` carrying the no-paste rule. Register each as a `ThirdPartyComponentManifest` with `integration_mode: ValidationOracleOrReference` and `execution_allowed: false` — this exercises your v0.4 Part XI intake pipeline against real data on day one, which is worth doing for its own sake.

---

## 2. What Is Genuinely Missing From v1.0

The v1.0 architecture is strong on governance, determinism, and supply chain. Reading it against the reference projects, four gaps stand out — all of them in the **live-execution and reconciliation** layer, which is exactly the part that no amount of architecture rigor substitutes for.

| # | Gap | Why it matters | Proposed owner |
|---|---|---|---|
| **G20** | **Broker error taxonomy and account health state machine.** v1.0 defines order intent and risk gates but not what happens when a venue returns an ambiguous, transient, or permanent failure. | This is where real money is lost. A misclassified `403` (market-closed vs auth-failure) either disables a healthy account or retries into a rate-limit ban. | New §19.6, modeled on §3.1 below |
| **G21** | **Position reconciliation after uncertainty.** v0.4 §31 says "state is reconciled after uncertainty" — one sentence, no mechanism. | Your own README already says "any ambiguous order state halts the session rather than guessing." Halting is correct for v0.1 but does not scale past one session. | New §19.7 |
| **G22** | **Cost basis provenance.** No field distinguishes a broker-reported average cost from one PRISMATIK reconstructed itself. | Silent conflation corrupts realized P&L and therefore every performance metric and tax lot downstream. | Extend `Position` in §11 |
| **G23** | **Local inference plane.** v1.0 §20 assumes hosted models. No contract for a local OpenAI-compatible endpoint, its capability envelope, or its determinism posture. | You have decided on LM Studio. See §5. | New §20.6 |

---

## 3. Harvest: Patterns Worth Reimplementing Clean-Room

### 3.1 OpenAlice's Unified Trading Account — the single best find

`packages/uta-protocol` defines one `IBroker` interface implemented by Alpaca, CCXT, IBKR, LongBridge, and LeverUp. It is AGPL, so **read it, then close it**. Five design decisions in it are worth adopting outright, and all five are facts about brokers rather than code:

**(a) Error classification is a first-class type with a permanence bit.**

```rust
pub enum BrokerErrorCode {
    Config,        // permanent — disables the account
    Auth,          // permanent — disables the account
    Network,       // transient — auto-recover
    Exchange,      // transient — venue rejected, do not retry blindly
    MarketClosed,  // transient — expected, not a failure
    Connecting,    // "data pending, retry shortly" — NOT a failure
    Unknown,
}
```

The `Connecting` state is the non-obvious one and it is the reason this design works. It means *the account is mid-connect or mid-recovery*. A read during `Connecting` returns immediately without blocking, **without counting as a health failure**, and without disabling the account. Systems that lack this state either block their whole read path on a slow venue handshake or spuriously degrade account health during normal reconnects.

The second non-obvious rule: **classify `MarketClosed` before `Auth`.** Venues return `403` for both. Getting the order wrong means a routine after-hours read permanently disables a healthy account.

**(b) Every monetary field is a string; arithmetic goes through a decimal type.** `avgCost`, `marketPrice`, `marketValue`, `unrealizedPnL`, `realizedPnL` — all strings at the boundary, `rust_decimal` for math. This is unglamorous and it is the single highest-leverage correctness decision in a trading system. IEEE-754 artifacts in position math are silent, compounding, and only surface during reconciliation when the numbers no longer tie out.

**(c) `multiplier` is mandatory, not optional.** Shares-per-contract: `1` for equities/crypto/forex, `100` for US equity options, venue-specific for futures (ES = `50`), issuer-specific and sometimes non-integer for structured products. OpenAlice's own comment records that they made it optional first and had to force it later — every broker must *declare* a value rather than inherit an implicit `1`. Given your Phase 3 options plane, make it required from the start.

**(d) Cost-basis provenance — this closes G22.**

```rust
pub enum AvgCostSource {
    Broker,  // venue reported it directly — authoritative
    Wallet,  // venue has no cost basis (CCXT spot from fetchBalance) —
             // PRISMATIK reconstructed it from its own journal
}
```

Crypto spot via CCXT frequently has *no* real cost basis; you are synthesizing it from your own order history. That must be visible in the type, must flow into the evidence graph, and must be surfaced in the UI wherever a P&L figure derived from a `Wallet` basis is displayed. This is precisely the "user-facing evidence and uncertainty contract" your v0.4 §22 already claims as a trust boundary — here is a concrete instance of it.

**(e) Leverage/liquidation live in a nested `risk` struct, present only for perps.** Deliberately *not* flattened into `Position`. On IBKR, margin is an account-level concept living on `OrderState` and account-summary tags; on CCXT isolated-margin perps it is genuinely per-position. Nesting it means `position.risk?.leverage` returns `None` for spot rather than a misleading implicit `1×`. Same principle as (d): make the absence of information representable.

**(f) Trading modes with an environment lock.** `lite | readonly | pro`, resolved as env → persisted config → auto-detect, where env is `envLocked` and cannot be overridden at runtime. Credentials at rest are sealed (AES-GCM envelope with `$sealed`/`iv`/`tag`/`data`). This composes cleanly with your existing double-gate: the env lock is a *third*, coarser gate that can put an entire deployment into readonly without touching session logic.

> **ADR-024** — Broker abstraction and error taxonomy. Clean-room; design observed in OpenAlice (AGPL, not copied).
> **ADR-025** — Decimal discipline and monetary field representation at all boundaries.
> **ADR-026** — Cost basis provenance and the uncertainty contract for synthesized bases.

### 3.2 nofx — the reconciliation layer (closes G21)

Go, AGPL, but the *file layout* tells you what a live multi-venue trader actually needs, and it is more than v1.0 currently names:

- `position_rebuild.go` / `position_snapshot.go` — rebuild authoritative position state from venue truth after any uncertainty window.
- `runtime_health.go` — per-venue health independent of per-account health.
- `auto_trader_throttle.go` — request pacing distinct from the venue's own rate limit.
- `syncloop/` — continuous background reconciliation, not one-shot at startup.
- `grid_regime.go` + `docs/market-regime-classification-en.md` — regime classification gating strategy behavior.

The pattern to adopt: **reconciliation is a continuous loop, not a startup step.** Your current design halts the session on ambiguity, which is right for v0.1 and correct as a *fallback*, but the production shape is: snapshot → diff against journal → classify divergence → auto-heal the benign cases → halt only on the genuinely irreconcilable. Add `prismatik-reconciliation` as a crate in Phase 6, before Phase 7 controlled execution.

nofx also carries `docs/token-estimation.zh-CN.md` and `docs/prompt-guide.md` — LLM cost governance as a documented engineering concern. That belongs in your §16 provider cost-governance plane, extended to cover model tokens alongside market-data API quota.

### 3.3 ai-market-maker — risk veto placement

AGPL. One structural decision worth naming: the **Risk Guard is a hard veto positioned after portfolio logic and before the OMS**, not a filter inside strategy code. Multi-agent "desks" propose; a single deterministic gate disposes. This matches the 2026 literature consensus in §4.2 and matches your v0.4 §31 rule that "workers cannot bypass risk evaluation." Make it structurally impossible rather than policy-forbidden: the OMS should accept only a `RiskApproved<OrderIntent>` newtype that no code path outside the risk kernel can construct.

Also worth copying as discipline: **every backtest reports excess return versus buy-and-hold by default.** Your README already reports it. Make it non-optional in the report schema — a strategy result that omits the benchmark should fail validation, not render.

### 3.4 ai-trading-claude — MIT, directly harvestable

Five parallel agents (technical / fundamental / sentiment / risk / thesis) → composite score 0–100 → discrete signal. MIT-licensed, so this one you *can* draw from directly. Two things to take:

1. **The fan-out/synthesize shape** for your §20 AI plane — independent analysts with disjoint evidence scopes, then a synthesis pass. Disjoint scopes matter: it prevents the correlated-error failure mode where all five agents read the same news article and "independently agree."
2. **The composite score must expose its decomposition.** A 0–100 with no visible contribution breakdown is exactly the opaque-scoring anti-pattern your evidence plane exists to prevent. Ship the sub-scores, their weights, and the evidence IDs behind each.

### 3.5 QuantDinger — Apache-2.0, closest full-stack analogue

The end-to-end loop you are building, already assembled: *AI research → strategy code → backtest → paper → live → monitoring*, Postgres 18 + Redis 8, an MCP server, and a dedicated `docker-compose.observability.yml`. Permissively licensed, so it is safe to study closely and to borrow from where useful.

Two specifics: (a) they ship observability as a **separate, always-present compose file** rather than an afterthought — adopt that for your §26 plane; (b) their `mcp_server/` is a working reference for exposing a trading platform to agents over MCP, which de-risks your §20.3 `rmcp` work.

### 3.6 adata — the multi-source failover pattern

China A-share data, which you should ignore entirely. The transferable idea is stated in its own README: *"采用多数据源融合切换"* — multi-source fusion with automatic switching, adopted specifically to guarantee availability.

Your provider plane (§16) governs cost and entitlements. It should also govern **degradation**. Formalize:

```rust
pub struct ProviderChain {
    pub primary: ProviderId,
    pub fallbacks: Vec<ProviderId>,
    pub agreement_policy: AgreementPolicy, // require N-of-M for critical reads
    pub divergence_action: DivergenceAction, // Halt | PreferPrimary | FlagAndContinue
}
```

Critically, **provider identity must flow into the evidence graph**. A bar sourced from the fallback is not the same evidence as one from the primary, and a backtest that silently mixed them is not reproducible. AI-Trader's changelog shows the same pattern in the US market (Alpha Vantage primary → yfinance fallback on rate-limit or empty response), which confirms this is a general need and not a China-market artifact.

### 3.7 Kronos — MIT, the one model artifact to prioritize

Already in v1.0 §6.7, correctly. Reinforced by 2026 evidence (§4.1): AAAI 2026, decoder-only, pretrained on 12B K-line records from 45 exchanges, with a hierarchical tokenizer using Binary Spherical Quantization over OHLCV bars. Reported zero-shot: +93% RankIC over the leading general-purpose TSFM, −9% volatility-forecast MAE vs the strongest baseline, +22% generative fidelity on synthetic K-line sequences.

That last number is the underrated one for you. **Generative fidelity on synthetic sequences is what makes Kronos useful to your Monte Carlo lab**, not just to point forecasting — it is a regime-aware path generator for backtest augmentation and stress scenarios. The local copy in `reference/Kronos-master/` includes `finetune/` with Qlib integration and a `webui/`, which maps directly onto your §6.7 Phase 5 plan.

---

## 4. Bleeding Edge: 2026 Findings Not in v1.0

### 4.1 TSFM landscape — v1.0's table holds, with refinements

Your §6.7 artifact table is correct as written. Updates worth folding in:

- **Chronos-2** (Amazon, Oct 2025) is the current general-purpose leader on fev-bench, GIFT-Eval, and Chronos Benchmark II, beating TimesFM-2.5 and TiRex on win rate and skill score under both WQL and MASE, and beating Chronos-Bolt in >90% of head-to-head comparisons. Encoder-only, group + time attention, 120M params. Apache-2.0. Native zero-shot covariate support — which matters for you, since it lets a forecast condition on options-flow or filing events without retraining.
- **Caveat to record in the model registry:** on GIFT-Eval, only Chronos-2 and TimesFM-2.5 beat Auto-Theta at *secondly* frequency, and foundation models get relatively stronger as frequency drops. This is a patching artifact — fixed patch schemes bias toward low-frequency components. **Practical consequence: do not assume a TSFM beats a classical baseline at intraday resolution. Your §17 calibration plane must benchmark against Auto-Theta / ARIMA / GARCH per frequency band, not once globally.**
- Independent evaluation (Cisco's technical report) places TimesFM-2.5, Chronos-2, and Toto-1.0 within a few points on CRPS/MASE. Treat published leaderboard margins as noise-adjacent; your own conformal calibration is the arbiter.
- **Moirai / Moirai-2 remain CC BY-NC 4.0.** Your v1.0 registry hard-deny at commercial registration is the correct handling — keep it.
- Two papers to add to Appendix B: *Re(Visiting) Time Series Foundation Models in Finance* (arXiv:2511.18578) and *Forecasting Realized Volatility with TSFMs vs Econometric Benchmarks* (arXiv:2607.05291). The latter is the direct GARCH-family comparison your §17.3 needs.

### 4.2 Agentic trading — the literature now names your architecture

*Agentic Trading: When LLM Agents Meet Financial Markets* (arXiv:2605.19337) taxonomizes deployed systems into two families:

- **Adaptive** — the LLM selects tools and actions at execution time (AI-Trader is their example).
- **Procedural** — predefined stages: analysis → decision → execution (**nofx is their named example**, which is why it is in your `reference/` folder and worth the read).

Their finding: procedural is materially easier to gate; adaptive requires a runtime firewall. PRISMATIK's `StrategyIR` + deterministic runtime is procedural by construction — that is a **defensible security property, and you should say so explicitly in §20**, because it is a genuine differentiator against the adaptive-agent products you compete with.

The consensus blueprint from this literature — worth stating verbatim in §20 as the AI plane's invariant:

> event-triggered LLM activation → typed tool schemas only → read-only truth store → proposal object → deterministic N-gate risk engine → dry-run/paper by default → broker adapter → WORM audit → offline replay and eval harness.
> **The LLM never holds write authority over anything it can also hallucinate about.**

Specific items to add:

- **Audit-log integrity.** An HMAC-chained log detects in-place tampering *only if the key is not in the same process*. A root-level attacker rewrites the chain otherwise. Your §14 ledger needs a **WORM mirror** — S3 Object Lock in governance mode, or a local append-only device with an off-box anchor. Merkle-anchoring the ledger head to an external transparency log (you already use Rekor for artifacts) gives you the same property without a cloud dependency.
- **Adversarial evaluation is now a named discipline.** AgentDojo (prompt injection against tool-using agents over untrusted data), LlamaFirewall, GuardAgent, InferAct. Your §27 testing layers should include a prompt-injection suite — market commentary, filings text, and news are all untrusted input that reaches your AI plane. A malicious 8-K is a realistic attack vector for a filings-aware product.
- **TradeTrap** (arXiv:2512.02261) tests whether LLM trading agents are *faithful* — whether stated reasoning matches actual decision drivers. Directly relevant to your explainability claims. If you surface agent reasoning as evidence, faithfulness is a correctness property, not a UX nicety.
- **Execution Assumptions and Reproducibility in LLM-Based backtests** (arXiv:2606.08285) — cite in §27.2 alongside your replay harness.
- **Determinism at the inference layer:** temperature 0.0, bounded tool rounds per step, stateless-per-step agents where the harness constructs the state summary. This composes with your determinism kernel: the LLM call is a *seeded, recorded* effect, and replay must return the recorded response rather than re-invoking the model.

### 4.3 Regulatory clock — one date to put on the roadmap

- **EU AI Act high-risk obligations: 2 August 2026.** Automated decision-making affecting access to financial services is explicitly high-risk. That is **eight days from this document's date.** If Phase 8 (enterprise/on-prem) targets EU customers, the compliance plane (§22) is no longer a Phase 8 concern — it is a Phase 0 architectural constraint, because retrofitting logging, human-oversight, and technical-documentation requirements is far more expensive than designing for them.
- **SR 11-7 / OCC 2011-12**, plus the OCC's 2024 generative-AI update, define "model" broadly enough that your TSFM *and* your LLM plane require inventory, validation, and ongoing monitoring. Your §17 model registry already satisfies most of this — say so explicitly in §22, because "we already have a compliant model inventory" is a concrete enterprise sales asset.

### 4.4 Wasm plugin host — sharpen §6.4

Your v1.0 picks wasmtime (Mode A, Cranelift-only) with Extism as a Phase 4 prototype. 2026 evidence refines this:

- **Prefer `wasm32-wasip2` with the Component Model and WIT** over Extism's PDK. WASI Preview 2 stabilized April 2026; the Component Model ships in Wasmtime. Extism is still on P1-era targets. For a typed, language-agnostic, capability-scoped plugin boundary — which is exactly what your §20.4 needs — the Component Model is the right substrate. **Revised recommendation: wasmtime + WIT directly, Mode A; Extism drops to Mode E reference.** This removes a dependency and a Phase 4 prototype from the plan.
- **WASI 0.3** adds native async to the Component Model with `stream<T>` and `future<T>`. Relevant if plugins consume streaming market data. Track it; do not block on it.
- **Concrete hardening, all host-side:** epoch-based interruption (also enables hot-swapping a running module with a patched one), fuel metering, hard memory ceilings, and a `StoreLimits` config such that a guest cannot terminate the host. Recent CVEs — CVE-2026-27572 (`wasi:http` header DoS) and CVE-2026-27204 (resource exhaustion) — are mitigated by exactly this configuration. **The sandbox is only as good as the host config; the failure mode is always an over-permissive host function, not a wasm escape.**
- **Plugin supply chain:** wasm binaries are harder to review than source. Gate with `wasm-tools` disassembly, **capability diffing on imports/exports between versions**, and cosign verification before load. Capability diffing is the load-bearing check — a plugin update that newly imports a network capability must fail the gate automatically, not await human review.

### 4.5 Rust engine landscape — one correction to a common assumption

NautilusTrader is migrating to pure Rust/PyO3 (v2), MSRV 1.96, with a `high-precision` feature switching value types between 64-bit and 128-bit integers. Their catalog is **Parquet on `object_store`, queried with DataFusion; transport is Cap'n Proto + tonic/prost — not Arrow Flight.**

Worth noting because Arrow Flight is a common default assumption for market-data transport. Neither Nautilus nor barter-rs uses it. Your v1.0 DataFusion adoption (§6.2) matches their proven shape exactly. If you want Flight, that is a green-field decision to justify on its own merits, not an industry convention to inherit.

Their `high-precision` 128-bit feature flag is worth mirroring in `prismatik-determinism`: crypto quantities routinely need more than 64-bit fixed-point precision, and making it a compile-time feature keeps the fast path fast.

---

## 5. Local Inference: LM Studio as the AI Plane Backend (closes G23)

Your decision: LM Studio hosting models in the ≤16GB class. This is a good fit and it strengthens several existing invariants rather than compromising them.

### Why it fits

- **Evidence never leaves the machine.** Your product is self-hosted and its moat is a governed evidence graph containing the user's positions, journal, and thesis notes. Sending that to a hosted API is the single largest privacy objection an enterprise buyer will raise. Local inference removes it structurally.
- **Cost governance becomes trivial** for the local tier — no per-token spend, so §16 cost governance applies only to the escalation tier.
- **Reproducibility improves.** A pinned local model with pinned weights and temperature 0.0 is far closer to a deterministic effect than a hosted endpoint that can be silently updated beneath you.

### Architectural contract

LM Studio exposes an OpenAI-compatible server (default `http://localhost:1234/v1`). Model it as a **provider under the existing §16 provider plane**, not as a special case:

```rust
pub struct InferenceProvider {
    pub id: ProviderId,
    pub kind: InferenceProviderKind, // LmStudioLocal | OpenAiCompatible | Hosted
    pub endpoint: Url,
    pub model_id: String,
    pub model_digest: ContentHash,   // pinned weights — required for reproducibility
    pub context_window: usize,
    pub capabilities: InferenceCapabilities, // tool_use, json_schema, vision, reasoning
    pub determinism: DeterminismProfile,     // temperature, seed, top_p — pinned
    pub egress: EgressPolicy,                // Loopback for local — enforced, not assumed
    pub max_tokens_per_call: usize,
    pub cost_model: CostModel,               // Zero for local
}
```

Four rules:

1. **`model_digest` is required.** "Qwen3-14B" is not a reproducible identifier. Pin the exact quantized artifact hash. LM Studio can swap the loaded model without your knowing; the digest is how a replay detects it.
2. **`egress: Loopback` is enforced, not documented.** The local provider must be structurally incapable of reaching the network. This is the property that makes the privacy claim defensible in an enterprise security review.
3. **Every inference call is a recorded effect in the determinism kernel.** Replay returns the recorded response. This is non-negotiable regardless of backend.
4. **Capability declaration drives routing, not model name.** A 14B model's tool-use and structured-output reliability differ sharply from a hosted frontier model's. Declare capabilities explicitly and let the router degrade gracefully.

### Model tiering in a 16GB envelope

At ≤16GB VRAM you are realistically running 7B–14B class models at 4–5 bit quantization, or ~20–30B MoE architectures with a small active-parameter count. That is genuinely capable for *structured, bounded* work and unreliable for *open-ended, long-horizon* reasoning. Route accordingly:

| Tier | Workload | Model class | Notes |
|---|---|---|---|
| **T0 — deterministic** | Symbology normalization, field extraction, classification, tagging | 3B–7B instruct, 4-bit | No reasoning needed. Schema-constrained output. Fastest path. |
| **T1 — local default** | Filing summarization, evidence extraction, chart-tool composition, journal drafting | 7B–14B instruct, Q4_K_M / Q5 | Bounded context, strict JSON schema, single-pass. The workhorse tier. |
| **T2 — local reasoning** | Multi-evidence synthesis, thesis critique, risk narrative | 14B reasoning-tuned, or ~30B-A3B MoE | Slower. Budget the latency; do not put it on an interactive path. |
| **T3 — escalation** | Genuinely hard synthesis, adversarial critique, code generation | Hosted frontier, **user opt-in per call** | Must surface exactly what evidence would leave the machine, and require consent. |

Design implications:

- **Constrained decoding is mandatory at T0/T1.** Use LM Studio's JSON-schema / structured-output support and reject anything that fails schema validation. Do not parse free text from a 7B model. This alone eliminates the majority of small-model failure modes.
- **Keep prompts short and evidence explicit.** Small models degrade sharply with long context. Retrieve narrowly (this is what LanceDB in §15.4 is for), pass IDs and short excerpts, never dump a full 10-K.
- **Model swap must invalidate cached inference.** Cache keys include `model_digest` + `determinism_profile` + prompt hash.
- **A quantized model is a third-party artifact.** It enters §17 model governance with the same manifest, license check, and provenance requirements as a TSFM. Quantized community re-uploads on HuggingFace frequently have unclear provenance relative to the base model's license — that check is not optional.
- **T3 escalation is a consent surface, not a config flag.** Show the user the exact payload. This is the same uncertainty-contract discipline as §3.1(d).

> **ADR-027** — Local inference plane: LM Studio as an OpenAI-compatible provider, tiered routing, loopback egress enforcement, digest pinning.

---

## 6. Revised Backlog Deltas

### Add to Phase 0 (Foundation)

1. `reference/LICENSE_DISPOSITION.md` + build/SBOM exclusion for `reference/`.
2. Register all 21 reference projects as `ValidationOracleOrReference` manifests — real data through the intake pipeline on day one.
3. Decimal discipline: `rust_decimal` at every boundary, string representation in all serialized monetary fields. **This gets harder to retrofit every week.**
4. Compliance-plane skeleton, moved up from Phase 8 — EU AI Act high-risk obligations land 2 Aug 2026.
5. Inference call as a recorded effect in `prismatik-determinism`.

### Add to Phase 1 (Crypto MVP)

6. `BrokerError` taxonomy with the permanence bit and the `Connecting` state (§3.1a).
7. `ProviderChain` with declared fallbacks and evidence-graph provenance (§3.6).
8. LM Studio provider + T0/T1 tiers with schema-constrained decoding (§5).
9. `Position` with required `multiplier` and `avg_cost_source` (§3.1c/d).

### Add to Phase 4 (Plugin Host)

10. **Drop the Extism prototype.** Go direct to wasmtime + Component Model / WIT on `wasm32-wasip2` (§4.4).
11. Capability-diffing gate on plugin imports/exports, plus cosign verification at load.

### Add to Phase 5 (TSFM)

12. Per-frequency-band baseline benchmarking — Auto-Theta / ARIMA / GARCH, not one global comparison (§4.1).
13. Kronos as the K-line specialist *and* as the synthetic path generator for the Monte Carlo lab (§3.7).

### Add to Phase 6 (Portfolio / Paper)

14. `prismatik-reconciliation` — continuous sync loop, divergence classification, auto-heal for benign cases, halt only on the irreconcilable (§3.2). **Land this before Phase 7 controlled execution, not during.**

### Add to Continuous Tracks

15. Prompt-injection adversarial suite (AgentDojo-style) over filings, news, and market commentary.
16. WORM mirror + external anchoring for the audit ledger (§4.2).

---

## 7. Honest Assessment

Three things I would flag if I were reviewing this architecture cold:

**The v1.0 document is unusually strong on governance and unusually thin on execution mechanics.** Determinism kernel, audit ledger, supply chain, conformal calibration — all well specified. Broker error handling, reconciliation, and cost-basis provenance get a sentence each. That asymmetry is backwards relative to where trading systems actually fail. Gaps G20–G22 are the correction, and they are the most valuable thing in this addendum.

**The reference folder's licensing means the plan's assumed velocity is optimistic.** Fourteen of twenty-one projects are read-only. "Pull as much from these reference projects as possible" resolves, correctly, to *pull as much understanding as possible and write the code yourself*. That is slower than it looks on a Gantt chart, and worth planning for honestly now rather than discovering in Phase 1.

**The 2 August 2026 EU AI Act date is eight days out.** I do not know whether EU customers are in scope for you — if they are not, this is a non-issue and you can ignore it. If they are, the compliance plane cannot stay in Phase 8. I have flagged it rather than assumed either way, because it is the one item here that changes phase ordering.

---

## Appendix — Sources

**Reference projects (local, `D:\DevOps\PRISMATIK\reference\`)** — licenses verified by reading each `LICENSE` file, 2026-07-26.

**Web, verified 2026-07-26:**

- [NautilusTrader](https://github.com/nautechsystems/nautilus_trader) · [Architecture](https://nautilustrader.io/docs/latest/concepts/architecture/) · [Rust developer guide](https://nautilustrader.io/docs/latest/developer_guide/rust/) · [barter](https://docs.rs/barter)
- [Chronos-2: From Univariate to Universal Forecasting](https://arxiv.org/pdf/2510.15821) · [Amazon Science announcement](https://www.amazon.science/blog/introducing-chronos-2-from-univariate-to-universal-forecasting) · [chronos-forecasting](https://github.com/amazon-science/chronos-forecasting)
- [Kronos: A Foundation Model for the Language of Financial Markets](https://arxiv.org/abs/2508.02739) · [AAAI 2026](https://ojs.aaai.org/index.php/AAAI/article/view/39730/43691)
- [Re(Visiting) Time Series Foundation Models in Finance](https://arxiv.org/pdf/2511.18578) · [Forecasting Realized Volatility with TSFMs vs Econometric Benchmarks](https://arxiv.org/pdf/2607.05291) · [TSFM strengths and limitations](https://aihorizonforecast.substack.com/p/time-series-foundation-models-a-deep) · [Cisco Time Series Model Technical Report](https://arxiv.org/pdf/2511.19841)
- [Agentic Trading: When LLM Agents Meet Financial Markets](https://arxiv.org/html/2605.19337v1) · [TradeTrap](https://arxiv.org/pdf/2512.02261) · [FinHarness](https://arxiv.org/pdf/2605.27333) · [Execution Assumptions and Reproducibility in LLM-Based Backtesting](https://arxiv.org/pdf/2606.08285) · [AgenticAITA](https://arxiv.org/pdf/2605.12532) · [Type-Checked Compliance with Lean 4](https://arxiv.org/pdf/2604.01483) · [AgentTrading: risk-gated, custody-aware](https://medium.com/@gwrx2005/agenttrading-a-risk-gated-custody-aware-autonomous-trading-agent-for-cryptoassets-architecture-620f522297f6)
- [AI agent compliance for financial services](https://fin.ai/learn/evaluate-ai-agent-compliance-financial-services) · [LLM guardrails for fintech](https://www.getmaxim.ai/articles/llm-guardrails-for-fintech-compliance-hallucination-prevention-and-audit-trails/)
- [WASI.dev](https://wasi.dev/) · [WASI and the Component Model: current status](https://eunomia.dev/blog/2025/02/16/wasi-and-the-webassembly-component-model-current-status/) · [wasmCloud: Component Model, shared memory, WASI 0.3](https://wasmcloud.com/community/2025-09-10-community-meeting/) · [WebAssembly hardening](https://www.systemshardening.com/articles/wasm/) · [moonrepo WASM plugins](https://moonrepo.dev/docs/guides/wasm-plugins)
