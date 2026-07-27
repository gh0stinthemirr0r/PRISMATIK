# P0-SS-06 — Release signing floor (cosign / SBOM / SLSA)

**Wave:** 0 / P0-REMAINDER  
**Status:** **floor landed** (scripts + docs + refuse-invalid). **Not** full release-ops.  
Prod cosign / **SLSA L3** remain **WAIVED beyond floor** per `TURBO_GATE_POLICY.md`
until first public release.  
**Related:** P0-SS-07 updater verification (`prismatik-security::updater`);
Wave 3 residual section.

This item is split so Wave 3 can move off **"missing"** without pretending
production Sigstore keys or SLSA Level 3 generators exist in-repo.

## Automated (in-repo floor)

| Piece | Path | Notes |
|---|---|---|
| SBOM script (sh) | `scripts/generate-sbom.sh` | Runs `cargo cyclonedx` if installed; else documents install + keeps `artifacts/sbom/` |
| SBOM script (ps1) | `scripts/generate-sbom.ps1` | Same on Windows |
| SBOM output dir | `artifacts/sbom/README.md` | Placeholder path committed; generated BOMs local |
| Cosign / SLSA verify docs | `scripts/cosign-verify-example.md` | Interface map + example CLI; **no** production keys |
| Updater refuse-invalid | `crates/prismatik-security/src/updater/mod.rs` | `StubCosignVerifier` + `StubSlsaProvenanceVerifier`; negative tests |
| CI (blocking) | `.github/workflows/ci.yml` job `updater-provenance` | `cargo test -p prismatik-security updater` — **no cosign binary** |
| CI (optional) | `.github/workflows/ci.yml` job `sbom-generate` | `continue-on-error: true`; does not fail default CI if cyclonedx missing |

### Verify locally

```bash
cargo test -p prismatik-security updater -- --nocapture
bash scripts/generate-sbom.sh   # or: pwsh scripts/generate-sbom.ps1
```

## Author-ops checklist — cosign + SLSA L3 (before public signed release)

Floor above is **not** this checklist. Mark each item only when author-ops
actually lands it. Do **not** claim SLSA L3 or production cosign done under turbo.

### Cosign / Sigstore

- [ ] Production cosign / Sigstore keyless OIDC identity (Fulcio + Rekor)
- [ ] Published signature / bundle per release artifact
- [ ] Certificate identity + OIDC issuer allowlist pinned to live org + workflow
- [ ] Verify path documented against `scripts/cosign-verify-example.md` (live IDs)
- [ ] Real `CosignVerifier` replaces stub; updater still refuse-on-failure
- [ ] Negative: tampered artifact / bad identity rejected in release smoke

### SLSA L3 provenance

- [ ] Provenance generation wired (`release.yml` / `slsa-github-generator` or equiv.)
- [ ] In-toto / SLSA provenance attached to each public artifact
- [ ] Builder ID + source repo claims match allowlist
- [ ] Real `SlsaProvenanceVerifier` replaces stub; refuse-on-failure kept
- [ ] CI or release job verifies provenance (beyond floor `updater-provenance` stubs)
- [ ] Negative: invalid / missing provenance refused (live path, not only stubs)

### SBOM + packaging

- [ ] CycloneDX from `scripts/generate-sbom.{sh,ps1}` attached to GitHub Releases
- [ ] Optional CI `sbom-generate` promoted or mirrored in release workflow
- [ ] Platform code signing (Apple / Windows / Linux) per `DOCS/spec/CI_WORKFLOWS.md` §7

### Script / CI reference map

| Concern | In-repo today | Author-ops target |
|---|---|---|
| SBOM generate | `scripts/generate-sbom.{sh,ps1}`, CI `sbom-generate` | Attach BOM to release |
| Cosign verify examples | `scripts/cosign-verify-example.md` | Live identity + bundles |
| Refuse-invalid unit floor | CI `updater-provenance` | Same traits, real Sigstore/SLSA impls |
| SLSA L3 generate | _(absent)_ | `release.yml` / slsa-github-generator |

**Do not claim production cosign keys or SLSA L3 generators exist in this repository.**

## Doctrine reminder

SBOMs do not prevent supply-chain attacks. Controls that resist attack:

- reviewed lockfile
- `cargo vet` / deny / audit (P0-SS-04 and related)
- artifact signing
- **provenance verification at install time** (updater)

See architecture supply-chain notes and `DOCS/spec/CI_WORKFLOWS.md` §7.3.
