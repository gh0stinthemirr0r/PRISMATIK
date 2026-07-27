# Wave 0 Gate — Turbo Floor vs Author-Ops

**Policy:** [`TURBO_GATE_POLICY.md`](../waves/TURBO_GATE_POLICY.md) · **Workbook:** [`Wave_0_Foundation.md`](../waves/Wave_0_Foundation.md)  
**Verdict:** **P0-FLOOR met** (Option B). Remainder satisfied for Wave 3 OPEN with listed residuals.

## Exit criteria

| # | Criterion | Class | Status |
|---|---|---|---|
| 1 | Determinism allowlist / no forbidden clocks-rng | turbo-floor | **verified 2026-07-26** — see log below |
| 2 | Grep gate negative test | turbo-floor | **verified 2026-07-26** — see log below |
| 3 | Entropy split order-independence | turbo-floor | met |
| 10 | Bindings drift gate | turbo-floor | met |
| 13 | Tauri capability policy + ≥2.12 | turbo-floor | met |
| 4 | DST replay 64 seeds | turbo-floor | met (remainder) |
| 6–8 | Dual-sig negatives / audit tamper / append p99 | turbo-floor | met (remainder) |
| 9 | Updater refuses invalid provenance | turbo-floor | met (remainder) |
| 11–12 | Registry coverage / QuantLib calendar | turbo-floor | met (remainder) |
| 14 | ADRs 0020–0028 | turbo-floor | met (partial / ongoing) |
| 5 | Machine B cross-verify (P0-DK-10) | author-ops residual | **waived for OPEN** — resolve before public signed binary |
| — | Prod cosign / SLSA L3 (P0-SS-06 beyond floor) | author-ops residual | open |

## Verification log — 2026-07-26 (criteria 1 & 2 made demonstrably true)

These two criteria were previously recorded as "met" without a passing gate
behind them. Both are now verified by named artifacts:

**Criterion 1 — determinism gate.** `cargo clippy --workspace --all-targets
-- -D warnings` is green. Prior to this pass it was red: ~30 disallowed
`OffsetDateTime::now_utc` / `Uuid::new_v4` / `HashMap` violations across
storage, features, options, portfolio, tsfm, execution, cli, plus a broken
`clippy.toml` path (`rand::thread_rng` does not exist in rand 0.10). All fixed
at the source (injected `Clock` into `SqliteBackend`; `DetMap` everywhere;
`SystemClock` in the CLI shell; `clippy.toml` paths corrected). The
`scripts/determinism_grep.py` guard (which CI runs) passes with 1 justified
allowlist entry (`crates/prismatik-determinism/ALLOWLIST.txt`, cap 10).

**Criterion 2 — grep-gate negative test.** `scripts/determinism_grep.py` is
now comment-aware, allowlisted, and matches the full forbidden set per
`DOCS/spec/CI_WORKFLOWS.md`. The negative test
`crates/prismatik-determinism/tests/grep_gate.rs` proves the gate fails on a
deliberate `HashMap` violation and passes when clean. CI runs both the gate
and the negative test (`.github/workflows/ci.yml` → `determinism-grep` job).

**Cross-check of related negatives (criterion 6–7):**
`dual_required_rejects_single_valid_ed25519` (signature.rs) proves a single
valid Ed25519 half fails under `DualRequired` (DoD #6).
`tamper_is_detected` (ledger.rs) proves `verify_all` rejects a corrupted entry
(DoD #7). Both run as part of `cargo test --workspace` (501 passed, 0 failed).

