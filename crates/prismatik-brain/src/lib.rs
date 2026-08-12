//! The knowledge layer: entity-linked documents, retrieval, and gap analysis.
//!
//! # Why this is not a vector store
//!
//! The obvious build is embeddings plus a vector database. This does something
//! narrower and, for a trading desk, more useful.
//!
//! A market brain's entities are not open-vocabulary. They are instruments, and
//! the desk already holds an authoritative list of them. So entity extraction
//! here is exact matching against a known symbol vocabulary rather than
//! probabilistic named-entity recognition: it costs no model call, cannot
//! hallucinate a link, and is right or wrong in a way you can test. Every write
//! wires its own typed edges as a side effect of being stored.
//!
//! Retrieval is BM25 over the text plus traversal over those edges. That is a
//! deliberate trade: it gives up paraphrase matching, and gets determinism, no
//! embedding model, no vector index to keep in sync, and results whose
//! provenance is a term match rather than a cosine distance nobody can audit.
//!
//! # Gap analysis
//!
//! Retrieval that returns the best five documents for a query about which the
//! brain knows nothing is worse than useless — it launders absence into
//! confidence. Every query therefore reports which of its terms and symbols the
//! corpus does not cover, so an agent can say what it does not know.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// What a document is, so retrieval can weight and filter by provenance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    /// An operator's own note.
    Note,
    /// A regulatory filing or extract.
    Filing,
    /// A news item or headline.
    News,
    /// Model or agent output retained for later reference.
    Research,
    /// A recorded decision and its outcome.
    Decision,
    /// A transcript, e.g. an earnings call.
    Transcript,
    /// A trading technique: what it measures, what it needs, whether this
    /// desk can currently run it.
    ///
    /// Distinct from `Research` because a technique is durable method rather
    /// than a dated opinion. A note about NVDA goes stale; the definition of
    /// a carry trade does not, and the two should not be retrieved with the
    /// same recency assumptions.
    Technique,
}

/// A stored document.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: String,
    pub kind: DocumentKind,
    pub title: String,
    pub body: String,
    /// Instruments this document is *about*, as opposed to merely mentioned.
    ///
    /// Supplied by the caller when known (a filing is about its issuer); left
    /// empty otherwise and inferred from mentions.
    #[serde(default)]
    pub about: Vec<String>,
    /// RFC3339 creation time, used only for recency tie-breaking.
    pub created_at: String,
}

/// How a document relates to an instrument.
/// Ordering is meaningful: `About` sorts before `Mentions` so a document's
/// subject outranks a passing reference in every listing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// The document is principally about this instrument.
    About,
    /// The instrument appears in the text.
    Mentions,
}

/// A typed edge from a document to an instrument.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub document_id: String,
    pub symbol: String,
    pub kind: EdgeKind,
}

/// A retrieval hit.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub document: Document,
    /// BM25 score, plus any graph boost.
    pub score: f64,
    /// Terms from the query that this document actually matched.
    pub matched_terms: Vec<String>,
    /// True when the document was reached through an entity edge rather than
    /// by matching query text.
    pub via_graph: bool,
}

/// What a query found, and what it could not.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub hits: Vec<Hit>,
    /// Query terms with no coverage anywhere in the corpus.
    ///
    /// The honest half of retrieval: an agent that reports these can say what
    /// it does not know instead of answering confidently from whatever ranked
    /// highest.
    pub uncovered_terms: Vec<String>,
    /// Symbols named in the query that the brain holds nothing about.
    pub uncovered_symbols: Vec<String>,
    pub corpus_size: usize,
}

/// Words carrying no retrieval signal.
const STOPWORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "of", "to", "in", "on", "for", "is", "are", "was",
    "were", "be", "been", "it", "its", "this", "that", "these", "those", "with", "as", "at", "by",
    "from", "has", "have", "had", "will", "would", "what", "which", "who", "how", "why", "do",
    "does", "did", "about", "into", "over", "than", "then", "so", "if", "we", "i", "you", "they",
];

/// BM25 term-frequency saturation.
const K1: f64 = 1.2;
/// BM25 length normalization.
const B: f64 = 0.75;
/// Score multiplier for a document reached only through an entity edge.
///
/// Below 1 on purpose: a graph neighbour is context, not an answer, and should
/// never outrank a document that actually matched the question.
const GRAPH_WEIGHT: f64 = 0.35;

