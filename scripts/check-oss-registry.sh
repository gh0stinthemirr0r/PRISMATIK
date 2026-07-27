#!/usr/bin/env sh
# P0-SS-05: every workspace member must appear in prismatik-oss-registry.
set -eu
cd "$(dirname "$0")/.."
cargo test -p prismatik-oss-registry registry_coverage -- --nocapture
