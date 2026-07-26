# PRISMATIK Provider Adapter Specifications

**Document:** `spec/PROVIDER_ADAPTERS.md`
**Status:** NORMATIVE — RFC 2119 keywords apply
**Companion to:** `spec/CRATE_ARCHITECTURE.md` §3.2 (`prismatik-market-data`), `PRISMATIK_Unified_Solution_Architecture_v1.0.md` §16 (provider plane)
**Date:** 2026-07-26

---

## 0. Purpose

This document specifies, **per provider**, the concrete details an engineer needs to implement a `Provider` adapter: endpoint catalog, auth model, rate limits, error mapping, cassette format for contract tests, normalization rules, and provider-specific quirks.

The cardinal rules (v1.0 §16, v1.2 §3.5):
- Every provider enters through the `Provider` port — there is no other path.
- Provider identity flows into the evidence graph; a bar from the fallback is NOT the same evidence as one from the primary.
- The `provider` column in normalized records records the provider ACTUALLY used per request, NOT the configured chain.
- Entitlements are enforced BEFORE a request is made — an unentitled request is a local error rather than a remote 403 that burns quota.

---

## 1. The Provider Port (Restated)

```rust
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn capabilities(&self) -> ProviderCapabilities;
    fn entitlements(&self) -> &EntitlementSet;
    fn cost_of(&self, request: &ProviderRequest) -> CostUnits;
    async fn health(&self) -> ProviderHealth;
}
```

Each provider adapter implements `Provider` plus a per-capability trait (e.g. `BarsProvider`, `QuotesProvider`, `OptionsFlowProvider`, `FilingsProvider`). The adapter lives in `prismatik-market-data::adapters::<provider_id>`.

---

## 2. CoinGecko (Wave 1 — Crypto MVP)

### 2.1 Endpoint Catalog

| Endpoint | Path | Auth | Capability |
|---|---|---|---|
| Search | `GET /search` | Demo key (query param `x_cg_demo_api_key`) | `AssetSearch` |
| Global | `GET /global` | Demo/Pro | `CryptoGlobalStats` |
| Markets (per-coin) | `GET /coins/markets` | Demo/Pro | `CryptoMarkets` |
| Coin detail | `GET /coins/{id}` | Pro | `CoinDetail` |
| Market chart | `GET /coins/{id}/market_chart` | Pro | `OHLCV` |
| OHLC | `GET /coins/{id}/ohlc` | Pro | `OHLC` |
| Categories | `GET /coins/categories` | Pro | `Categories` |
| Exchanges | `GET /exchanges` | Pro | `Exchanges` |
| Trending | `GET /search/trending` | Demo | `Trending` |

### 2.2 Auth

- **Demo API key:** free, query param `x_cg_demo_api_key`. Rate limited (~30 calls/min, varies).
- **Pro API key:** paid, header `x-cg-pro-api-key`. Higher limits per plan.

**Entitlement enforcement:** the adapter checks entitlements BEFORE making the call. If the user has Demo entitlement and requests `CoinDetail` (Pro-only), return `AdmissionDecision::NotEntitled` locally. No call made.

### 2.3 Rate Limits

- **Demo:** ~30 calls/min, public rate-limit (returns 429 with `Retry-After`).
- **Pro Analyst:** 500 calls/min.
- **Pro Lite/Pro/Enterprise:** higher.

Detection: parse `Retry-After` header on 429. Also parse response body forsoft-limit warnings (CoinGecko occasionally returns 200 with a warning header for approaching limit).

### 2.4 Error Mapping

| CoinGecko response | `BrokerErrorCode` analog | Notes |
|---|---|---|
| 200 OK | — | success |
| 401 Invalid API key | `Auth` (permanent) | disable account; alert user |
| 403 Forbidden | `Exchange` (transient) | check entitlements |
| 429 Too Many Requests | `Network` (transient) | back off per `Retry-After` |
| 500/502/503/504 | `Network` (transient) | back off exponentially |
| Empty 200 (no `data` field) | trigger `FailoverTrigger::Empty` | fall through to chain fallback |

