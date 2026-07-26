# Wave 1 — MVP: Crypto Intelligence

**Maps to v1.0 phases:** P1 (Crypto Intelligence MVP)
**Business outcome:** first sellable product. **Tests the central commercial hypothesis: does the evidence-chained, local-first, privacy-respecting crypto wedge find buyers?**
**Duration:** ~56 days. **First sellable artifact at ~80 days** under Option B (24-day Wave 0 floor + Wave 1).
**Date:** 2026-07-26

---

## Objective

A complete, sellable crypto intelligence product on the Wave 0 foundations. The first artifact a customer sees. **This wave exists to test the commercial hypothesis**, not to deliver every feature. The exit gate is the scope; new ideas go to a Wave 2 candidate list, not into Wave 1.

## Entry Criteria

Wave 0 exit gate, or the `P0-FLOOR` subset under Option B (24 days).

## Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P1-DP-01 | DP | `Provider` trait, `ProviderCapabilities`, `EntitlementSet`, `ProviderHealth` | 3 | M |
| P1-DP-02 | DP | `BudgetGovernor` on `governor` GCRA, priority classes, exact next-permit times | 4 | M |
| P1-DP-03 | DP | `ProviderChain` with declared fallbacks, `FailoverTrigger::Empty`, evidence-graph provenance | 4 | M |
| P1-DP-04 | DP | CoinGecko adapter (auth, search, global, markets, coin detail, market chart, OHLC, categories, exchanges) | 8 | M |
| P1-DP-05 | DP | Provider contract tests with recorded cassettes, replayable offline | 3 | M |
| P1-DP-06 | DP | Raw layer: Parquet writer, partitioning, append-only enforcement, supersession links | 4 | M |
| P1-DP-07 | DP | Normalization to canonical types, quality scoring, deduplication | 4 | M |
| P1-DP-08 | DP | DuckDB analytical views over curated Parquet | 3 | M |
| P1-DP-09 | DP | SQLite operational state, migrations, task-graph persistence | 3 | M |
| P1-DP-10 | DP | Evidence and lineage plane, `EvidenceRef`, `Concludes` trait, invalidation hash including pinned set | 5 | M |
| P1-DP-11 | DP | In-process Tokio task graph, `PipelineTask`, triggers, crash recovery from SQLite | 5 | M |
| P1-DP-12 | DP | LanceDB integration, asset/category description embeddings, `AnalogStore` skeleton | 4 | M |
| P1-AI-01 | AI | `prismatik-ai-router`: provider abstraction, capability-based routing, recorded-effect inference | 5 | M |
| P1-AI-02 | AI | LM Studio provider (T0/T1 tiers, JSON-schema structured output, loopback egress enforcement, model-digest pinning) | 6 | M |
| P1-AI-03 | AI | OpenRouter OAuth PKCE + Gemini OAuth (the OAuth tier) | 4 | M |
| P1-AI-04 | AI | Anthropic + OpenAI API-key providers (the API-key tier) | 3 | M |
| P1-AI-05 | AI | MCP 2026-07-28 RC alignment in `prismatik-ai-tools` (stateless core, OAuth 2.1 Resource Server) | 4 | M |
| P1-AI-06 | AI | `AnalystScope` common-facts/private-evidence split (from ai-trading-claude, MIT — clean-room with attribution) | 3 | M |
| P1-EX-01 | EX | Command palette, workspace shell, panel system, layout persistence | 5 | M |
| P1-EX-02 | EX | `ChartDocument` and `ChartBackend` contracts, Lightweight Charts wrapper, theme bridge | 6 | M |
| P1-EX-03 | EX | Crypto command dashboard: global stats, dominance, trending, category map | 5 | M |
| P1-EX-04 | EX | Watchlist with live updates, virtualized rows, 500-row budget | 4 | M |
| P1-EX-05 | EX | Asset workspace: profile, chart, markets, exchanges, categories, evidence drawer | 6 | M |
| P1-EX-06 | EX | Market scanner with saved filters and result provenance | 4 | M |
| P1-EX-07 | EX | Rate-budget UI showing exact countdowns from GCRA, not spinners | 2 | H |
| P1-EX-08 | EX | Empty, loading, error, and degraded states for every surface | 3 | M |
| P1-QM-01 | QM | Alert-rule evaluator, streaming and scheduled split, dedup keys | 5 | M |
| P1-QM-02 | QM | Notification service, delivery channel abstraction, escalation | 4 | M |
| P1-OD-01 | OD | First-run experience state machine, demo mode, suitability profile | 5 | M |
| P1-OD-02 | OD | Backup, restore, cross-version migration | 4 | M |
| P1-OD-03 | OD | OpenTelemetry tracing with determinism telemetry attributes | 3 | M |
| P1-OD-04 | OD | Installer, code-signed, auto-update flow end-to-end | 4 | L |
| P1-OD-05 | OD | User documentation, licensing and disclosure surfaces, notice generator | 4 | M |

