//! The source register — what PRISMATIK may legally build on.
//!
//! PRISMATIK is closed-source commercial software. That makes licence a
//! correctness property, not paperwork: an AGPL parser or a scraped feed does
//! not fail a test, it fails in a way that surfaces years later as a legal
//! problem. This register records the verdict for every data source and
//! library the survey turned up, so the question is answered once instead of
//! re-litigated each time somebody reaches for an obvious-looking endpoint.
//!
//! It seeds into the knowledge base alongside the technique corpus, which
//! means the agents can reason over it too. An analyst asked how to get an
//! options chain should be able to find that the Cboe delayed-quote endpoint
//! every open-source project uses is one Cboe prohibits automated extraction
//! from — before somebody wires it up, not after.
//!
//! **The prohibited entries are the point.** A register that only listed the
//! good options would leave the attractive traps undocumented, and those are
//! exactly what a search finds first.

use prismatik_brain::{Document, DocumentKind};
use prismatik_determinism::ContentHash;
use serde::Serialize;

/// Whether this source can be used in a closed-source commercial product.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Standing {
    /// Verified permissive or public domain. Use freely.
    Clear,
    /// Usable, but with a condition or cost that has to be honoured.
    Conditional,
    /// Must not be used. Licence forbids it, or the operator forbids the access.
    Prohibited,
    /// Could not be established. Treat as prohibited until resolved.
    Unverified,
}

impl Standing {
    fn label(self) -> &'static str {
        match self {
            Self::Clear => "CLEAR",
            Self::Conditional => "CONDITIONAL",
            Self::Prohibited => "PROHIBITED",
            Self::Unverified => "UNVERIFIED",
        }
    }
}

/// One data source or library, with its verdict.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Source {
    pub(crate) slug: &'static str,
    pub(crate) name: &'static str,
    /// What need it would serve.
    pub(crate) fills: &'static str,
    /// Licence or terms, as verified.
    pub(crate) terms: &'static str,
    pub(crate) standing: Standing,
    /// The reasoning. Never empty — a verdict without one cannot be checked.
    pub(crate) note: &'static str,
}

