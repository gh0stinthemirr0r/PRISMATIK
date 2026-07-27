# ADR-0032 — Naming Consolidation After Acquisition

**Status:** Accepted (amended 2026-07-26)  
**Date:** 2026-07-26  
**Context:** Pre-acquisition and research-borrowed vocabulary (e.g. `KronosFloorTokenizer`, `UwFlowPrint`, YATA-branded façades, research-repo model names) leaked into PRISMATIK-owned APIs, making product boundaries unclear.

## Decision

1. **Keep** real third-party **integration** identifiers — systems we actually call:
   - Provider adapters and `ProviderId` (`coingecko`, `alpaca`, `finnhub`, `unusual_whales`, …)
   - Live API env vars (`ALPACA_*`, `FINNHUB_*`, …)
   - Wire protocol field names required for interoperability (e.g. OpenLineage camelCase)
   - Comments that attribute a methodology or typical upstream once (“typical backend: MAPIE”, López de Prado purge/embargo)

2. **Rename** all **PRISMATIK-owned** types, modules, functions, catalogs, fixture ids, and user-visible copy that borrowed foreign product or research-repo branding:
   - Floor stubs (`KronosFloorTokenizer` → `PrismatikSeriesTokenizer`)
   - Domain types named after a vendor (`UwFlowPrint` → `OptionsFlowPrint`)
   - Façade modules branded as foreign crates (`yata` → `external_indicator`)
   - Sidecar enums that duplicate vendor marketing (`MapieAci` → `AdaptiveConformal`)
   - TSFM **capability roles** — keep plural-registry / license-deny / tokenizer-binding **logic**, but use Prismatik role names (`FinancialBar`, `GeneralCovariate`, `StreamingSequence`, `LargeScaleForecast`, `ProbabilisticBaseline`, `RestrictedLicense`). Do **not** brand owned enums after GitHub/research repos we are not integrating as APIs (e.g. Kronos).
   - UI / workbook strings that present methodology brands as our product (prefer “purged + embargoed walk-forward” over “AFML” in UX)

3. **Do not** rename `legacy-v0.1/**`, existing `prismatik-*` crate names, or third-party crate dependencies. Historical architecture PDFs may retain assessment prose; living scorecards and code must use Prismatik vocabulary.

## Consequences

- New engineers should read owned APIs as PRISMATIK / domain vocabulary.
- Live provider integrations remain explicitly named so operators know what was wired.
- Research-model *capabilities* (bar tokenizer, license hard-deny, contamination gates) stay; research-repo *names* do not.
- Future floors must not reintroduce borrowed branding into type, function, or catalog variant names.

## Related

- Buyout naming consolidation plan (2026-07-26)
- `ProviderId` in `prismatik-domain`
- `OnboardedTsfmFamily` in `prismatik-tsfm` (role catalogue)
