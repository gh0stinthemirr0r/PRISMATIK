# Wave 2 Gate — Turbo Floor vs Author-Ops

**Policy:** [`TURBO_GATE_POLICY.md`](../waves/TURBO_GATE_POLICY.md) · **Workbook:** [`Wave_2_Workbook.md`](../waves/Wave_2_Workbook.md)  
**Verdict:** **Exit gate green (turbo).** Live tokens / cloud / labels waived per policy.

## Exit criteria (DoD)

| # | Criterion | Class | Status |
|---|---|---|---|
| 1 | Ticker continuity 2015–2026 | turbo-floor | met |
| 2 | CA read-time only | turbo-floor | met |
| 3 | observation_delay PIT | turbo-floor | met |
| 4 | 13F filing_date gate | turbo-floor | met |
| 5 | Calendar vs QuantLib | turbo-floor | met |
| 6 | Adjusted contract HardDeny | turbo-floor | met |
| 7 | Pricing 1e-8 grid | turbo-floor | met |
| 8 | Flow classification evidence | turbo-floor | met |
| 9 | IV surface path | turbo-floor | met (WebGPU/Canvas); native wgpu deferred |
| 10 | TQ formula version | turbo-floor | met |
| — | Live Alpaca/Finnhub tokens | author-ops residual | waived (wiring floor done) |
| — | ClickHouse cloud credentials | author-ops residual | deferred floor / cloud types |
| — | 20 hand-labelled flow prints | author-ops residual | waived → Wave 4 journal |
| — | Hardware soak / native wgpu | author-ops residual | waived / deferred |
