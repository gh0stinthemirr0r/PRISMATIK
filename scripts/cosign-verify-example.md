# Cosign / SLSA verify interface (P0-SS-06 floor)

This document describes the **verification interface** the updater expects.
It does **not** ship production cosign keys, Fulcio identities, or SLSA builder
attestations. Those remain **author / release-ops**.

## Code floor (default CI)

Offline refuse-invalid lives in `prismatik-security::updater`:

| Trait / type | Role |
|---|---|
| `CosignVerifier` | Signature check over artifact digest |
| `SlsaProvenanceVerifier` | Builder identity + source repo (SLSA-shaped claims) |
| `StubCosignVerifier` | Ed25519 dual-signature floor — **not** Sigstore/Rekor |
| `StubSlsaProvenanceVerifier` | Allowlisted `builder_id` / `source_repo` strings — **not** in-toto JSON |
| `UpdaterProvenanceVerifier` | Compose digest + signature + provenance; refuse on any failure |
| `ProvenanceAttestation` | Envelope: `artifact_digest`, `signature`, `builder_id`, `source_repo` |

Negative tests (no cosign binary required):

```bash
cargo test -p prismatik-security updater -- --nocapture
```

Covered cases: tampered bytes, missing attestation, wrong builder identity.

## Production mapping (author release-ops)

When a real release pipeline exists, map Sigstore / SLSA material into the
same decision surface:

1. **Digest** — hash the artifact; must equal attested subject digest.
2. **Cosign** — verify blob / container signature (keyless OIDC → Fulcio, Rekor inclusion). Map failure → `RejectReason::SignatureInvalid`.
3. **SLSA** — verify provenance predicate (builder ID, source repo, materials). Map failure → `RejectReason::ProvenanceInvalid`.
4. **Missing any required attestation** → `RejectReason::MissingAttestation`.

A future `CosignVerifier` / `SlsaProvenanceVerifier` impl may shell out to
`cosign` or use a Rust Sigstore client. Until then, stubs keep CI hermetic.

## Example: verify with cosign (when installed)

Install: <https://docs.sigstore.dev/cosign/system_config/installation/>

**These examples assume author-ops already published signatures.** There are
**no** repository-committed production signing keys.

```bash
# Blob signature (release artifact)
cosign verify-blob \
  --certificate-identity "https://github.com/OWNER/REPO/.github/workflows/release.yml@refs/tags/vX.Y.Z" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
  --bundle artifact.cosign.bundle \
  artifact.bin

# Attestation / provenance (SLSA-shaped)
cosign verify-attestation \
  --type slsaprovenance \
  --certificate-identity "https://github.com/OWNER/REPO/.github/workflows/release.yml@refs/tags/vX.Y.Z" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
  artifact.bin
```

Replace `OWNER/REPO` and the workflow identity with the real release workflow
once author-ops wires `release.yml` (see `DOCS/spec/CI_WORKFLOWS.md` §7).

## SBOM

```bash
bash scripts/generate-sbom.sh
```

SBOM generation is inventory-only; it does not replace cosign or updater
verification. See `DOCS/waves/P0_SS_06_Release_Signing.md`.
