//! Wave 2 DoD #1 — hand-checked identity continuity corpus (~50 events).

use prismatik_identity::{assert_event_as_of, IdentityCorpus, IdentityEventType};

#[test]
fn identity_corpus_has_fifty_events() {
    let corpus = IdentityCorpus::load_embedded().unwrap();
    assert!(
        corpus.events().len() >= 50,
        "expected >= 50 events, got {}",
        corpus.events().len()
    );
    let mut ids: Vec<_> = corpus
        .events()
        .iter()
        .map(|event| event.id.as_str())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), corpus.events().len());
}

#[test]
fn identity_corpus_resolves_every_event_as_of() {
    let corpus = IdentityCorpus::load_embedded().unwrap();
    for event in corpus.events() {
        assert_event_as_of(corpus.mapper(), event);
    }
}

#[test]
fn facebook_to_meta_continuity_is_in_corpus() {
    let corpus = IdentityCorpus::load_embedded().unwrap();
    let event = corpus
        .events()
        .iter()
        .find(|row| {
            row.event_type == IdentityEventType::TickerChange
                && row.old_symbol.as_deref() == Some("FB")
                && row.new_symbol.as_deref() == Some("META")
        })
        .expect("FB→META must be present");
    assert_eq!(
        event.effective_date,
        time::Date::from_calendar_date(2022, time::Month::June, 9).unwrap()
    );
    assert!(!event.synthetic);
}
