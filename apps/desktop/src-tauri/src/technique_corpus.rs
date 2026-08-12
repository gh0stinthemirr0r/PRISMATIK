//! The technique corpus — durable method the desk reasons from.
//!
//! PRISMATIK's knowledge base holds things people *said*: notes, filings,
//! calls. This module seeds it with something different — a survey of the
//! systematic-trading technique families, each recorded with what it measures,
//! what data it needs, and **whether this desk can currently run it**.
//!
//! That last field is the point. A corpus of strategy descriptions is a
//! reading list; a corpus that knows which strategies are blocked, and on
//! what, is a gap register the agents can reason over. Ask an analyst about
//! carry and it can answer that the technique needs a futures term structure
//! the desk does not yet ingest, rather than describing a trade PRISMATIK has
//! no way to take.
//!
//! **Every entry is written here, not copied.** The technique families are
//! drawn from the public systematic-trading literature and the curated lists
//! that index it; the descriptions, the data requirements and every readiness
//! judgement are PRISMATIK's own assessment of PRISMATIK.
//!
//! The corpus is a floor, not a ceiling. It seeds on first run and is
//! idempotent — documents are content-addressed, so re-seeding replaces
//! rather than duplicates — and operators add to the same brain through the
//! Knowledge surface. Editing a description here and restarting supersedes
//! the old version in place.

use prismatik_brain::{Document, DocumentKind};
use prismatik_determinism::ContentHash;
use serde::Serialize;

/// Whether the desk can run a technique today.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Readiness {
    /// Every input exists. It can be built against the current data layer.
    Ready,
    /// Some inputs exist and some do not.
    Partial,
    /// A required dataset is absent. The blocker is named in `blocker`.
    Blocked,
}

impl Readiness {
    fn label(self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::Partial => "PARTIAL",
            Self::Blocked => "BLOCKED",
        }
    }
}

/// One technique family.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Technique {
    pub(crate) slug: &'static str,
    pub(crate) name: &'static str,
    /// Broad family, for grouping on the surface.
    pub(crate) family: &'static str,
    /// What the signal actually measures.
    pub(crate) measures: &'static str,
    /// The inputs required, stated as datasets rather than vendors.
    pub(crate) requires: &'static str,
    pub(crate) readiness: Readiness,
    /// What is missing, or what already covers it. Never empty.
    pub(crate) blocker: &'static str,
}

