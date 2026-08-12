//! The desk's knowledge base — persistence and retrieval over `prismatik-brain`.
//!
//! The brain itself is a pure in-memory structure. This owns the parts that
//! touch the world: loading documents at startup, writing them back, keeping
//! the instrument vocabulary in step with what the desk actually tracks, and
//! exposing retrieval to the agents.
//!
//! Two rules shape what goes in:
//!
//! - **Documents are things a human or a provider said**, not things PRISMATIK
//!   computed. Regime, edge and skill are recomputed deterministically on every
//!   read and would go stale the moment they were written down; they reach the
//!   agents through `quant_context` instead.
//! - **Retrieval always reports its gaps.** A packet that lists what was found
//!   and stays silent about what was missing invites a model to treat the
//!   corpus as complete.

use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use prismatik_brain::{Brain, Document, DocumentKind};
use prismatik_determinism::{Clock, ContentHash, SystemClock};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

/// Documents retained. Bounded so the file cannot grow without limit.
const MAX_DOCUMENTS: usize = 5_000;

/// Documents returned to an agent packet for one query.
const RETRIEVAL_LIMIT: usize = 6;

static BRAIN: OnceLock<Mutex<Brain>> = OnceLock::new();
static PATH: OnceLock<PathBuf> = OnceLock::new();

fn brain() -> &'static Mutex<Brain> {
    BRAIN.get_or_init(|| Mutex::new(Brain::new(Vec::new())))
}

/// Load persisted documents and rebuild the index.
pub(crate) fn initialize(data_dir: &std::path::Path) -> Result<(), String> {
    let path = data_dir.join("knowledge.json");
    let stored: Vec<Document> = if path.exists() {
        fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            // A corrupt knowledge file must not stop the desk starting. It is
            // an aid to judgement, not a ledger.
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let _ = PATH.set(path);

    let mut guard = brain().lock().map_err(|_| "knowledge base unavailable")?;
    for document in stored {
        guard.insert(document);
    }
    Ok(())
}

/// Point the vocabulary at whatever the desk currently tracks.
///
/// Extraction is exact matching, so an untracked symbol cannot be linked.
/// Refreshing on demand keeps new instruments linkable without a restart.
fn refresh_vocabulary(app: &AppHandle, guard: &mut Brain) {
    if let Ok(tracked) = crate::tracking::read_tracked(app) {
        guard.learn_symbols(tracked.into_iter().map(|row| row.symbol));
    }
}

fn persist(guard: &Brain) {
    let Some(path) = PATH.get() else { return };
    let documents: Vec<&Document> = guard.documents().collect();
    if let Ok(bytes) = serde_json::to_vec_pretty(&documents) {
        let _ = fs::write(path, bytes);
    }
}

/// A document as the UI supplies it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentDraft {
    pub(crate) kind: String,
    pub(crate) title: String,
    pub(crate) body: String,
    /// Instruments this is principally about. Mentions are extracted.
    #[serde(default)]
    pub(crate) about: Vec<String>,
}

fn parse_kind(value: &str) -> DocumentKind {
    match value.trim().to_ascii_lowercase().as_str() {
        "filing" => DocumentKind::Filing,
        "news" => DocumentKind::News,
        "research" => DocumentKind::Research,
        "decision" => DocumentKind::Decision,
        "transcript" => DocumentKind::Transcript,
        _ => DocumentKind::Note,
    }
}

/// Store a document and wire its entity edges.
#[tauri::command]
pub(crate) fn add_document(app: AppHandle, draft: DocumentDraft) -> Result<usize, String> {
    if draft.title.trim().is_empty() && draft.body.trim().is_empty() {
        return Err("a document needs a title or a body".into());
    }
    let now = SystemClock::new().now();
    let document = Document {
        // Content-addressed, so re-adding identical material replaces rather
        // than duplicates it.
        id: ContentHash::from_bytes(format!("{}|{}", draft.title, draft.body).as_bytes())
            .to_string(),
        kind: parse_kind(&draft.kind),
        title: draft.title.trim().to_owned(),
        body: draft.body.trim().to_owned(),
        about: draft
            .about
            .into_iter()
            .map(|symbol| symbol.trim().to_uppercase())
            .filter(|symbol| !symbol.is_empty())
            .collect(),
        created_at: now.to_string(),
    };

    let mut guard = brain().lock().map_err(|_| "knowledge base unavailable")?;
    refresh_vocabulary(&app, &mut guard);
    guard.insert(document);

    // Trim oldest-first when over the cap.
    if guard.len() > MAX_DOCUMENTS {
        let mut ordered: Vec<(String, String)> = guard
            .documents()
            .map(|doc| (doc.created_at.clone(), doc.id.clone()))
            .collect();
        ordered.sort();
        let excess = guard.len() - MAX_DOCUMENTS;
        for (_, id) in ordered.into_iter().take(excess) {
            guard.remove(&id);
        }
    }

    persist(&guard);
    Ok(guard.len())
}