### 2.5 Cassette Format

Contract tests record HTTP exchanges as YAML cassettes (vcr-style) in `tests/cassettes/coingecko/`. Each cassette:

```yaml
- name: search_bitcoin
  request:
    method: GET
    path: /search
    query: { query: "bitcoin" }
    headers: { x_cg_demo_api_key: "$TEST_COINGECKO_KEY" }
  response:
    status: 200
    headers: { content-type: "application/json", retry-after: null }
    body: |
      { "coins": [{ "id": "bitcoin", "name": "Bitcoin", ... }] }
    recorded_at: 2026-07-15T14:32:00Z
```

**Tests run with network disabled** (Wave 1 DoD criterion 8). The cassette is replayed; the adapter under test must produce identical normalized output across replays.

### 2.6 Quirks

- CoinGecko `id` is the canonical crypto identifier per v0.4 §30; PRISMATIK maps to `ExternalIdentifier::CoinGeckoId`. NO fallback for identity — divergences must halt.
- Pagination via `page` param; the adapter handles iteration transparently.
- The Demo plan is rate-limited aggressively enough that the desktop MVP MUST surface exact retry times (per Wave 1 DoD criterion 2).

---

## 3. Alpaca (Wave 2 — Equity, Wave 5 — Execution)

### 3.1 Two Surfaces

- **Market Data API** (`data.alpaca.markets`): bars, quotes, trades. Subscription tiers.
- **Trading API** (`api.alpaca.markets` paper / live): orders, positions, account.

### 3.2 Endpoint Catalog (Market Data)

| Endpoint | Path | Auth | Capability |
|---|---|---|---|
| Bars | `GET /v2/stocks/{symbol}/bars` | API key | `Bars` |
| Quotes | `GET /v2/stocks/{symbol}/quotes/latest` | API key | `Quotes` |
| Trades | `GET /v2/stocks/{symbol}/trades` | API key | `Trades` |
| Snapshots | `GET /v2/stocks/snapshots` | API key | `Snapshots` |

### 3.3 Endpoint Catalog (Trading)

| Endpoint | Path | Auth | Capability |
|---|---|---|---|
| Account | `GET /v2/account` | API key (paper/live) | `AccountInfo` |
| Positions | `GET /v2/positions` | API key | `Positions` |
| Submit order | `POST /v2/orders` | API key (paper/live) | `OrderSubmit` |
| Cancel order | `DELETE /v2/orders/{id}` | API key | `OrderCancel` |

### 3.4 Auth

- **Market Data:** `APCA-API-KEY-ID` and `APCA-API-SECRET-KEY` headers.
- **Trading Paper:** `https://paper-api.alpaca.markets`.
- **Trading Live:** `https://api.alpaca.markets`. Requires the live gate: `PRISMATIK_LIVE_TRADING=I_ACCEPT_FULL_RESPONSIBILITY_FOR_LIVE_TRADING` env + per-session `live_confirm`.

**MCP V2 OAuth:** Alpaca's official MCP server supports OAuth (best OAuth-first path for execution). The Wave 5 Alpaca adapter evaluates both API-key and OAuth paths.

### 3.5 Rate Limits

- 200 requests/min per endpoint (varies by subscription).
- Returns 429 with `Retry-After`. Back off exponentially on consecutive 429s.

### 3.6 Error Mapping

| Alpaca response | Mapped code | Notes |
|---|---|---|
| 401 Invalid API key | `Auth` | disable account |
| 403 Forbidden | classify `MARKET_CLOSED` first (response body contains `market is closed`), else `Exchange` | **Per v1.2 §3.8: classify `MARKET_CLOSED` BEFORE `Auth`** |
| 422 Unprocessable | `Exchange` | order rejection; surface reason |
| 429 | `Network` | back off |
| 502/503/504 | `Network` | back off |

### 3.7 Submission Semantics (Wave 5)