/// The seeded corpus.
///
/// Ordered by family so the generated documents read coherently when listed.
/// Readiness is assessed against the adapters in
/// `crates/prismatik-market-data/src/adapters/` and the analytics in
/// `prismatik-regime`, `prismatik-options` and `prismatik-backtest` as they
/// actually stand — not against what is planned.
pub(crate) const TECHNIQUES: &[Technique] = &[
    // ---- Price-only. Everything needed is already ingested. ----
    Technique {
        slug: "time-series-momentum",
        name: "Time-series momentum",
        family: "Trend",
        measures:
            "The sign of an instrument's own trailing return over a lookback, typically 3 to 12 \
             months. Long when its own past return is positive, short when negative — a \
             statement about the instrument against itself, not against its peers.",
        requires: "Daily bars over at least a year per instrument.",
        readiness: Readiness::Ready,
        blocker: "Nothing. Daily bars are ingested for every tracked instrument, and the \
                  backtest engine already runs a Momentum strategy on them.",
    },
    Technique {
        slug: "cross-sectional-momentum",
        name: "Cross-sectional momentum",
        family: "Trend",
        measures:
            "Relative trailing return across a universe, usually 12 months skipping the most \
             recent one. Long the top decile, short the bottom. Distinct from time-series \
             momentum: an instrument can rank top while its own return is negative.",
        requires: "Daily bars for a universe wide enough to rank — dozens of names, not a \
                   handful.",
        readiness: Readiness::Partial,
        blocker: "Bars are available, but ranking is only as meaningful as the universe is \
                  wide. The tracked list is a watchlist, not an index; the screener can widen \
                  it, but a survivorship-free historical universe is not yet maintained.",
    },
    Technique {
        slug: "short-term-reversal",
        name: "Short-term reversal",
        family: "Mean reversion",
        measures: "The tendency of a one-week to one-month move to partially retrace. Short the \
             recent winners, long the recent losers — the opposite sign to momentum at a \
             shorter horizon.",
        requires: "Daily bars. Realistic transaction costs, because the effect is small \
                   relative to the turnover it demands.",
        readiness: Readiness::Ready,
        blocker: "Nothing. The backtest engine has a MeanReversion strategy and a fill model \
                  with explicit assumptions.",
    },
    Technique {
        slug: "fifty-two-week-high",
        name: "52-week high effect",
        family: "Trend",
        measures: "Proximity of price to its trailing one-year high, as a ratio. Names near their \
             high have tended to continue — read as an anchoring effect rather than a \
             valuation one.",
        requires: "Daily bars over a year.",
        readiness: Readiness::Ready,
        blocker: "Nothing.",
    },
    Technique {
        slug: "low-volatility-factor",
        name: "Low volatility and betting against beta",
        family: "Risk factor",
        measures:
            "Realized volatility, or beta to a market proxy. Low-volatility and low-beta names \
             have historically delivered better risk-adjusted returns than the leverage-adjusted \
             theory predicts.",
        requires: "Daily bars plus a market index series for beta.",
        readiness: Readiness::Ready,
        blocker: "Nothing. Realized volatility is computed in prismatik-regime, and the \
                  covariance and correlation matrices in prismatik-risk give beta directly.",
    },
    Technique {
        slug: "pairs-trading",
        name: "Pairs trading and statistical arbitrage",
        family: "Relative value",
        measures: "The spread between two historically co-moving instruments, normalised to a \
             z-score. Enter when the spread is stretched, exit on reversion. Cointegration \
             rather than correlation is the honest test — correlated series can drift apart \
             forever.",
        requires: "Aligned daily bars for both legs and a cointegration test.",
        readiness: Readiness::Partial,
        blocker: "Bars and a sample covariance matrix exist in prismatik-risk. No cointegration \
                  test (Engle-Granger or Johansen) is implemented, and that is the part that \
                  separates a real pair from a coincidence.",
    },
    Technique {
        slug: "sector-and-asset-rotation",
        name: "Sector and asset-class rotation",
        family: "Trend",
        measures: "Relative strength across sector or asset-class proxies, rotating into the \
             strongest and out of the weakest on a fixed schedule.",
        requires: "Daily bars for a set of sector or asset-class ETFs.",
        readiness: Readiness::Ready,
        blocker: "Nothing, provided the proxies are tracked.",
    },
    Technique {
        slug: "calendar-seasonality",
        name: "Calendar and seasonality effects",
        family: "Seasonality",
        measures: "Recurring calendar structure: turn-of-the-month, the January barometer, payday \
             flows, option-expiration week, intraday patterns in crypto. Each is a claim that \
             the date carries information.",
        requires: "Daily or intraday bars with a reliable trading calendar.",
        readiness: Readiness::Partial,
        blocker: "Daily bars and prismatik-calendar are present. Intraday bars are not \
                  routinely ingested, so the intraday members of this family cannot be tested. \
                  These effects are also the most prone to being artefacts of the sample — \
                  they need the scored forecast loop more than most.",
    },
    Technique {
        slug: "crypto-rebalancing-premium",
        name: "Rebalancing premium in crypto",
        family: "Portfolio construction",
        measures: "The excess return of a periodically rebalanced basket over buy-and-hold, which \
             rises with constituent volatility and falls with correlation. A portfolio-\
             construction effect rather than a forecast.",
        requires: "Daily bars for a basket of crypto assets.",
        readiness: Readiness::Ready,
        blocker: "Nothing. Crypto bars come through the CoinGecko adapter.",
    },
    // ---- Needs fundamentals. Filings arrive; normalised financials do not. ----
    Technique {
        slug: "value-factor",
        name: "Value",
        family: "Fundamental factor",
        measures:
            "Price relative to a fundamental anchor — book, earnings, cash flow. Long cheap, \
             short expensive, on the premise that the ratio mean-reverts.",
        requires: "Point-in-time fundamentals aligned to when they were public, not when they \
                   were restated.",
        readiness: Readiness::Blocked,
        blocker: "SEC EDGAR filings are ingested as documents, but no normalised financial \
                  statement extraction exists. Point-in-time alignment is the harder half: \
                  using restated figures is the classic way to backtest a look-ahead bias into \
                  a value strategy and not notice.",
    },
    Technique {
        slug: "quality-and-accruals",
        name: "Quality, accruals and asset growth",
        family: "Fundamental factor",
        measures:
            "Earnings quality signals — the accrual component of earnings, return on assets, \
             balance-sheet growth, composite F-score style ranks. High-accrual and \
             fast-growing firms have tended to underperform.",
        requires: "Point-in-time balance sheet and cash flow statements.",
        readiness: Readiness::Blocked,
        blocker: "Same blocker as value: filings are stored as text, not parsed into \
                  statements.",
    },
    Technique {
        slug: "event-driven-earnings",
        name: "Earnings announcement effects",
        family: "Event",
        measures: "Return behaviour around scheduled earnings — the announcement premium, \
             post-announcement drift, and reversal patterns, sometimes conditioned on \
             buybacks or short interest.",
        requires: "A forward earnings calendar and daily bars; short interest for the \
                   conditioned variants.",
        readiness: Readiness::Partial,
        blocker: "prismatik-calendar and prismatik-filings provide the scaffolding and bars \
                  are present. No earnings-date feed is wired, and no short-interest dataset \
                  exists.",
    },
    Technique {
        slug: "filing-text-signals",
        name: "Filing and news text signals",
        family: "Alternative data",
        measures: "Language rather than numbers: lexical density and readability of filings, tone \
             changes between successive reports, overnight sentiment. The claim is that how a \
             company writes carries information its statements do not.",
        requires: "Full filing text across successive periods, and a text pipeline.",
        readiness: Readiness::Partial,
        blocker: "EDGAR text is ingested and prismatik-brain indexes it with BM25 and entity \
                  links. There is no period-over-period differencing, which is where most of \
                  this family's signal lives.",
    },
    // ---- Needs instruments the desk does not carry. ----
    Technique {
        slug: "fx-carry",
        name: "FX carry and currency factors",
        family: "Carry",
        measures: "Interest-rate differentials across currencies — long high-yielders, short \
             low-yielders — plus currency momentum and PPP-based value. Carry is famously a \
             short-volatility exposure: it earns steadily and loses violently.",
        requires: "Spot FX rates, forward points or short rates per currency.",
        readiness: Readiness::Blocked,
        blocker: "No FX adapter. FRED supplies some rate series, but no spot or forward FX \
                  curve. Nothing in this family can be attempted without it.",
    },
    Technique {
        slug: "commodity-term-structure",
        name: "Commodity term structure and carry",
        family: "Carry",
        measures: "The slope of the futures curve. Backwardated markets have delivered positive \
             roll yield and contangoed ones negative, which is carry expressed in futures \
             rather than rates. Related members: commodity momentum, skewness and return \
             asymmetry.",
        requires: "Futures curves by contract month, with a documented roll convention.",
        readiness: Readiness::Blocked,
        blocker: "No futures adapter. The CFTC adapter supplies positioning (Commitments of \
                  Traders), which is a sentiment input, not a price curve. The roll convention \
                  matters as much as the data: a continuous series stitched differently is a \
                  different series.",
    },
    Technique {
        slug: "spread-trading",
        name: "Physical and calendar spreads",
        family: "Relative value",
        measures: "The difference between two related contracts — WTI against Brent, or one \
             delivery month against another — traded on the premise that the economic link \
             bounds the spread.",
        requires: "Aligned futures series for both legs.",
        readiness: Readiness::Blocked,
        blocker: "Same futures gap as term structure.",
    },
    // ---- Needs the options chain. ----
    Technique {
        slug: "volatility-risk-premium",
        name: "Volatility risk premium",
        family: "Volatility",
        measures: "The persistent gap between implied and subsequently realized volatility. \
             Harvested by selling options — typically a short at-the-money straddle with a \
             far out-of-the-money put bought back as crash protection.",
        requires: "An options chain with strikes and expiries, implied volatilities, and a \
                   realized-volatility estimator.",
        readiness: Readiness::Partial,
        blocker: "prismatik-options now prices, solves implied volatility and produces greeks, \
                  and prismatik-regime supplies realized volatility. The chain itself is \
                  missing: the unusual_whales adapter carries options *flow*, not strikes and \
                  quotes. A chain adapter is the single remaining input.",
    },
    Technique {
        slug: "dispersion-trading",
        name: "Dispersion trading",
        family: "Volatility",
        measures: "The gap between index implied volatility and the volatility implied by its \
             constituents — equivalently, the implied correlation embedded in index options. \
             Selling index volatility against buying constituent volatility is a short \
             implied-correlation position, which pays when names move independently and \
             loses hard when everything sells off together.",
        requires: "Option chains on both the index and its constituents, index weights, and \
                   a correlation estimator. Variants rank constituents by analyst forecast \
                   dispersion, which needs an estimates dataset.",
        readiness: Readiness::Blocked,
        blocker: "Needs the chain adapter, plus index constituent weights. The pricing and \
                  implied-vol machinery exists; the data does not. The analyst-dispersion \
                  variant additionally needs an estimates feed (I/B/E/S or equivalent) that \
                  no current adapter supplies.",
    },
    // ---- Cross-asset and macro predictors. ----
    Technique {
        slug: "cross-asset-predictors",
        name: "Cross-asset and macro predictors",
        family: "Macro",
        measures: "One market used to forecast another: crude oil against equity returns, the Fed \
             model comparing earnings yield to bond yield, synthetic lending rates from \
             options as a market-return predictor.",
        requires: "Aligned macro and market series; the lending-rate variant needs an options \
                   chain.",
        readiness: Readiness::Partial,
        blocker: "The FRED adapter covers macro series and bars cover the market side, so the \
                  oil and Fed-model variants are buildable now. The options-derived variant \
                  waits on the chain.",
    },
];