/// Split text into lowercase alphanumeric terms, dropping stopwords.
pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .filter(|token| token.len() > 1 && !STOPWORDS.contains(&token.as_str()))
        .collect()
}

/// The knowledge base.
#[derive(Debug, Default)]
pub struct Brain {
    documents: BTreeMap<String, Document>,
    /// term -> document id -> term frequency
    postings: BTreeMap<String, BTreeMap<String, usize>>,
    /// document id -> token count, for length normalization
    lengths: BTreeMap<String, usize>,
    edges: Vec<Edge>,
    /// The instrument vocabulary entity extraction matches against.
    vocabulary: BTreeSet<String>,
}

impl Brain {
    /// Construct with the instrument vocabulary to link against.
    ///
    /// Extraction is exact matching, so the vocabulary defines what the brain
    /// can link. Passing the tracked list plus anything screened keeps links
    /// grounded in instruments the desk actually knows about.
    pub fn new(vocabulary: impl IntoIterator<Item = String>) -> Self {
        Self {
            vocabulary: vocabulary
                .into_iter()
                .map(|symbol| symbol.trim().to_uppercase())
                .filter(|symbol| !symbol.is_empty())
                .collect(),
            ..Default::default()
        }
    }

    /// Extend the vocabulary. Existing documents are not re-linked.
    pub fn learn_symbols(&mut self, symbols: impl IntoIterator<Item = String>) {
        for symbol in symbols {
            let symbol = symbol.trim().to_uppercase();
            if !symbol.is_empty() {
                self.vocabulary.insert(symbol);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.documents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Symbols mentioned in a text, matched against the vocabulary.
    ///
    /// Case-sensitive on the token but vocabulary-bounded, so ordinary prose
    /// cannot manufacture a link: "it" and "a" are never symbols, and a ticker
    /// only counts when the desk already knows it.
    pub fn extract_symbols(&self, text: &str) -> Vec<String> {
        let mut found = BTreeSet::new();
        for raw in text.split(|c: char| !c.is_alphanumeric()) {
            if raw.is_empty() {
                continue;
            }
            let candidate = raw.to_uppercase();
            // Require the source token to be upper-case or the exact ticker, so
            // the word "all" does not link to a hypothetical ALL.
            let looks_like_ticker = raw
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
            if looks_like_ticker && self.vocabulary.contains(&candidate) {
                found.insert(candidate);
            }
        }
        found.into_iter().collect()
    }

    /// Store a document and wire its edges. Replaces any document with the
    /// same id, index and edges included.
    pub fn insert(&mut self, document: Document) {
        self.remove(&document.id);

        let tokens = tokenize(&format!("{} {}", document.title, document.body));
        self.lengths.insert(document.id.clone(), tokens.len());
        for token in &tokens {
            *self
                .postings
                .entry(token.clone())
                .or_default()
                .entry(document.id.clone())
                .or_insert(0) += 1;
        }

        // Typed edges as a side effect of the write — no model call, so the
        // graph cannot drift from the text it was derived from.
        let about: BTreeSet<String> = document
            .about
            .iter()
            .map(|symbol| symbol.trim().to_uppercase())
            .filter(|symbol| !symbol.is_empty())
            .collect();
        for symbol in &about {
            self.edges.push(Edge {
                document_id: document.id.clone(),
                symbol: symbol.clone(),
                kind: EdgeKind::About,
            });
        }
        for symbol in self.extract_symbols(&format!("{} {}", document.title, document.body)) {
            if !about.contains(&symbol) {
                self.edges.push(Edge {
                    document_id: document.id.clone(),
                    symbol,
                    kind: EdgeKind::Mentions,
                });
            }
        }

        self.documents.insert(document.id.clone(), document);
    }

    /// Remove a document and everything derived from it.
    pub fn remove(&mut self, id: &str) -> bool {
        let existed = self.documents.remove(id).is_some();
        if !existed {
            return false;
        }
        self.lengths.remove(id);
        self.postings.retain(|_, docs| {
            docs.remove(id);
            !docs.is_empty()
        });
        self.edges.retain(|edge| edge.document_id != id);
        true
    }

    pub fn get(&self, id: &str) -> Option<&Document> {
        self.documents.get(id)
    }

    /// Documents linked to a symbol, `About` edges first.
    pub fn about(&self, symbol: &str) -> Vec<&Document> {
        let symbol = symbol.trim().to_uppercase();
        let mut linked: Vec<(&Edge, &Document)> = self
            .edges
            .iter()
            .filter(|edge| edge.symbol == symbol)
            .filter_map(|edge| self.documents.get(&edge.document_id).map(|doc| (edge, doc)))
            .collect();
        linked.sort_by(|(a, ad), (b, bd)| {
            // About before Mentions, then newest first.
            a.kind
                .cmp(&b.kind)
                .then_with(|| bd.created_at.cmp(&ad.created_at))
        });
        let mut seen = BTreeSet::new();
        linked
            .into_iter()
            .filter(|(_, doc)| seen.insert(doc.id.clone()))
            .map(|(_, doc)| doc)
            .collect()
    }

    fn average_length(&self) -> f64 {
        if self.lengths.is_empty() {
            return 1.0;
        }
        self.lengths.values().sum::<usize>() as f64 / self.lengths.len() as f64
    }

    /// Retrieve for a query, with coverage gaps reported.
    pub fn query(&self, text: &str, limit: usize) -> Answer {
        let terms = tokenize(text);
        let query_symbols = self.extract_symbols(text);
        let n = self.documents.len() as f64;
        let avgdl = self.average_length();

        let mut scores: BTreeMap<String, (f64, Vec<String>)> = BTreeMap::new();
        let mut uncovered_terms = Vec::new();

        for term in &terms {
            let Some(posting) = self.postings.get(term) else {
                uncovered_terms.push(term.clone());
                continue;
            };
            let df = posting.len() as f64;
            // BM25 IDF, floored at zero so a term in every document contributes
            // nothing rather than pushing scores negative.
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln().max(0.0);
            for (doc_id, &tf) in posting {
                let dl = *self.lengths.get(doc_id).unwrap_or(&1) as f64;
                let tf = tf as f64;
                let contribution = idf * (tf * (K1 + 1.0)) / (tf + K1 * (1.0 - B + B * dl / avgdl));
                let entry = scores.entry(doc_id.clone()).or_insert((0.0, Vec::new()));
                entry.0 += contribution;
                entry.1.push(term.clone());
            }
        }

        // Graph expansion: documents linked to a symbol named in the query but
        // not matched lexically still carry context worth surfacing.
        let mut via_graph: BTreeSet<String> = BTreeSet::new();
        let mut uncovered_symbols = Vec::new();
        for symbol in &query_symbols {
            let linked = self.about(symbol);
            if linked.is_empty() {
                uncovered_symbols.push(symbol.clone());
                continue;
            }
            for doc in linked {
                let entry = scores.entry(doc.id.clone()).or_insert((0.0, Vec::new()));
                if entry.1.is_empty() {
                    entry.0 += GRAPH_WEIGHT;
                    via_graph.insert(doc.id.clone());
                }
            }
        }

        let mut hits: Vec<Hit> = scores
            .into_iter()
            .filter_map(|(id, (score, matched))| {
                self.documents.get(&id).map(|doc| Hit {
                    document: doc.clone(),
                    score,
                    matched_terms: matched,
                    via_graph: via_graph.contains(&id),
                })
            })
            .collect();
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.document.created_at.cmp(&a.document.created_at))
        });
        hits.truncate(limit);

        Answer {
            hits,
            uncovered_terms,
            uncovered_symbols,
            corpus_size: self.documents.len(),
        }
    }

