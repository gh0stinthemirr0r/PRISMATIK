#!/usr/bin/env bash
# Trusted-core cargo-vet check (P0-SS-04).
# Scopes vetting to production deps of:
#   prismatik-determinism, prismatik-identity, prismatik-audit, prismatik-manifest
# by excluding other workspace members and is_dev_only packages.
set -euo pipefail
cd "$(dirname "$0")/.."

FILTER='exclude(any(name(prismatik-calendar),name(prismatik-storage),name(prismatik-domain),name(prismatik-market-data),name(prismatik-features),name(prismatik-analog-store),name(prismatik-indicator-core),name(prismatik-quant-kernel),name(prismatik-strategy),name(prismatik-backtest),name(prismatik-simulation),name(prismatik-tsfm),name(prismatik-calibration),name(prismatik-crypto),name(prismatik-options),name(prismatik-filings),name(prismatik-cot),name(prismatik-events),name(prismatik-risk),name(prismatik-portfolio),name(prismatik-execution),name(prismatik-journal),name(prismatik-plugin-host),name(prismatik-ai-router),name(prismatik-ai-tools),name(prismatik-security),name(prismatik-observability),name(prismatik-renderer),name(prismatik-oss-registry),name(prismatik-application),name(prismatik-cli),is_dev_only(true)))'

echo "cargo vet check (trusted-core filter-graph)"
exec cargo vet check --locked --filter-graph="$FILTER"
