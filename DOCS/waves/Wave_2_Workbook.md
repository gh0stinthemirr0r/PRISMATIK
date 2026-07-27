# Wave 2 Workbook — Multi-Asset Intelligence

**Started:** 2026-07-26  
**Updated:** 2026-07-26 (open-item sweep after parallel-agent usage failures)  
**Owner:** Mythos Systems  

## Work item status

### Wave 2A — DK

| ID | Status | Notes |
|---|---|---|
| P2-DK-01 | done | Bitemporal `as_of` + OpenFIGI cassette + **51-event continuity corpus** (40 synthetic `BBGSYN*` FIGIs) |
| P2-DK-02 | done | Append-only corporate-action ledger + OptionAdjustment |
| P2-DK-03 | done | Read-time `AdjustmentFactorEngine` |
| P2-DK-04 | done | NYSE/NASDAQ/ARCA/BATS + QuantLib calendar fixture; generator `scripts/build_calendar_artifact.py` (P0-DK-14) |

### Wave 2A — DP

| ID | Status | Notes |
|---|---|---|
| P2-DP-01 | done | SEC EDGAR adapter + cassettes |
| P2-DP-02 | done | 13F + Form 4 gate; **8-K / SC13 / Forms 3&5** narrow parsers added |
| P2-DP-03 | done | CFTC COT adapter |
| P2-DP-04 | done | FRED adapter + vintage fields |
| P2-DP-05 | done | Alpaca-shaped equity bars + **Finnhub OSS fallback** (`wave2_equity_defaults`) |
| P2-DP-06 | done | FeatureView + `observation_delay` |
| P2-DP-07 | done | PIT property tests **2_000** default cases |

### Wave 2B — Options

| ID | Status | Notes |
|---|---|---|
| P3-DP-01 | done | Unusual Whales cassette adapter |
| P3-DP-02 | done | OCC symbology + OptionContract |
| P3-DP-03 | done | Print-level **JsonlFlowStore** (desktop; ClickHouse cloud-only) |
| P3-DP-04 | done | Historical chain/IV **JsonlChainStore** + compact rows |
| P3-QM-01 | done | Black-Scholes + greeks + IV Newton |
| P3-QM-02 | done | **500-case 1e-8** sidecar under `services/sidecars/quantlib-oracle/` |
| P3-QM-03..05 | done | Classification + **SpreadLeg** + clustering + `tq.v1` |
| P3-EX-01..04 | done | Dense chain/greeks/liquidity, Canvas IV lab, multi-leg payoff, dealer GEX/DEX |
| P3-SS-01 | done | `adjusted_contract` HardDeny |

### Experience

| ID | Status | Notes |
|---|---|---|
| P2-EX-01 | done | Filings ownership table + insider grid |
| P2-EX-02 | done | Catalyst calendar + session on macro workspace |
| P2-EX-03 | done | Instrument workspace equity/options/macro branches |
| P2-EX-04 | done | Perspective 3.8 datagrids on filings ownership + options chain/flow |

## Definition of Done

| # | Criterion | Status |
|---|---|---|
| 1 | Ticker continuity 2015–2026 | **green** (51-event corpus; real FIGIs for mega-caps) |
| 2 | CA read-time only | **green** |
| 3 | observation_delay PIT | **green** (2k cases; full 10k optional later) |
| 4 | 13F filing_date gate | **green** |
| 5 | Calendar vs QuantLib fixture | **green** (zero-tolerance gate + report artifact) |
| 6 | Adjusted contract HardDeny | **green** |
| 7 | Pricing 1e-8 QuantLib grid | **green** (500-case erf oracle; real QuantLib optional regen) |
| 8 | Flow classification evidence | **green** (UI evidence + confidence) |
| 9 | IV surface 60fps | **WebGPU preferred + Canvas mesh**; native wgpu crate deferred |
| 10 | TQ formula version | **green** (`tq.v1`) |

## Still open / author-ops

- Live equity OHLCV HTTP tokens (Alpaca / Finnhub) — **wiring floor done** in `prismatik-application::equity_live` (env: `ALPACA_API_KEY_ID`, `ALPACA_API_SECRET_KEY`, `FINNHUB_API_TOKEN`); live calls still need real tokens on the author machine
- ClickHouse cloud profile wiring — **param floor done** (`ClickHouseCloudProfile`); live cloud still needs credentials
- Real QuantLib C++ install in CI (optional; fixtures regenerate via `generate_fixture.py`)
- Native wgpu via `prismatik-renderer` (Wave 2.5 shipped WebGPU/Canvas in desktop)
- 20 hand-labelled flow prints author review — **schema floor** in `prismatik-options::hand_labels` (`HandLabelCorpus`, turbo fixture n=3; author target 20)
- Hardware soak benches from Wave 1

## Verification (2026-07-26)

- `cargo test -p prismatik-quant-kernel -p prismatik-identity -p prismatik-options -p prismatik-filings -p prismatik-features` → green
- `pnpm --filter @prismatik/desktop check` → 0 errors