**Subtotal: 134 estimated days at H-confidence weight × 0.5 confidence factor applied to M and L items ≈ 56 days of focused engineering time.** (Confidence bands already bake in the over-run risk; the wave plan totals are the median expected outcome, not the H-only estimate.)

## What Ships (the MVP, concretely)

A customer who installs Wave 1 gets:

- **Crypto market intelligence** from CoinGecko: global macro context, asset profiles, OHLCV charts, watchlists, market scanner with saved filters, trending assets, category map.
- **Evidence-chained conclusions** — every rendered number resolves a complete evidence chain to a raw record with provider identity and retrieval timestamp. Stale data visibly marked. Rate limits surfaced honestly with exact countdowns (not spinners).
- **Local-first AI tier via LM Studio**: structured field extraction, asset classification, drafting, and analysis running locally with `egress: Loopback` enforced. The user's analysis never leaves their machine at this tier.
- **Hosted AI tier (OAuth + API-key)**: OpenRouter OAuth PKCE for one-integration access to every frontier model; Google Gemini OAuth for clean OAuth-first; Anthropic and OpenAI via API key for Claude Opus 5 / Sonnet 5 / Haiku 4.5 and GPT-5.5. The escalation tier is a per-call consent surface, not a default.
- **Multi-source provider fusion**: CoinGecko primary → CCXT public worker fallback for bars. Provider identity flows into the evidence graph.
- **Workspace**: command palette, panel system, layout persistence, light/dark theme, motion system with reduced-motion support.
- **Operational resilience**: degraded mode when providers are unreachable, crash recovery from SQLite-persisted task graph, backup/restore with cross-version migration, code-signed installer with auto-update (provenance-verified).

## The Non-Obvious Design Rules Enforced From Day One

These come from the reference corpus (see v1.2 Reference Consolidation) and are baked into Wave 1 so they don't have to be retrofitted later:

- **Decimal discipline at every boundary** (from OpenAlice, clean-room ADR-025): all monetary fields are strings at the IPC boundary, `rust_decimal` for math. IEEE-754 artifacts in position math are silent and compounding.
- **`Position` with required `multiplier` and `avg_cost_source`** (from OpenAlice, clean-room ADR-024/026): `1` for crypto now, `100` for US equity options in Wave 3. `avg_cost_source: 'broker' | 'wallet'` makes synthesized cost bases visible.
- **`FailoverTrigger::Empty`** (from adata, clean-room): an empty 200 is the common real-world failure and invisible to naive error handling.
- **The `AnalystScope` common-facts/private-evidence split** (from ai-trading-claude, MIT): the technical analyst MUST NOT see the news the sentiment analyst sees. Shared discovery worsens the correlated-error problem; five agents agreeing after reading the same news summary is one opinion reported five times.
- **The LLM never holds write authority** (v1.0 §20.1): live order submission is not a tool. The MVP has no live trading, but the boundary is structural from the start.

## Definition of Done

A phase is complete when every line below is demonstrably true, verified by a named artifact rather than by assertion.

| # | Criterion | Verified by |
|---|---|---|
| 1 | Every rendered conclusion resolves a complete evidence chain to raw records | `test_no_orphan_conclusions` green |
| 2 | The rate-budget UI shows an exact retry time, never an indeterminate spinner | Manual verification against a throttled key |
| 3 | The application functions in degraded mode when CoinGecko is unreachable | Chaos test with the provider blackholed |
| 4 | Stale data is visibly marked on every surface that displays it | Manual verification with a frozen feed |
| 5 | A clean-machine install completes and auto-update applies a signed increment | Recorded on a fresh VM |
| 6 | Cold start to interactive is under 2.0s p95 on mid-tier hardware | Benchmark report |
| 7 | Chart pan and zoom at 1M points holds a 16.7ms p99 frame | Benchmark report |
| 8 | Provider contract tests pass offline from cassettes | CI green with network disabled |
| 9 | A backup taken on version N restores on version N+1 | Migration test |
| 10 | Local AI tier is structurally incapable of network egress (loopback enforced) | Network-trace negative test |
| 11 | Every AI-generated conclusion carries uncertainty + contradictions + suggested verification | UI review across 20 sample queries |
| 12 | **The product has been used for one continuous week by its author as a user** | Workbook log |