- `submit` returns `SubmissionResult`. On 200/201 → `Accepted`. On 4xx with explicit rejection → `Rejected`. On timeout or 5xx after the request left the process → `Unknown`.
- **The `Unknown` path MUST NOT retry blindly** (v1.0 §19.2). The instrument is quarantined; `BrokerGateway::reconcile` is called; the delta against local belief determines the resolution.

### 3.8 Idempotency

`client_order_id` is Alpaca's idempotency key. PRISMATIK generates it as a UUIDv7 (time-ordered) and submits it. Submitting the same `client_order_id` twice produces exactly one order (Wave 5 DoD criterion 4).

---

## 4. SEC EDGAR (Wave 2)

### 4.1 Endpoint Catalog

| Endpoint | Path | Auth | Capability |
|---|---|---|---|
| Submissions | `GET /submissions/CIK{cik}.json` | None (declared user agent) | `FilingIndex` |
| Company facts | `GET /api/xbrl/companyfacts/CIK{cik}.json` | None | `CompanyFacts` |
| Filing index | `GET /cgi-bin/browse-edgar` | None | `FilingSearch` |
| Full text | `GET /cgi-bin/browse-edgar?action=getcompany&type={form}` | None | `FormSearch` |
| Filing document | `GET /{accession-no}/{cik}-{form}.htm` | None | `FilingDocument` |

### 4.2 Auth and Rate Limits

- **No API key.** Declared `User-Agent` header with contact email is required per SEC fair-access policy: `"Mythos Systems PRISMATIK contact:aaron@mythos.systems"`.
- **10 requests/sec maximum** (SEC fair-access). Hard-coded in the adapter; enforced by `BudgetGovernor`.

### 4.3 Quirks

- 13F holdings observable_at = filing_date (~45 days after period_end). The `normalized_institutional_holding` schema in `spec/DATA_SCHEMAS.md` §2.3 makes this explicit.
- XBRL parsing is complex; use `xellogto/us-gaap-xbrl-parser` (or equivalent) and conform to canonical concepts.
- `cik` is the SEC's canonical identifier; map to `ExternalIdentifier::SecCik`.

### 4.4 Error Mapping

| SEC response | Mapped code | Notes |
|---|---|---|
| 200 OK | — | success |
| 403/429 (fair access violation) | `Network` | back off; this is rare at 10 req/sec |
| 404 | n/a | record empty; this is a real "no such filing" not an error |

---

## 5. Unusual Whales (Wave 3 — Options)

### 5.1 Endpoint Catalog (selection — full API at api.unusualwhales.com/docs)

| Endpoint | Path | Auth | Capability |
|---|---|---|---|
| Options flow | `GET /options/option-trades/{ticker}` | Bearer token | `OptionsFlow` |
| Dark pool | `GET /dark-pool/{ticker}` | Bearer token | `DarkPool` |
| Congressional trades | `GET /government/congress-trades` | Bearer token | `CongressionalTrades` |
| Form 4 | `GET /government/form-4/{ticker}` | Bearer token | `InsiderTransactions` |
| 13F | `GET /government/thirteen-f/{ticker}` | Bearer token | `InstitutionalHoldings` |
| Greek exposure | `GET /options/gex/{ticker}` | Bearer token | `GreekExposure` |

### 5.2 Auth and Rate Limits

- **Bearer token** in `Authorization` header. Token from $50/mo subscription.
- Rate limits per subscription tier; documented in api.unusualwhales.com/docs.

### 5.3 Quirks

- Flow data is real-time and noisy; classification with explicit confidence is mandatory (Wave 3 DoD criterion 8). Unclassified prints are used freely.
- Trade-quality score formula is disclosed and versioned alongside every score (Wave 3 DoD criterion 10).

### 5.4 Error Mapping

Standard set: 401 → `Auth`, 429 → `Network`, 5xx → `Network`. Empty 200 → `FailoverTrigger::Empty` (no fallback for flow data; surface empty result honestly).

---

## 6. FRED (Wave 2 — Macro)

### 6.1 Endpoint Catalog