    /// Render an answer as an evidence block for a model packet.
    ///
    /// The gap section is not optional: a packet that lists what was found but
    /// stays silent about what was missing invites the model to treat the
    /// corpus as complete.
    pub fn render(&self, answer: &Answer) -> String {
        let mut block = String::new();
        if answer.hits.is_empty() {
            block.push_str(&format!(
                "Knowledge base: no documents matched. The corpus holds {} document(s).\n",
                answer.corpus_size
            ));
        } else {
            block.push_str(&format!(
                "Knowledge base ({} of {} documents matched):\n",
                answer.hits.len(),
                answer.corpus_size
            ));
            for hit in &answer.hits {
                block.push_str(&format!(
                    "- [{:?}] {} ({}){}\n  {}\n",
                    hit.document.kind,
                    hit.document.title,
                    hit.document.created_at,
                    if hit.via_graph {
                        " — linked by instrument, not by text match"
                    } else {
                        ""
                    },
                    truncate(&hit.document.body, 400),
                ));
            }
        }
        if !answer.uncovered_terms.is_empty() || !answer.uncovered_symbols.is_empty() {
            block.push_str("\nNot covered by the knowledge base:");
            if !answer.uncovered_symbols.is_empty() {
                block.push_str(&format!(
                    "\n- No documents about: {}",
                    answer.uncovered_symbols.join(", ")
                ));
            }
            if !answer.uncovered_terms.is_empty() {
                block.push_str(&format!(
                    "\n- No document contains: {}",
                    answer.uncovered_terms.join(", ")
                ));
            }
            block.push_str("\nSay so rather than answering from adjacent material.\n");
        }
        block
    }

