# Audit append p99 report (P0-DK-08)

**Budget:** &lt; **1 ms** p99 per append  
**Harness:** `crates/prismatik-audit/benches/audit_append.rs`  
**Threshold test:** `ledger::tests::append_p99_smoke_under_1ms`  
**Wave doc:** `DOCS/waves/P0_DK_08_Audit_Append_Benchmark.md`

## Host (this soak)

| Field | Value |
|---|---|
| Date (UTC) | 2026-07-27 |
| OS | Windows NT 10.0.26200 |
| CPU | Intel Core i9-14900HX |
| rustc | 1.88.0 (6b00bc388 2025-06-23) |
| Profile | `cargo bench` (`[profile.bench]` → release + debuginfo) |
| Ledger | `InMemoryAuditLedger` |

## Criterion results

Command:

```bash
cargo bench -p prismatik-audit --bench audit_append
```

| Benchmark | Criterion point estimate | vs 1ms budget |
|---|---|---|
| `audit_append/in_memory/single_append` | **~1.43 µs** (slope); mean ~1.65 µs; median ~1.48 µs | **PASS** (~700× headroom) |
| `audit_append/in_memory/append_at_1k` | **~286 µs** (mean); median ~270 µs | **PASS** (~3.5× headroom) |

Notes:

- Criterion reports mean/median/slope with 95% CI, not a strict empirical p99.
  Both means and upper CI bounds are far below 1ms on this host.
- `append_at_1k` measures one append after a 1024-leaf warm tree (full Merkle
  rebuild is currently O(n); still under budget at this size).
- HTML plots (when plotters available): `target/criterion/audit_append/` (gitignored).

### Raw criterion confidence intervals (ns)

**single_append**

- mean: 1646 ns [1558, 1747]
- median: 1484 ns [1402, 1546]
- slope: 1430 ns [1384, 1478]

**append_at_1k**

- mean: 286485 ns [277541, 295841]
- median: 269500 ns [260650, 278700]

## Smoke threshold test

```bash
cargo test -p prismatik-audit append_p99_smoke_under_1ms -- --nocapture
```

- Sample size: 256 timed appends after 32 warmups
- Gate: empirical p99 ≤ 1_000_000 ns
- Result on this host (debug test profile): **PASS** — p50≈131 µs, p99≈352 µs
  (release criterion numbers above are the soak artifact; debug smoke is the CI gate)

CI VMs may be noisier than this laptop soak. The smoke test fails loudly if
p99 exceeds 1ms; treat flakes as a signal to re-run on a quiet host rather than
silently raising the budget.

## Verdict

**P0-DK-08 harness + budget evidence: GREEN on authoring host.**  
In-memory append meets the 1ms p99 budget with substantial headroom for the
empty-tree interactive path; steady-state at 1k leaves remains under budget.