pub(crate) const SOURCES: &[Source] = &[
    // ---- Public domain. The best answers in the survey. ----
    Source {
        slug: "sec-financial-statement-data-sets",
        name: "SEC Financial Statement Data Sets",
        fills: "Point-in-time fundamentals",
        terms: "US Government work — public domain",
        standing: Standing::Clear,
        note: "Quarterly flat files carrying every statement line item as filed, with a \
               submission table giving the filing date. Point-in-time by construction, and it \
               sidesteps XBRL parsing entirely. The single highest-value finding of the survey: \
               it closes the fundamentals gap for nothing, and is mostly a rework of the EDGAR \
               adapter already in the tree rather than a new integration.",
    },
    Source {
        slug: "eia-open-data",
        name: "EIA Open Data API",
        fills: "Energy futures curves",
        terms: "US Government work — public domain",
        standing: Standing::Clear,
        note: "Serves NYMEX contract-1 through contract-4 settlements for WTI, natural gas, \
               heating oil and RBOB. A genuine four-point energy curve with no licence \
               question. Coverage is energy only, and a gap in petroleum futures after April \
               2024 was reported — verify the current series before relying on it.",
    },
    Source {
        slug: "ecb-data-portal",
        name: "ECB Data Portal (SDMX)",
        fills: "FX spot, euro-area rates",
        terms: "Free to reuse commercially; attribution required",
        standing: Standing::Clear,
        note: "The ESCB treats its statistics as a public good, explicitly free to reuse \
               irrespective of commercial or non-commercial use, conditional on citing the \
               source. Attribution is the whole obligation.",
    },
    Source {
        slug: "frankfurter",
        name: "Frankfurter",
        fills: "FX spot",
        terms: "Explicitly free for commercial use",
        standing: Standing::Clear,
        note: "Keyless, no daily cap, self-hostable via Docker, 201 currencies with history to \
               1948, aggregated from central banks. Self-hosting removes the rate-limit \
               question entirely. Their terms pass through a caveat to check each underlying \
               provider.",
    },
    // ---- Usable with a condition. ----
    Source {
        slug: "alpaca-option-snapshots",
        name: "Alpaca option snapshots",
        fills: "Equity options chains",
        terms: "Permitted under standard Alpaca terms",
        standing: Standing::Conditional,
        note: "Runs on a credential the desk already holds, so integration cost is near zero. \
               The free plan is delayed fifteen minutes and indicative rather than full OPRA — \
               irrelevant to a monthly strategy, disqualifying for anything intraday.",
    },
    Source {
        slug: "deribit-public",
        name: "Deribit public endpoints",
        fills: "Crypto options chains and term structure",
        terms: "Unauthenticated public market data",
        standing: Standing::Conditional,
        note: "The cleanest free full-chain source that exists, and the standard test bed for \
               volatility surface calibration. Crypto only — but that makes crypto the natural \
               first market for the chain, surface and term-structure work, with no vendor \
               negotiation at all.",
    },
    Source {
        slug: "databento",
        name: "Databento",
        fills: "OPRA chains, CME/ICE futures, intraday bars",
        terms: "Apache-2.0 Rust client; metered paid data",
        standing: Standing::Conditional,
        note: "The only compliant path to broad CME curves, and it closes three gaps in one \
               integration. The client crate is Apache-2.0 so there is no linking problem; the \
               data is a running cost. Symbology and definition calls are free.",
    },
    Source {
        slug: "volsurf",
        name: "volsurf crate",
        fills: "Volatility surface fitting",
        terms: "Apache-2.0",
        standing: Standing::Clear,
        note: "SVI, SABR, SSVI, Dupire local vol, calibration, no-panic Result API. Small \
               scope, actively maintained. Reimplementing this would be waste.",
    },
    Source {
        slug: "unit-root",
        name: "unit-root crate",
        fills: "Augmented Dickey-Fuller",
        terms: "Apache-2.0",
        standing: Standing::Conditional,
        note: "Complete and tiny, but unmaintained since 2023. Vendor it rather than depend on \
               it. This is the ADF half of Engle-Granger.",
    },
    // ---- Must not be used. These are the entries that earn the register. ----
    Source {
        slug: "cboe-delayed-quotes",
        name: "Cboe delayed-quote JSON endpoint",
        fills: "Equity options chains",
        terms: "Automated extraction prohibited by Cboe",
        standing: Standing::Prohibited,
        note: "It works, it is unauthenticated, and open-source projects use it constantly — \
               which is exactly why it is in this register. Cboe prohibits automated extraction \
               of delayed quotes and blocks the addresses of parties who attempt it. The data \
               is Cboe LiveVol property. A legal and operational hazard for a commercial \
               product, and the most likely trap for anyone solving the chain problem by \
               search.",
    },
    Source {
        slug: "nautilus-trader",
        name: "NautilusTrader (nautilus-* crates)",
        fills: "Trading framework",
        terms: "LGPL-3.0-or-later",
        standing: Standing::Prohibited,
        note: "Rust's static linking makes the LGPL relinking obligation effectively \
               unsatisfiable in a shipped closed-source binary. It also duplicates the backtest \
               engine wholesale, so the licence question is moot twice over.",
    },
    Source {
        slug: "crabrl",
        name: "crabrl XBRL parser",
        fills: "Fundamentals parsing",
        terms: "AGPL-3.0",
        standing: Standing::Prohibited,
        note: "The most obvious fit for the fundamentals gap, and unusable. Moot in any case: \
               the SEC flat files avoid XBRL entirely.",
    },
    Source {
        slug: "sec-fetcher",
        name: "sec-fetcher",
        fills: "EDGAR retrieval",
        terms: "PolyForm Noncommercial 1.0.0",
        standing: Standing::Prohibited,
        note: "Explicitly forbids commercial use.",
    },
    Source {
        slug: "hurst-crate",
        name: "hurst crate",
        fills: "Hurst exponent",
        terms: "GPL-3.0-or-later",
        standing: Standing::Prohibited,
        note: "Disqualifying, and unnecessary — rescaled-range analysis is a short function \
               implementable from the published method.",
    },
    Source {
        slug: "yahoo-derived-crates",
        name: "Yahoo Finance-backed crates",
        fills: "Market data",
        terms: "Crate licence permissive; underlying data terms are not",
        standing: Standing::Prohibited,
        note: "The trap worth naming: a permissive licence on the wrapper says nothing about \
               the terms on the data behind it. Yahoo prohibits commercial redistribution.",
    },
    // ---- Unresolved. Treated as prohibited until someone checks. ----
    Source {
        slug: "dolthub-options",
        name: "DoltHub options price history",
        fills: "Historical options chains",
        terms: "Could not be confirmed",
        standing: Standing::Unverified,
        note: "Daily US equity option prices since 2021, clonable as a SQL database — excellent \
               for backtest history if the terms permit. Nobody has read them. Unverified is \
               not a soft yes.",
    },
    Source {
        slug: "generated-quant-crates",
        name: "High-volume unvalidated Rust quant crates",
        fills: "Claims to fill several gaps at once",
        terms: "Permissive, but that is not the problem",
        standing: Standing::Unverified,
        note: "A class rather than one package. Several crates advertise exactly the missing \
               pieces — Johansen, VECM, GARCH variants, Black-Litterman, hundreds of indicators \
               — with a handful of stars, no citations, and dozens of releases in weeks; at \
               least one concedes in its own README that the headline function is a \
               placeholder. They appear to be machine-generated. For a trading terminal \
               numerical correctness is the product, so small verifiable scope beats broad \
               claims every time.",
    },
];