    /// All edges, for inspection and persistence.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Every stored document, oldest id first.
    pub fn documents(&self) -> impl Iterator<Item = &Document> {
        self.documents.values()
    }
}

fn truncate(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(id: &str, kind: DocumentKind, title: &str, body: &str, about: &[&str]) -> Document {
        Document {
            id: id.into(),
            kind,
            title: title.into(),
            body: body.into(),
            about: about.iter().map(|s| (*s).to_owned()).collect(),
            created_at: format!("2026-01-{:02}T00:00:00Z", id.len().clamp(1, 28)),
        }
    }

    fn brain() -> Brain {
        Brain::new(["AAPL".into(), "NVDA".into(), "SPY".into(), "ALL".into()])
    }

    #[test]
    fn extraction_is_bounded_by_the_vocabulary() {
        let b = brain();
        assert_eq!(b.extract_symbols("NVDA beat on datacenter"), vec!["NVDA"]);
        // Not in the vocabulary, so not a link however ticker-shaped.
        assert!(b.extract_symbols("TSLA rallied").is_empty());
    }

    #[test]
    fn ordinary_prose_cannot_manufacture_a_link() {
        // "all" is a real English word and also a real ticker. Lower-case prose
        // must not link, or every document would attach to it.
        let b = brain();
        assert!(b.extract_symbols("all of the above").is_empty());
        assert_eq!(b.extract_symbols("ALL reported earnings"), vec!["ALL"]);
    }

    #[test]
    fn insert_wires_about_and_mentions_edges() {
        let mut b = brain();
        b.insert(doc(
            "d1",
            DocumentKind::Filing,
            "AAPL 10-Q",
            "Results discussed alongside NVDA supply.",
            &["AAPL"],
        ));
        let kinds: Vec<_> = b
            .edges()
            .iter()
            .map(|e| (e.symbol.as_str(), e.kind))
            .collect();
        assert!(kinds.contains(&("AAPL", EdgeKind::About)));
        assert!(kinds.contains(&("NVDA", EdgeKind::Mentions)));
        // The subject is not also recorded as a mere mention.
        assert_eq!(
            kinds.iter().filter(|(s, _)| *s == "AAPL").count(),
            1,
            "AAPL should have exactly one edge"
        );
    }

    #[test]
    fn removing_a_document_removes_its_index_and_edges() {
        let mut b = brain();
        b.insert(doc(
            "d1",
            DocumentKind::Note,
            "NVDA thesis",
            "datacenter",
            &["NVDA"],
        ));
        assert!(b.remove("d1"));
        assert!(b.is_empty());
        assert!(b.edges().is_empty());
        assert!(b.query("datacenter", 5).hits.is_empty());
        assert!(!b.remove("d1"), "removing twice is not an error");
    }

    #[test]
    fn reinserting_the_same_id_does_not_duplicate_edges() {
        let mut b = brain();
        b.insert(doc("d1", DocumentKind::Note, "NVDA", "one", &["NVDA"]));
        b.insert(doc("d1", DocumentKind::Note, "NVDA", "two", &["NVDA"]));
        assert_eq!(b.len(), 1);
        assert_eq!(b.edges().len(), 1);
        // The index reflects the newer body, not a merge of both.
        assert!(b.query("two", 5).hits.len() == 1);
        assert!(b.query("one", 5).hits.is_empty());
    }

    #[test]
    fn a_text_match_outranks_a_graph_neighbour() {
        let mut b = brain();
        b.insert(doc(
            "match",
            DocumentKind::Note,
            "Datacenter demand",
            "datacenter capex is rising",
            &[],
        ));
        // Reached only through its About edge: the symbol appears nowhere in
        // its text, so the graph is the sole route to it.
        b.insert(doc(
            "neighbour",
            DocumentKind::Note,
            "Position sizing note",
            "unrelated prose",
            &["NVDA"],
        ));
        let answer = b.query("NVDA datacenter", 5);
        assert_eq!(answer.hits[0].document.id, "match");
        let neighbour = answer
            .hits
            .iter()
            .find(|h| h.document.id == "neighbour")
            .expect("graph expansion should surface the linked document");
        assert!(neighbour.via_graph);
        assert!(neighbour.matched_terms.is_empty());
        assert!(neighbour.score < answer.hits[0].score);
    }

    #[test]
    fn a_document_naming_the_symbol_in_its_text_is_not_a_graph_hit() {
        // It was reached by matching the query, so its provenance is a term
        // match and must be reported as such.
        let mut b = brain();
        b.insert(doc(
            "d1",
            DocumentKind::Note,
            "NVDA position",
            "prose",
            &["NVDA"],
        ));
        let answer = b.query("NVDA", 5);
        assert!(!answer.hits[0].via_graph);
        assert_eq!(answer.hits[0].matched_terms, vec!["nvda".to_string()]);
    }

    #[test]
    fn gaps_are_reported_for_terms_and_symbols_with_no_coverage() {
        let mut b = brain();
        b.insert(doc(
            "d1",
            DocumentKind::Note,
            "AAPL note",
            "margins",
            &["AAPL"],
        ));
        let answer = b.query("SPY liquidity margins", 5);
        assert!(answer.uncovered_symbols.contains(&"SPY".to_string()));
        assert!(answer.uncovered_terms.contains(&"liquidity".to_string()));
        // A covered term is not reported as a gap.
        assert!(!answer.uncovered_terms.contains(&"margins".to_string()));
    }

    #[test]
    fn an_empty_brain_answers_with_gaps_rather_than_nothing() {
        let b = brain();
        let answer = b.query("NVDA outlook", 5);
        assert!(answer.hits.is_empty());
        assert_eq!(answer.corpus_size, 0);
        let rendered = b.render(&answer);
        assert!(rendered.contains("no documents matched"));
    }

    #[test]
    fn the_rendered_block_always_states_what_is_missing() {
        let mut b = brain();
        b.insert(doc("d1", DocumentKind::Note, "AAPL", "margins", &["AAPL"]));
        let rendered = b.render(&b.query("AAPL tariffs", 5));
        assert!(rendered.contains("Not covered"));
        assert!(rendered.contains("tariffs"));
    }

    #[test]
    fn about_edges_sort_ahead_of_mentions() {
        let mut b = brain();
        b.insert(doc(
            "mention",
            DocumentKind::News,
            "Sector piece",
            "NVDA cited",
            &[],
        ));
        b.insert(doc(
            "subject",
            DocumentKind::Filing,
            "NVDA 10-K",
            "annual",
            &["NVDA"],
        ));
        let linked = b.about("NVDA");
        assert_eq!(linked[0].id, "subject");
    }

    #[test]
    fn stopwords_do_not_become_gaps() {
        let b = brain();
        let answer = b.query("what is the outlook", 5);
        assert!(!answer
            .uncovered_terms
            .iter()
            .any(|t| t == "the" || t == "is"));
        assert!(answer.uncovered_terms.contains(&"outlook".to_string()));
    }

    #[test]
    fn truncation_never_splits_a_character() {
        let long = "é".repeat(500);
        assert!(truncate(&long, 401).ends_with('…'));
    }
}
