# Wave 1 Benchmark Notes (DoD 6–7)

## Cold start (DoD 6)

Target: interactive workspace &lt; 2.0s p95 on mid-tier hardware.

Harness (manual for Wave 1):
1. Build release: `cargo build --release` in `apps/desktop/src-tauri`.
2. Measure process start → first `get_crypto_markets` IPC return with PowerShell `Measure-Command` or Windows Performance Recorder.
3. Record result in `DOCS/waves/Wave_1_Workbook.md`.

Status: harness documented; author must record hardware-specific p95.

## Chart 1M points (DoD 7)

Target: pan/zoom p99 frame ≤ 16.7ms with 1M candlesticks.

Approach:
- Generate synthetic `CandlestickPoint[]` of length 1_000_000 in a bench page (dev-only).
- Profile with Chromium Performance panel inside the Tauri WebView.
- If p99 exceeds budget, downsample to LTTB for overview and keep full res in visible range (follow-up).

Status: methodology documented; full 1M soak deferred to author hardware pass.

## Installer / auto-update (DoD 5)

- `tauri.conf.json` `bundle.active` enabled for Wave 1 packaging path.
- Code-signing: provide `TAURI_SIGNING_PRIVATE_KEY` in CI secrets; document VM install recording in workbook.
- Updater plugin wiring is scaffolded; signed increment verification is an ops checklist item.