/// A technique with its readiness, for the gap register surface.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TechniqueView {
    #[serde(flatten)]
    pub(crate) technique: Technique,
    pub(crate) readiness_label: &'static str,
}

/// Every technique, with counts the caller can summarise.
#[tauri::command]
pub(crate) fn list_techniques() -> Vec<TechniqueView> {
    TECHNIQUES
        .iter()
        .map(|technique| TechniqueView {
            technique: *technique,
            readiness_label: technique.readiness.label(),
        })
        .collect()
}

/// Render one technique as a knowledge-base document.
///
/// The readiness line is written into the body rather than kept as metadata
/// so that retrieval surfaces it: an agent that finds "dispersion trading"
/// should see in the same breath that the desk cannot currently run it.
fn document_for(technique: &Technique) -> Document {
    let body = format!(
        "Family: {family}.\n\n\
         What it measures: {measures}\n\n\
         Data required: {requires}\n\n\
         PRISMATIK readiness: {readiness}. {blocker}",
        family = technique.family,
        measures = technique.measures,
        requires = technique.requires,
        readiness = technique.readiness.label(),
        blocker = technique.blocker,
    );
    Document {
        // Content-addressed on the slug and the body, so re-seeding an
        // unchanged corpus is a no-op and an edited description supersedes
        // the old one rather than sitting beside it.
        id: ContentHash::from_bytes(format!("technique:{}|{}", technique.slug, body).as_bytes())
            .to_string(),
        kind: DocumentKind::Technique,
        title: format!("Technique — {}", technique.name),
        body,
        // Techniques are about method, not instruments. Leaving `about` empty
        // lets the extractor link whatever symbols the text genuinely
        // mentions rather than asserting a relationship that is not there.
        about: Vec::new(),
        created_at: "2026-01-01T00:00:00Z".to_owned(),
    }
}

