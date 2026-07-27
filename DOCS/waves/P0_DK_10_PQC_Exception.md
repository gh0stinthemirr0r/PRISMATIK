# P0-DK-10 — Wave 0 PQC exception (ML-DSA deferred)

**Status:** active exception  
**Opened:** 2026-07-26  
**Wave:** 0 / P0-REMAINDER  
**Owner:** Determinism Kernel (DK)

## Exception

ML-DSA-65 (FIPS 204) cryptographic sign/verify is **not** wired in Rust yet.
The dual-signature **format** ships with both halves:

- `ed25519` — implemented (sign + verify)
- `ml_dsa` — field present in schema / `DualSignature`; may be `null` / `None`

Retrofit when a mature ML-DSA crate is selected is **field population**, not a
schema migration.

## Policy

Verification is explicit via `DualSignaturePolicy`:

| Policy | Behavior |
|---|---|
| `Ed25519OnlyException` | Valid Ed25519 alone passes; empty `ml_dsa` allowed. Tampered Ed25519 fails. |
| `DualRequired` | Both halves required. A **single valid Ed25519** alone **fails** (`MissingMlDsa`). |

Committed negatives live in `crates/prismatik-determinism/src/signature.rs` tests:

- `dual_required_rejects_single_valid_ed25519`
- `ed25519_only_exception_rejects_tampered_ed25519`

## Close criteria

1. ML-DSA-65 sign/verify wired behind the existing `ml_dsa` field.
2. Default production policy flips to `DualRequired`.
3. Golden manifests re-signed with both halves.
4. This exception marked **closed** with the PR that flips the default.

## Cross-machine verify

Template: `DOCS/waves/P0_DK_10_Cross_Machine_Verify.md` (author fills machine B).
