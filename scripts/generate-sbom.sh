#!/usr/bin/env sh
# P0-SS-06 floor: generate a CycloneDX SBOM into artifacts/sbom/ when
# `cargo cyclonedx` is available; otherwise ensure the output directory + README
# exist and print install instructions. Does not claim production release signing.
set -eu
ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

OUT_DIR="artifacts/sbom"
mkdir -p "$OUT_DIR"

README="$OUT_DIR/README.md"
if [ ! -f "$README" ]; then
  cat >"$README" <<'EOF'
# SBOM output directory (P0-SS-06 floor)

Generated CycloneDX SBOMs land here when `scripts/generate-sbom.sh` (or
`.ps1`) can run `cargo cyclonedx`.

SBOMs are **compliance / inventory** artifacts, not a supply-chain defense.
Signing and provenance verification live under `prismatik-security::updater`
and author release-ops (cosign / SLSA). See `DOCS/waves/P0_SS_06_Release_Signing.md`.
EOF
fi

if command -v cargo-cyclonedx >/dev/null 2>&1 \
  || cargo cyclonedx --help >/dev/null 2>&1; then
  echo "P0-SS-06: running cargo cyclonedx → $OUT_DIR"
  # Prefer workspace JSON; cyclonedx-cargo writes beside the manifest by default.
  cargo cyclonedx --manifest-path Cargo.toml --format json --output-cdx \
    || cargo cyclonedx --manifest-path Cargo.toml --format json
  # cargo-cyclonedx writes beside each manifest, so a workspace run scatters
  # one *.cdx.json per member crate. Sweep the root AND every member directory,
  # or the generated files are left stranded in the tree (and get committed).
  for f in bom.json *.cdx.json crates/*/*.cdx.json apps/*/src-tauri/*.cdx.json; do
    if [ -f "$f" ]; then
      mv -f "$f" "$OUT_DIR/$(basename "$f")"
      echo "moved $f → $OUT_DIR/"
    fi
  done
  # Some versions write under target/
  if [ -f target/cyclonedx/bom.json ]; then
    cp -f target/cyclonedx/bom.json "$OUT_DIR/bom.json"
    echo "copied target/cyclonedx/bom.json → $OUT_DIR/bom.json"
  fi
  echo "P0-SS-06: SBOM generation finished (see $OUT_DIR/)."
  exit 0
fi

cat <<'EOF'
P0-SS-06: cargo cyclonedx not installed — placeholder path ready.

Install (author / release host):
  cargo install cargo-cyclonedx

Then re-run:
  bash scripts/generate-sbom.sh

Output directory (committed README, generated files local):
  artifacts/sbom/

No production cosign keys or SLSA attestations are produced by this script.
See scripts/cosign-verify-example.md and DOCS/waves/P0_SS_06_Release_Signing.md.
EOF
exit 0