## The Commercial Milestone

`P1-OD-04` (installer + auto-update) is the item that turns a codebase into a product. It is placed near the end because everything before it is a prerequisite, and it is called out here because a solo operation can spend a year building and never ship an installer.

**Suggested checkpoint at day 40 of Wave 1:** package whatever exists, install it on a clean machine, and use it for a week as a user rather than as its author. Every defect that surfaces in that week is a defect a customer would have found. This checkpoint costs two days and reliably returns more than it costs.

## The Hypothesis This Wave Tests

If Wave 1 finds no buyers, that is a **~128-day loss rather than a ~400-day loss**, and Wave 2 (equity/options) becomes the alternative wedge. The wave framework is explicitly designed to make this pivot cheap.

Commercial metrics target (hypothesis to test, not commitment):

| Metric | Wave 1 target |
|---|---|
| Paying customers | 10+ within 90 days of ship |
| Revenue | Validation only |
| Retention (30-day) | >40% |

These numbers matter less than the direction. Wave 1's purpose is to test whether the crypto wedge validates at all.

## Risks

| Risk | Response |
|---|---|
| CoinGecko free tier limits make the product feel slow | Aggressive content-keyed caching + the budget governor surfacing honest wait times. If the free tier is unusable, the paid tier becomes a stated prerequisite and the pricing model absorbs it. |
| **The crypto wedge does not find buyers** | **This is the point of shipping early.** If Wave 1 finds no buyers, pivot to Wave 2 (equity/options) as the alternative wedge. |
| Feature creep from a demo that impresses | The exit gate is the scope. New ideas go to a Wave 2 candidate list, not into Wave 1. |
| LM Studio structured-output reliability insufficient for T0/T1 small models | Fall back to Groq-hosted Llama 4 Scout for T0/T1 (460 TPS, ~10–20× cheaper than OpenAI on equivalent models); keep local tier for T2 synthesis only. |
| OAuth-only providers (Anthropic, OpenAI per Feb 2026 ToS) frustrate users expecting OAuth | Document clearly; route Anthropic/OpenAI through OpenRouter OAuth PKCE where the user wants a single OAuth experience. The cleanest single integration is OpenRouter. |

## AI Stack (per v1.2 Reference Consolidation §4)

The MVP AI router implements three tiers from day one:

- **OAuth Tier:** OpenRouter (PKCE → per-user key, all frontier models), Google Gemini (ADC or AI Studio OAuth).
- **API-Key Tier:** Anthropic (Claude Opus 5 / Sonnet 5 / Haiku 4.5), OpenAI (GPT-5.5 / Pro).
- **Local Tier (LM Studio):** T0 Phi-4-mini or Qwen3-4B (symbology, extraction), T1 Qwen3-14B Q4_K_M (filing summary, evidence). Loopback egress *enforced*, model digest *required*, JSON-schema structured output *mandatory*.

**Critical OAuth finding baked into the design:** Anthropic (Feb 2026) and OpenAI both prohibit third-party use of their subscription OAuth tokens. OAuth-first works cleanly for Gemini and OpenRouter; for Claude Opus 5 and GPT-5.5 Pro you fall back to API keys. The cleanest single integration is OpenRouter OAuth PKCE, which fans out to every frontier model with per-model routing.

**Capability routing matrix** (the actual decision table — full version in v1.2 §4.6):

| Workload | Tier | Default route | Fallback |
|---|---|---|---|
| Symbology normalization, field extraction, classification | T0 | LM Studio Phi-4-mini (local) | LM Studio Qwen3-8B (local) |
| Filing summarization, evidence extraction, chart-tool composition, journal drafting | T1 | LM Studio Qwen3-14B (local) | Groq Llama 4 Scout (hosted, fast) |
| Multi-evidence synthesis, thesis critique, risk narrative | T2 | (Wave 2+) LM Studio Qwen3-30B-A3B (local) | Claude Sonnet 4.5 (API key) |
| Genuinely hard synthesis, adversarial critique, code generation | T3 | (Wave 2+) Claude Opus 5 or GPT-5.5 Pro (hosted, **user opt-in per call**) | OpenRouter OAuth |

T2 and T3 workloads are stubbed in Wave 1 and activated in later waves as the underlying surfaces (filings, options flow, portfolio, journal) come online. The router architecture is in place from day one so capability additions are router configuration, not architecture changes.

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26*