/// A source with its label, for the register surface.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceView {
    #[serde(flatten)]
    pub(crate) source: Source,
    pub(crate) standing_label: &'static str,
}

#[tauri::command]
pub(crate) fn list_sources() -> Vec<SourceView> {
    SOURCES
        .iter()
        .map(|source| SourceView {
            source: *source,
            standing_label: source.standing.label(),
        })
        .collect()
}

fn document_for(source: &Source) -> Document {
    let body = format!(
        "Fills: {fills}.\n\nTerms: {terms}.\n\nStanding: {standing}. {note}",
        fills = source.fills,
        terms = source.terms,
        standing = source.standing.label(),
        note = source.note,
    );
    Document {
        id: ContentHash::from_bytes(format!("source:{}|{}", source.slug, body).as_bytes())
            .to_string(),
        kind: DocumentKind::Technique,
        title: format!("Source — {}", source.name),
        body,
        about: Vec::new(),
        created_at: "2026-01-01T00:00:00Z".to_owned(),
    }
}

pub(crate) fn documents() -> Vec<Document> {
    SOURCES.iter().map(document_for).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_source_carries_its_reasoning() {
        for source in SOURCES {
            assert!(
                !source.note.trim().is_empty(),
                "{} has no note",
                source.slug
            );
            assert!(
                !source.terms.trim().is_empty(),
                "{} has no terms",
                source.slug
            );
        }
    }

    #[test]
    fn slugs_are_unique() {
        let slugs: BTreeSet<&str> = SOURCES.iter().map(|s| s.slug).collect();
        assert_eq!(slugs.len(), SOURCES.len());
    }

    #[test]
    fn the_register_records_prohibitions_not_just_permissions() {
        // A register listing only the usable options would leave the
        // attractive traps undocumented — and those are what a search finds
        // first. Cboe in particular must stay named.
        let prohibited: Vec<&str> = SOURCES
            .iter()
            .filter(|s| s.standing == Standing::Prohibited)
            .map(|s| s.slug)
            .collect();
        assert!(prohibited.len() >= 5, "too few prohibitions to be credible");
        assert!(prohibited.contains(&"cboe-delayed-quotes"));
        assert!(prohibited.contains(&"nautilus-trader"));
    }

    #[test]
    fn standing_reaches_retrieval_through_the_body() {
        for document in documents() {
            assert!(document.body.contains("Standing:"), "{}", document.title);
        }
        let cboe = documents()
            .into_iter()
            .find(|d| d.title.contains("Cboe"))
            .expect("cboe is registered");
        assert!(cboe.body.contains("PROHIBITED"));
    }

    #[test]
    fn unverified_is_not_a_soft_yes() {
        // The register exists so nobody reads a blank as permission.
        for source in SOURCES
            .iter()
            .filter(|s| s.standing == Standing::Unverified)
        {
            assert!(
                source.note.len() > 20,
                "{} is unverified without explaining what is unresolved",
                source.slug,
            );
        }
    }
}
