# Turbo Mode Gate Policy — Author-Ops Waivers

**Date:** 2026-07-26  
**Authority:** Author directive — continue without breakpoints until structural wave floors land.  
**Scope:** Unblocks codeable Wave 3+ work. Does **not** claim production release readiness.

## Waivers

| ID | Item | Waiver | Residual risk | Resolve by |
|---|---|---|---|---|
| P0-DK-10 | Machine B cross-verify | **WAIVED for Wave 3 OPEN** | Dual-sig format + negatives + Machine A template remain; second-host verify unproven | Wave 5 release candidate (fill Machine B before any signed public binary) |
| P0-SS-06 | Prod cosign / SLSA L3 | **WAIVED beyond floor** | Scripts + stub verifiers only | First public release (Wave 1 ship / Wave 5 live) |
| W2 tokens | Live Alpaca/Finnhub keys | **WAIVED** | `equity_live` refuses without env; cassette path green | Author machine when keys available |
| W2 flow labels | 20 hand-labelled prints | **WAIVED** | Classifier + UI evidence path exists | Wave 4 journal loop |
| W1 soak | Hardware soak benches | **WAIVED** | Criterion benches exist; soak logs optional | Pre-ship OD |
| W2 ClickHouse cloud | Cloud profile | **DEFERRED floor** | Local/sqlite profiles; cloud types stubbed in storage | Wave 6 DP |
| W2 native wgpu | `prismatik-renderer` | **DEFERRED** | Desktop WebGPU/Canvas shipped | Wave 3B EX path clouds |

## Turbo completion definition

For Waves 3–7 under this policy, **“wave floor complete”** means:

1. Every work-item ID has types + unit tests (or explicit stub returning typed error) in the owning crate.
2. Normative hard-deny / negative tests called out in that wave’s DoD are present where enforceable offline.
3. Workbook scorecard marks items **floor** / **done** / **author-ops** honestly.
4. No claim of production live trading, marketplace, or K8s deploy without the waived author/release ops above.

## Wave 3 OPEN

Under this policy, `P0-REMAINDER` hard gate is **satisfied for OPEN** with P0-DK-10 Machine B waived as above. Wave 3A+ implementation proceeds.
