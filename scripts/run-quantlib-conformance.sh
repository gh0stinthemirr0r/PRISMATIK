#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo test -p prismatik-quant-kernel quantlib_conformance_500_relative_1e8 -- --nocapture
