# SBOM output directory (P0-SS-06 floor)

Generated CycloneDX SBOMs land here when `scripts/generate-sbom.sh` (or
`.ps1`) can run `cargo cyclonedx`.

SBOMs are **compliance / inventory** artifacts, not a supply-chain defense.
Signing and provenance verification live under `prismatik-security::updater`
and author release-ops (cosign / SLSA). See `DOCS/waves/P0_SS_06_Release_Signing.md`.

## Generate

```bash
bash scripts/generate-sbom.sh
# or
pwsh scripts/generate-sbom.ps1
```

Install the generator if missing:

```bash
cargo install cargo-cyclonedx
```

Generated `bom.json` / `*.cdx.json` files are local release artifacts and are
not required in the default CI path.
