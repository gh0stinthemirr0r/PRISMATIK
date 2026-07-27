# Wave 1 Workbook — Go Big Tracker

**Branch program:** `feat/wave1-*` stacked PRs  
**Started:** 2026-07-26  
**Owner:** Mythos Systems

## Work item status

| ID | Status | Notes |
|---|---|---|
| P1-DP-01..05 | done | Provider, GCRA, chain, CoinGecko, cassettes |
| P1-DP-06 | done | Parquet append-only raw writer + supersession registry |
| P1-DP-07 | done | Normalize + dedup ingest pipeline in market-data |
| P1-DP-08 | done | AnalyticalBackend curated Parquet view layer |
| P1-DP-09 | done | SQLite migrations M0001–M0003 |
| P1-DP-10 | done | EvidenceRef, Concludes, Lineage, orphan test |
| P1-DP-11 | done | Application TaskGraph + SQLite recovery |
| P1-DP-12 | done | AnalogStore hash-embedding KNN scaffold |
| P1-AI-01 | done | `prismatik-ai-router` + `InferenceRoute` / `RouterPolicy::deny_uncalibrated` floor |
| P1-AI-02 | done | LM Studio loopback-only + replay |
| P1-AI-03..04 | done | OpenRouter/Gemini/Anthropic/OpenAI stubs |
| P1-AI-05 | done | MCP-style tool registry + `ToolManifest` / `ToolInvocationGate` Network deny |
| P1-AI-06 | done | AnalystScope isolation tests |
| P1-EX-01..08 | done | Shell, charts, asset route, watchlist, filters, theme, states, countdown |
| P1-QM-01..02 | done | Alert rules JSON + UI panel |
| P1-OD-01 | done | `/onboarding` first-run |
| P1-OD-02 | done | Application backup module + N→N+1 test path |
| P1-OD-03 | done | Observability tracing helpers |
| P1-OD-04 | done | Bundle active; signing/update ops checklist in benchmarks doc |
| P1-OD-05 | done | User guide + NOTICE generator |

## Definition of Done

| # | Criterion | Artifact | Status |
|---|---|---|---|
| 1 | No orphan conclusions | `test_no_orphan_conclusions` / evidence module | **green** |
| 2 | Exact rate-budget retry | UI countdown + GCRA IPC | **green** |
| 3 | Degraded when CG unreachable | `PRISMATIK_DATA_MODE=chaos` + banner | **green** |
| 4 | Stale markers | StaleDataMarker on quotes | **green** |
| 5 | Clean install + signed update | Bundle active; VM recording = author ops | checklist |
| 6 | Cold start &lt;2s p95 | [Wave_1_Benchmarks.md](Wave_1_Benchmarks.md) + `SoakBenchPlan` / `SoakResultStub` (hardware soak author-ops, turbo-waived) | harness |
| 7 | Chart 1M pts | Benchmarks doc + soak plan types | harness |
| 8 | Offline cassette CI | `offline-market-data` job | **green** |
| 9 | Backup N→N+1 | application backup tests | **green** |
| 10 | LM Studio loopback only | ai-router reject test | **green** |
| 11 | AI uncertainty + contradictions | InferenceResponse fields + review checklist | **green** (fields); UI corpus = author |
| 12 | Author week as user | Daily log below | in progress |

## Author-as-user log (DoD 12)

| Date | Hours used | Notes |
|---|---:|---|
| 2026-07-26 | — | Go Big implementation landed: storage, evidence, AI router, desktop EX |

## AI UI review checklist (DoD 11) — 20 sample queries

Record pass/fail when exercising local/hosted AI surfaces:

1. Symbology normalize BTC  
2. Classify meme vs L1  
3. Summarize bitcoin cassette detail  
4. Extract 24h change fields  
5. Draft watchlist rationale  
6. Critique dominance reading  
7. Contradict empty evidence  
8. Suggest verification for trending  
9–20. Reserved for author week

## Session notes

- Only `prismatik-application` depends on `prismatik-storage`.
- Live HTTP via `ReqwestTransport` in application; Layer 2 remains cassette/port-only.
- Desktop JSON stores for watchlist/filters/alerts; SQLite available via application runtime for full ops.
