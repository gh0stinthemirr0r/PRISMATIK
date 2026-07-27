# PRISMATIK User Guide (Wave 1)

## Install

1. Download the signed desktop installer for your platform (Windows MSI / NSIS when bundling is enabled).
2. On first launch, complete **Onboarding**: suitability acknowledgement → Demo cassette (offline) or Live keys.
3. Open **Workspace** from the landing page.

## Demo mode

Demo mode replays CoinGecko **cassettes** through the trusted Rust core. No network is required. Ctrl+K search works for recorded queries (`btc`, `eth`, `sol`, `link`, …).

## Live mode

Set `PRISMATIK_DATA_MODE=live` and provide a CoinGecko API key via the OS keychain / preference surface. All live calls pass the GCRA budget governor; the rate meter shows exact next-permit times.

## Chaos / degraded

Set `PRISMATIK_DATA_MODE=chaos` to blackhole CoinGecko. The workspace shows a degraded banner and falls back to cached/cassette paths where available.

## Evidence

Every quote and chart should resolve to an evidence chain (provider + retrieval timestamp + content hash). Open an asset page for the evidence drawer.

## AI (local)

LM Studio providers only accept **loopback** URLs. Model digests are required. Hosted providers require explicit API keys / OAuth and per-call consent for escalation tiers.

## Backup

Use **Settings → Backup** (application `backup` module) to export the data directory. Restore on the next app version verifies N→N+1 migration.

## Licensing

See in-app **Disclosures** and the generated `NOTICE` file for third-party licenses.
