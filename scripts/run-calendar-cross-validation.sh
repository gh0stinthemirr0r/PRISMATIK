#!/usr/bin/env bash
# Zero-tolerance calendar QuantLib cross-validation gate (P0-DK-14).
# Offline: uses committed fixture oracle — QuantLib is not required.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

resolve_python() {
  if [[ -n "${PYTHON:-}" ]]; then
    echo "${PYTHON}"
    return
  fi
  if command -v python3 >/dev/null 2>&1; then
    echo python3
    return
  fi
  if command -v python >/dev/null 2>&1; then
    echo python
    return
  fi
  echo "error: python3/python not found (set PYTHON=...)" >&2
  exit 127
}

PY="$(resolve_python)"
"${PY}" scripts/build_calendar_artifact.py --validate
cargo test -p prismatik-calendar cross_validation -- --nocapture