#[tauri::command]
pub(crate) fn remove_document(id: String) -> Result<usize, String> {
    let mut guard = brain().lock().map_err(|_| "knowledge base unavailable")?;
    guard.remove(&id);
    persist(&guard);
    Ok(guard.len())
}

/// A document plus the instruments it is linked to.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentView {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) about: Vec<String>,
    pub(crate) mentions: Vec<String>,
    pub(crate) created_at: String,
}

#[tauri::command]
pub(crate) fn list_documents(symbol: Option<String>) -> Result<Vec<DocumentView>, String> {
    let guard = brain().lock().map_err(|_| "knowledge base unavailable")?;
    let wanted = symbol
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty());

    let mut rows: Vec<DocumentView> = guard
        .documents()
        .filter(|doc| match &wanted {
            None => true,
            Some(symbol) => guard
                .edges()
                .iter()
                .any(|edge| &edge.document_id == &doc.id && &edge.symbol == symbol),
        })
        .map(|doc| {
            let mentions = guard
                .edges()
                .iter()
                .filter(|edge| edge.document_id == doc.id)
                .filter(|edge| !doc.about.contains(&edge.symbol))
                .map(|edge| edge.symbol.clone())
                .collect();
            DocumentView {
                id: doc.id.clone(),
                kind: format!("{:?}", doc.kind).to_lowercase(),
                title: doc.title.clone(),
                body: doc.body.clone(),
                about: doc.about.clone(),
                mentions,
                created_at: doc.created_at.clone(),
            }
        })
        .collect();
    rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    rows.truncate(300);
    Ok(rows)
}

/// Retrieval result for the UI.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeAnswer {
    pub(crate) rendered: String,
    pub(crate) hit_count: usize,
    pub(crate) uncovered_terms: Vec<String>,
    pub(crate) uncovered_symbols: Vec<String>,
    pub(crate) corpus_size: usize,
}

#[tauri::command]
pub(crate) fn query_knowledge(app: AppHandle, query: String) -> Result<KnowledgeAnswer, String> {
    let mut guard = brain().lock().map_err(|_| "knowledge base unavailable")?;
    refresh_vocabulary(&app, &mut guard);
    let answer = guard.query(&query, RETRIEVAL_LIMIT);
    Ok(KnowledgeAnswer {
        rendered: guard.render(&answer),
        hit_count: answer.hits.len(),
        uncovered_terms: answer.uncovered_terms,
        uncovered_symbols: answer.uncovered_symbols,
        corpus_size: answer.corpus_size,
    })
}

/// Retrieval block for an agent packet, or `None` when the corpus is empty.
///
/// Returning `None` rather than an empty section matters: a model shown a
/// "Knowledge base: nothing" heading tends to read it as "nothing exists",
/// whereas an absent section is simply an absent section.
pub(crate) fn retrieval_block(app: &AppHandle, query: &str) -> Option<String> {
    let mut guard = brain().lock().ok()?;
    if guard.is_empty() {
        return None;
    }
    refresh_vocabulary(app, &mut guard);
    let answer = guard.query(query, RETRIEVAL_LIMIT);
    Some(guard.render(&answer))
}

/// Number of stored documents, for status surfaces.
pub(crate) fn corpus_size() -> usize {
    brain().lock().map(|guard| guard.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_kinds_parse_and_default_to_note() {
        assert_eq!(parse_kind("filing"), DocumentKind::Filing);
        assert_eq!(parse_kind("TRANSCRIPT"), DocumentKind::Transcript);
        assert_eq!(parse_kind(" news "), DocumentKind::News);
        // Anything unrecognised is a note rather than an error: losing an
        // operator's text over a bad label would be the worse failure.
        assert_eq!(parse_kind("scribble"), DocumentKind::Note);
        assert_eq!(parse_kind(""), DocumentKind::Note);
    }
}
