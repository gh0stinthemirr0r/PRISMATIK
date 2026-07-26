#!/usr/bin/env bash
# Generate all crate stubs. Each crate gets Cargo.toml + src/lib.rs.
# Layer deps per DOCS/spec/CRATE_ARCHITECTURE.md.
set -euo pipefail

cd /d/DevOps/PRISMATIK

# crate_name|layer|deps_csv
CRATES=(
    # Layer 0 — no workspace deps
    "prismatik-determinism|0|"
    "prismatik-identity|0|prismatik-determinism"
    # Layer 1
    "prismatik-calendar|1|prismatik-determinism"
    "prismatik-audit|1|prismatik-determinism"
    "prismatik-manifest|1|prismatik-determinism,prismatik-audit"
    "prismatik-storage|1|prismatik-determinism,prismatik-identity"
    # Layer 2 — domain
    "prismatik-domain|2|prismatik-determinism,prismatik-identity"
    "prismatik-market-data|2|prismatik-domain"
    "prismatik-features|2|prismatik-domain,prismatik-market-data"
    "prismatik-analog-store|2|prismatik-domain,prismatik-features"
    "prismatik-indicator-core|2|prismatik-domain"
    "prismatik-quant-kernel|2|prismatik-domain"
    "prismatik-strategy|2|prismatik-domain,prismatik-determinism,prismatik-indicator-core,prismatik-features"
    "prismatik-backtest|2|prismatik-strategy,prismatik-calendar,prismatik-features"
    "prismatik-simulation|2|prismatik-backtest,prismatik-determinism"
    "prismatik-tsfm|2|prismatik-determinism,prismatik-manifest"
    "prismatik-calibration|2|prismatik-tsfm,prismatik-determinism"
    "prismatik-crypto|2|prismatik-domain,prismatik-market-data"
    "prismatik-options|2|prismatik-domain,prismatik-market-data"
    "prismatik-filings|2|prismatik-domain,prismatik-market-data"
    "prismatik-cot|2|prismatik-domain,prismatik-market-data"
    "prismatik-events|2|prismatik-domain,prismatik-market-data"
    "prismatik-risk|2|prismatik-domain,prismatik-portfolio"
    "prismatik-portfolio|2|prismatik-domain,prismatik-identity"
    "prismatik-execution|2|prismatik-domain,prismatik-identity,prismatik-risk"
    "prismatik-journal|2|prismatik-domain,prismatik-market-data"
    # Layer 3 — extension + AI
    "prismatik-plugin-host|3|prismatik-determinism,prismatik-audit"
    "prismatik-ai-tools|3|prismatik-domain,prismatik-market-data,prismatik-portfolio"
    "prismatik-security|3|prismatik-determinism"
    "prismatik-observability|3|prismatik-determinism"
    "prismatik-renderer|3|prismatik-domain"
    "prismatik-oss-registry|3|prismatik-determinism"
    # Layer 4
    "prismatik-application|4|prismatik-determinism,prismatik-identity,prismatik-calendar,prismatik-audit,prismatik-manifest,prismatik-storage,prismatik-domain,prismatik-market-data,prismatik-application"
    "prismatik-cli|4|prismatik-manifest"
)

LAYER_DESC=(
    [0]="Layer 0 — Foundational (no workspace deps)"
    [1]="Layer 1 — Kernel extension"
    [2]="Layer 2 — Domain"
    [3]="Layer 3 — Extension and AI"
    [4]="Layer 4 — Application and shell"
)

for entry in "${CRATES[@]}"; do
    IFS='|' read -r name layer deps <<< "$entry"
    dir="crates/$name"
    mkdir -p "$dir/src"
    
    # Cargo.toml
    cat > "$dir/Cargo.toml" <<TOML
[package]
name = "$name"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
description = "PRISMATIK $name crate."

[dependencies]
TOML
    # Add workspace deps
    if [[ -n "$deps" ]]; then
        IFS=',' read -ra DEP_ARRAY <<< "$deps"
        for dep in "${DEP_ARRAY[@]}"; do
            # avoid self-deps in application (idempotent)
            if [[ "$dep" != "$name" ]]; then
                echo "$dep.workspace = true" >> "$dir/Cargo.toml"
            fi
        done
    fi

    # lib.rs
    cat > "$dir/src/lib.rs" <<RS
//! # prismatik-$name
//!
//! ${LAYER_DESC[$layer]}
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: STUB — populated incrementally per wave plan.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

RS
done

echo "Created $(echo "${CRATES[@]}" | wc -w) crate stubs."
ls crates/ | wc -l