/// Every corpus entry as a document, ready to insert.
pub(crate) fn documents() -> Vec<Document> {
    TECHNIQUES.iter().map(document_for).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_technique_is_completely_specified() {
        // A corpus entry with an empty field would retrieve as a confident
        // heading with nothing under it.
        for technique in TECHNIQUES {
            for (field, value) in [
                ("slug", technique.slug),
                ("name", technique.name),
                ("family", technique.family),
                ("measures", technique.measures),
                ("requires", technique.requires),
                ("blocker", technique.blocker),
            ] {
                assert!(
                    !value.trim().is_empty(),
                    "{} has an empty {field}",
                    technique.slug,
                );
            }
        }
    }

    #[test]
    fn slugs_are_unique() {
        let slugs: BTreeSet<&str> = TECHNIQUES.iter().map(|t| t.slug).collect();
        assert_eq!(
            slugs.len(),
            TECHNIQUES.len(),
            "duplicate slug in the corpus"
        );
    }

    #[test]
    fn a_ready_technique_still_says_why_it_is_ready() {
        // "Ready" with a blank explanation is the same failure as an
        // unexplained blocker: the reader cannot check the claim.
        for technique in TECHNIQUES
            .iter()
            .filter(|t| t.readiness == Readiness::Ready)
        {
            assert!(
                technique.blocker.len() > 5,
                "{} claims READY without saying what covers it",
                technique.slug,
            );
        }
    }

    #[test]
    fn documents_are_content_addressed_and_stable() {
        // Seeding twice must not duplicate the corpus.
        let first = documents();
        let second = documents();
        let ids: Vec<&String> = first.iter().map(|d| &d.id).collect();
        let repeat: Vec<&String> = second.iter().map(|d| &d.id).collect();
        assert_eq!(ids, repeat);

        let unique: BTreeSet<&String> = ids.iter().copied().collect();
        assert_eq!(unique.len(), TECHNIQUES.len());
    }

    #[test]
    fn readiness_reaches_retrieval_through_the_body() {
        // The whole value of the corpus is that a retrieval hit carries the
        // readiness with it. If it lived only in metadata, an agent quoting a
        // technique would describe a trade the desk cannot take.
        for document in documents() {
            assert!(
                document.body.contains("PRISMATIK readiness:"),
                "{} lost its readiness line",
                document.title,
            );
        }
        let dispersion = documents()
            .into_iter()
            .find(|d| d.title.contains("Dispersion"))
            .expect("dispersion is in the corpus");
        assert!(dispersion.body.contains("BLOCKED"));
    }

    #[test]
    fn the_corpus_covers_every_family_we_surveyed() {
        let families: BTreeSet<&str> = TECHNIQUES.iter().map(|t| t.family).collect();
        for expected in [
            "Trend",
            "Mean reversion",
            "Risk factor",
            "Relative value",
            "Seasonality",
            "Fundamental factor",
            "Event",
            "Alternative data",
            "Carry",
            "Volatility",
            "Macro",
            "Portfolio construction",
        ] {
            assert!(families.contains(expected), "no {expected} technique");
        }
    }
}
