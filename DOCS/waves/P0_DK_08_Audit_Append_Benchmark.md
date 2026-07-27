# P0-DK-08 — Audit append 1ms p99 benchmark

**Wave:** 0 / P0-REMAINDER  
**Budget:** audit ledger append **&lt; 1ms p99**  
**Harness:** `crates/prismatik-audit/benches/audit_append.rs`  
**Threshold test:** `append_p99_smoke_under_1ms` in `prismatik-audit`  
**Author soak log (optional):** `crates/prismatik-audit/benches/AUDIT_APPEND_P99.md` — create after a quiet-host criterion run; not required for the harness floor.

## How to run

```bash
cargo test -p prismatik-audit append_p99_smoke_under_1ms -- --nocapture
cargo bench -p prismatik-audit --bench audit_append
```

Criterion writes HTML under `target/criterion/audit_append/` (gitignored).
Summarize numbers into `AUDIT_APPEND_P99.md` after each soak on representative hardware (optional evidence).

## CI posture

- The **smoke threshold test** fails the build if empirical p99 of 256 samples
  exceeds 1ms — loud regression signal.
- Full criterion soak is **author/hardware**; CI VMs are not proof of p99.
  If smoke flakes on a noisy runner, capture the measured p99 in the report and
  re-run on a quiet host before waiving.