| Endpoint | Path | Auth | Capability |
|---|---|---|---|
| Series | `GET /series?series_id={id}` | API key | `SeriesMeta` |
| Observations | `GET /series/observations?series_id={id}` | API key | `MacroSeries` |
| Release calendar | `GET /releases/dates` | API key | `ReleaseCalendar` |

### 6.2 Auth and Rate Limits

- API key as query param `api_key`.
- 120 requests/min per key.
- **Vintage handling:** FRED publishes data revisions. The adapter records `real_time_start`/`real_time_end` per observation, enabling point-in-time-correct macro joins.

### 6.3 Quirks

- Vintage data is critical for backtests. A 2018 backtest of "GDP growth" MUST use the GDP figure known as of 2018, not the current revised figure. The `observation_delay` in the `FeatureView` captures the publication lag.

---

## 7. CFTC (Wave 2 — CoT)

### 7.1 Endpoint Catalog

- `GET /api/v1/commitments?market_code={code}` — Commitments of Traders
- `GET /api/v1/commitments/lookup?cat={cat}` — lookup tables

### 7.2 Auth and Rate Limits

- **No API key.** Bulk downloads via the public site.
- **Weekly schedule:** CoT publishes Friday 3:30pm ET for Tuesday's data. The `observation_delay` reflects this 3-day lag.

---

## 8. Polygon.io (Optional — Equity)

### 8.1 Pricing Tiers

- Basic ($0/mo): EOD only
- Starter ($29/mo): 15-min delayed
- Developer ($79/mo): delayed
- Advanced ($199/mo): real-time

### 8.2 Endpoint Catalog

| Endpoint | Path | Auth | Capability |
|---|---|---|---|
| Aggregates (bars) | `GET /v2/aggs/ticker/{ticker}/range/{mult}/{timespan}/{from}/{to}` | API key | `Bars` |
| Quotes | `GET /v3/quotes/{ticker}` | API key | `Quotes` |
| Trades | `GET /v3/trades/{ticker}` | API key | `Trades` |
| Options chain | `GET /v3/snapshot/options/{ticker}` | Advanced | `OptionsChain` |

### 8.3 MCP

Polygon has an official MCP server (35+ tools). Wave 5+ may integrate the MCP path.

---

## 9. Databento (Optional — Institutional-grade)

### 9.1 Why Considered

- **Most Rust-friendly vendor** — official Python, C++, **Rust** client libraries.
- Pay-as-you-go usage-based pricing.
- 24-hour intraday replay.
- Strong NautilusTrader integration.

### 9.2 Capabilities

- SBE-encoded market data (CME, ICE, etc.) — see `spec/DATA_SCHEMAS.md` for ingestion.
- Implied volatility surfaces, options chains.
- Microstructure data for research.

### 9.3 Wave 6+ Consideration

Institutional data layer for enterprise customers; not desktop-default.

---

## 10. CCXT (Sidecar)

### 10.1 Surface

- **Process:** Python sidecar (`services/sidecars/ccxt-gateway/`).
- **No credentials, no data-plane writes, no audit events.** Output treated as provider data — provenance-stamped, never authoritative.

### 10.2 Communication

- Arrow over local socket (v1.0 §15.2). The trusted core sends a typed request; the sidecar returns Arrow batches.
- The sidecar is untrusted; its output is validated by the core.

### 10.3 Use Case

Crypto exchange breadth (100+ exchanges) when CoinGecko doesn't cover a venue. Identity resolution via CoinGecko canonical (no fallback).

---

## 11. Inference Providers (AI Stack — per v1.2 §4)

### 11.1 OpenRouter

- **Auth:** OAuth PKCE → per-user API key.
- **Endpoint:** `https://openrouter.ai/api/v1/chat/completions` (OpenAI-compatible).
- **Routing:** per-model via model ID; fanout to all frontier models.
- **Quirk:** some models require provider-specific keys routed through OpenRouter; surface this clearly in the UI.

### 11.2 Google Gemini

- **Auth:** OAuth 2.0 (ADC or AI Studio) — cleanest OAuth story among frontier providers.
- **Endpoint:** `https://generativelanguage.googleapis.com/v1/...` (or Vertex AI).
- **Quirk:** 1M context window; rate limits vary by model.

