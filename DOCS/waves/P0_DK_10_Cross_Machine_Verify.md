# P0-DK-10 — Cross-machine dual-signature verify

**Purpose:** record that a signed reproducibility manifest verifies on a second
machine with only the published schema + `prismatik-cli verify` (no app install).

**Status:** **WAIVED for turbo OPEN** (Machine B unproven). Dual-sig format +
negatives + Machine A template remain. **Required before any public signed
binary** — see `TURBO_GATE_POLICY.md`. Do **not** claim Machine B done until
the checklist below is filled and signed off.

**Related:** `P0_DK_10_PQC_Exception.md`, Wave 3 residual section.

---

## Bundle under test

| Field | Value |
|---|---|
| Manifest path | `crates/prismatik-manifest/golden/manifest_data_ingest_v1.json` (or attach path) |
| Schema version | `1.0.0` |
| Signature scheme | `ed25519_only` (Wave 0 PQC exception; `ml_dsa: null`) |
| Policy used | `Ed25519OnlyException` / `--allow-pq-pending` |
| Git commit / tag | _fill_ |
| Bundle hash (BLAKE3 of manifest file) | _fill_ |

### Dual-sig format checklist (both hosts)

- [ ] Manifest carries dual-signature object with `ed25519` half populated
- [ ] `ml_dsa` field present in schema / envelope (`null` / `None` OK under PQC exception)
- [ ] Policy documents `Ed25519OnlyException` vs future `DualRequired`
- [ ] Schema version matches published verifier expectations
- [ ] No reliance on app install or shared local state for verify

---

## Machine A (authoring / sign host)

| Field | Value |
|---|---|
| Host identity (hostname / inventory id) | _fill_ |
| Host OS | _e.g. Windows 11 10.0.26200_ |
| Arch | _e.g. x86_64_ |
| Rustc | _`rustc -V`_ |
| CLI / binary provenance | _source build path or release digest_ |
| Command | `cargo run -p prismatik-cli -- verify <manifest> --allow-pq-pending` |
| Result | _PASS / FAIL + report overall_ |
| Operator | _name_ |
| Date (UTC) | _YYYY-MM-DD_ |

---

## Machine B (independent verify host)

**Turbo OPEN:** Machine B remains **WAIVED** — leave fields pending until a
second host actually runs verify. Filling this section is a hard residual
before public signed binaries.

| Field | Value |
|---|---|
| Host identity (hostname / inventory id) | _pending — distinct from Machine A_ |
| Host OS | _pending_ |
| Arch | _pending_ |
| Rustc / binary provenance | _pending (fresh source build or released verifier digest)_ |
| Network | _offline / unused_ |
| Shared state with Machine A | _none (no copied keystore, no app data dir)_ |
| Command | _same verify invocation as Machine A_ |
| Manifest hash vs Machine A | _must match_ |
| Result | _pending — do not mark PASS under waiver_ |
| Operator | _pending_ |
| Date (UTC) | _pending_ |

### Machine B checklist

- [ ] Fresh checkout (or released `prismatik-cli`) with no application crate linked
- [ ] Host identity fields filled and **different** from Machine A
- [ ] Offline verify (network disabled or unused)
- [ ] Same manifest bytes as Machine A (hash match)
- [ ] Dual-sig format checklist above satisfied on this host
- [ ] Report `overall: true` under `Ed25519OnlyException`
- [ ] Negative: flip one `ed25519` signature byte → verify fails
- [ ] Negative: truncate / omit signature object → verify fails
- [ ] Negative: wrong policy (`DualRequired` while `ml_dsa` null) → fails closed
- [ ] Operator + UTC date recorded (sign-off)

**Machine B status:** **WAIVED** (turbo OPEN) — not executed.

---

## Notes

- Full dual (`DualRequired`) cross-machine verify waits on ML-DSA population
  (`DOCS/waves/P0_DK_10_PQC_Exception.md`).
- Attach CLI JSON report snippets below if useful.

```text
(Machine A report)
```

```text
(Machine B report — empty until waiver lifted)
```