### 11.3 Anthropic

- **Auth:** API key only (per Feb 2026 ToS, third-party OAuth prohibited).
- **Endpoint:** `https://api.anthropic.com/v1/messages`.
- **Quirk:** 1M context for Opus 5/Sonnet 4.5/Haiku 4.5.

### 11.4 OpenAI

- **Auth:** API key only.
- **Endpoint:** `https://api.openai.com/v1/chat/completions` or `/v1/responses` (stateful).
- **Quirk:** GPT-5.5 Pro is Responses-API-only.

### 11.5 LM Studio (Local)

- **Auth:** None (local).
- **Endpoint:** `http://localhost:1234/v1/chat/completions` (OpenAI-compatible).
- **Egress:** `Loopback` enforced. The local provider MUST be structurally incapable of network egress.
- **model_digest:** REQUIRED (pinned weights; replay detects model swap).
- **MCP client:** LM Studio supports remote MCP servers (opt-in). Connects to Alpha Vantage/Polygon/Alpaca MCP for fully-local-orchestrated workflows.

### 11.6 Other Hosted (xAI Grok, DeepSeek, Cohere, Groq, Together, Fireworks)

- All API-key based. Standardized via the `OpenAiCompatible` provider kind. Adapter-specific quirks documented at intake.

---

## 12. Provider Chain Configuration

```rust
pub struct ProviderChain {
    pub capability: DataCapability,
    pub primary: ProviderId,
    pub fallbacks: Vec<ProviderId>,
    pub agreement_policy: AgreementPolicy,
    pub divergence_action: DivergenceAction,
    pub failover_trigger: FailoverTrigger,
}
```

### 12.1 Default Chains

| Capability | Primary | Fallback | Notes |
|---|---|---|---|
| Crypto bars | Coinbase | CCXT public worker | CoinGecko for OHLCV (daily+) |
| Crypto identity | CoinGecko | — | NO fallback; identity divergences halt |
| Crypto quotes | CoinGecko | Coinbase | |
| Equity bars | Alpaca | Polygon | |
| Equity identity | OpenFIGI | — | NO fallback |
| Equity fundamentals | SEC EDGAR | — | |
| Options flow | Unusual Whales | — | NO fallback; surface empty honestly |
| Macro | FRED | OECD (for cross-country) | |
| CoT | CFTC | — | |
| Insider transactions | SEC EDGAR (Forms 3/4/5) | Unusual Whales | |

### 12.2 Agreement Policy

For critical reads (e.g. bars near order submission), require N-of-M agreement:

```rust
pub enum AgreementPolicy {
    None,
    NofM { n: usize, m: usize },
}
```

If N-of-M fails, `divergence_action` determines what happens: `Halt | PreferPrimary | FlagAndContinue`.

---

## 13. Cassette Library and Contract Test Discipline

### 13.1 Library Location

`tests/cassettes/<provider>/<endpoint>/<test-name>.yaml`. Generated by recording real calls; checked in.

### 13.2 Re-recording Policy

- Cassettes are re-recorded on a quarterly schedule OR when a provider announces breaking changes.
- Re-recording is a deliberate PR with a description of what changed.
- Stale cassettes older than 1 year are flagged.

### 13.3 Property Tests over Contract Tests

Contract tests verify the adapter produces correct output for a fixed input. **Property tests verify invariants across random inputs** (see `spec/TESTING.md`). Property tests are the higher-leverage investment.

---

## 14. Health and Observability

Each provider adapter reports `ProviderHealth`:

```rust
pub struct ProviderHealth {
    pub status: HealthStatus,    // Healthy | Degraded | Offline
    pub last_success: Option<OffsetDateTime>,
    pub last_failure: Option<OffsetDateTime>,
    pub consecutive_failures: u32,
    pub rate_limit_headroom: Option<f64>,    // 0.0 to 1.0
    pub observed_latency_p95: Option<Duration>,
}
```

Surfaced in the admin console and rate-budget UI.

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26 · Version 1.0*
