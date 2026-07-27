//! Hand-checked identity corpus for Wave 2 DoD #1.
//!
//! Fixture: `cassettes/identity_corpus/events_2015_2026.json`
//! (~50 ticker-continuity events spanning 2015–2026).

use crate::openfigi::OpenFigiMapper;
use crate::resolver::SymbologyError;
use crate::{AssetId, ExternalIdentifier, MicCode, SymbologyResolver};
use serde::Deserialize;
use time::{Date, Duration, OffsetDateTime, Time};

const CORPUS_JSON: &str = include_str!("../cassettes/identity_corpus/events_2015_2026.json");

/// Loaded DoD #1 identity corpus (events + OpenFIGI cassette bindings).
#[derive(Debug)]
pub struct IdentityCorpus {
    events: Vec<IdentityEvent>,
    mapper: OpenFigiMapper,
}

impl IdentityCorpus {
    /// Parse the embedded 2015–2026 hand-checked corpus.
    pub fn load_embedded() -> Result<Self, SymbologyError> {
        Self::from_json(CORPUS_JSON)
    }

    /// Parse a corpus document (mappings + transitions + events).
    pub fn from_json(json: &str) -> Result<Self, SymbologyError> {
        let doc: CorpusDocument = serde_json::from_str(json)
            .map_err(|error| SymbologyError::Storage(error.to_string()))?;
        if doc.events.is_empty() {
            return Err(SymbologyError::Storage(
                "identity corpus must contain at least one event".into(),
            ));
        }
        // Re-serialize the cassette slice so OpenFigiMapper stays the single loader.
        let cassette = serde_json::json!({
            "mappings": doc.mappings,
            "transitions": doc.transitions,
        });
        let mapper = OpenFigiMapper::from_json_cassette(&cassette.to_string())?;
        Ok(Self {
            events: doc.events,
            mapper,
        })
    }

    /// Hand-checked events in fixture order.
    pub fn events(&self) -> &[IdentityEvent] {
        &self.events
    }

    /// Bitemporal mapper backed by the corpus cassette rows.
    pub fn mapper(&self) -> &OpenFigiMapper {
        &self.mapper
    }

    /// Number of synthetic-FIGI assets in the cassette.
    pub fn synthetic_mapping_count(&self) -> usize {
        // Count via events marked synthetic that introduce FIGI rows; callers
        // that need precision should inspect the fixture. Tests assert below.
        self.events.iter().filter(|event| event.synthetic).count()
    }
}

/// One hand-checked continuity event.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct IdentityEvent {
    /// Stable fixture id.
    pub id: String,
    /// Continuity class.
    pub event_type: IdentityEventType,
    /// Calendar effective date (UTC midnight used for `as_of` checks).
    #[serde(deserialize_with = "deserialize_date")]
    pub effective_date: Date,
    /// Primary MIC for symbols in this event.
    pub mic: String,
    /// Optional MIC for the pre-event symbol when venue changes.
    #[serde(default)]
    pub old_mic: Option<String>,
    /// Optional MIC for a spin-off parent when it differs from `mic`.
    #[serde(default)]
    pub parent_mic: Option<String>,
    /// Optional MIC for a merger survivor when it differs from `mic`.
    #[serde(default)]
    pub surviving_mic: Option<String>,
    /// Pre-event ticker (renames / delists).
    #[serde(default)]
    pub old_symbol: Option<String>,
    /// Post-event ticker (renames).
    #[serde(default)]
    pub new_symbol: Option<String>,
    /// Single symbol (splits / class-share primary).
    #[serde(default)]
    pub symbol: Option<String>,
    /// Peer class-share ticker.
    #[serde(default)]
    pub peer_symbol: Option<String>,
    /// Surviving ticker after a merger delist.
    #[serde(default)]
    pub surviving_symbol: Option<String>,
    /// Spin-off parent ticker.
    #[serde(default)]
    pub parent_symbol: Option<String>,
    /// Spin-off child ticker.
    #[serde(default)]
    pub child_symbol: Option<String>,
    /// Canonical key for the primary asset.
    pub canonical_key: String,
    /// Peer / surviving / child canonical keys when applicable.
    #[serde(default)]
    pub peer_canonical_key: Option<String>,
    /// Surviving entity after a merger delist.
    #[serde(default)]
    pub surviving_canonical_key: Option<String>,
    /// Spun-off child entity.
    #[serde(default)]
    pub child_canonical_key: Option<String>,
    /// FIGI for the primary asset (may be synthetic).
    pub figi: String,
    /// Peer class-share FIGI when applicable.
    #[serde(default)]
    pub peer_figi: Option<String>,
    /// Split ratio text (informational).
    #[serde(default)]
    pub ratio: Option<String>,
    /// Human notes.
    pub notes: String,
    /// Provenance pointer.
    pub source: String,
    /// True when FIGI/cassette rows are synthetic placeholders.
    #[serde(default)]
    pub synthetic: bool,
}

/// Continuity event classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityEventType {
    /// Ticker rename with preserved canonical identity.
    TickerChange,
    /// Forward split; identity unchanged.
    Split,
    /// Distinct share classes at the same issuer.
    ClassShare,
    /// Acquiree ticker ceases; optional survivor continues.
    MergerDelist,
    /// Parent continues; child identity appears.
    Spinoff,
}

#[derive(Debug, Deserialize)]
struct CorpusDocument {
    mappings: serde_json::Value,
    #[serde(default)]
    transitions: serde_json::Value,
    events: Vec<IdentityEvent>,
}

fn deserialize_date<'de, D>(deserializer: D) -> Result<Date, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    let format = time::format_description::parse_borrowed::<2>("[year]-[month]-[day]")
        .map_err(serde::de::Error::custom)?;
    Date::parse(&raw, &format).map_err(serde::de::Error::custom)
}

fn utc_midnight(date: Date) -> OffsetDateTime {
    date.with_time(Time::MIDNIGHT).assume_utc()
}

fn day_before(date: Date) -> Date {
    date - Duration::days(1)
}

fn ticker(symbol: &str, mic: &str) -> ExternalIdentifier {
    ExternalIdentifier::TickerAtVenue {
        ticker: symbol.into(),
        mic: MicCode::from_str_unchecked(mic),
    }
}

fn expect_asset(key: &str) -> AssetId {
    AssetId::from_canonical_bytes(key.as_bytes())
}

fn assert_resolve(
    mapper: &OpenFigiMapper,
    id: &ExternalIdentifier,
    as_of: OffsetDateTime,
    expected: Option<AssetId>,
    context: &str,
) {
    let got = mapper
        .resolve_as_of(id, as_of)
        .unwrap_or_else(|error| panic!("{context}: resolve error: {error}"));
    assert_eq!(got, expected, "{context}");
}

/// Verify one corpus event against the bitemporal resolver.
pub fn assert_event_as_of(mapper: &OpenFigiMapper, event: &IdentityEvent) {
    let effective = utc_midnight(event.effective_date);
    let before = utc_midnight(day_before(event.effective_date));
    let primary = expect_asset(&event.canonical_key);
    let mic = event.mic.as_str();
    let old_mic = event.old_mic.as_deref().unwrap_or(mic);
    let parent_mic = event.parent_mic.as_deref().unwrap_or(mic);

    match event.event_type {
        IdentityEventType::TickerChange => {
            let old = event
                .old_symbol
                .as_deref()
                .unwrap_or_else(|| panic!("{}: missing old_symbol", event.id));
            let new = event
                .new_symbol
                .as_deref()
                .unwrap_or_else(|| panic!("{}: missing new_symbol", event.id));
            let old_id = ticker(old, old_mic);
            let new_id = ticker(new, mic);
            let figi = ExternalIdentifier::Figi(event.figi.clone());

            assert_resolve(
                mapper,
                &old_id,
                before,
                Some(primary),
                &format!("{}: old ticker before", event.id),
            );
            assert_resolve(
                mapper,
                &old_id,
                effective,
                None,
                &format!("{}: old ticker on/after", event.id),
            );
            assert_resolve(
                mapper,
                &new_id,
                before,
                None,
                &format!("{}: new ticker before", event.id),
            );
            assert_resolve(
                mapper,
                &new_id,
                effective,
                Some(primary),
                &format!("{}: new ticker on/after", event.id),
            );
            assert_resolve(
                mapper,
                &figi,
                before,
                Some(primary),
                &format!("{}: figi before", event.id),
            );
            assert_resolve(
                mapper,
                &figi,
                effective,
                Some(primary),
                &format!("{}: figi after", event.id),
            );

            let chain = mapper
                .identity_chain(&primary)
                .unwrap_or_else(|error| panic!("{}: chain error: {error}", event.id));
            assert!(
                chain.iter().any(|step| step.effective_at == effective),
                "{}: identity chain missing transition at {effective}",
                event.id
            );
        },
        IdentityEventType::Split => {
            let symbol = event
                .symbol
                .as_deref()
                .unwrap_or_else(|| panic!("{}: missing symbol", event.id));
            let id = ticker(symbol, mic);
            assert_resolve(
                mapper,
                &id,
                before,
                Some(primary),
                &format!("{}: before split", event.id),
            );
            assert_resolve(
                mapper,
                &id,
                effective,
                Some(primary),
                &format!("{}: after split", event.id),
            );
        },
        IdentityEventType::ClassShare => {
            let symbol = event
                .symbol
                .as_deref()
                .unwrap_or_else(|| panic!("{}: missing symbol", event.id));
            let peer = event
                .peer_symbol
                .as_deref()
                .unwrap_or_else(|| panic!("{}: missing peer_symbol", event.id));
            let peer_key = event
                .peer_canonical_key
                .as_deref()
                .unwrap_or_else(|| panic!("{}: missing peer_canonical_key", event.id));
            let peer_asset = expect_asset(peer_key);
            assert_ne!(
                primary, peer_asset,
                "{}: class shares must be distinct assets",
                event.id
            );
            assert_resolve(
                mapper,
                &ticker(symbol, mic),
                effective,
                Some(primary),
                &format!("{}: class A/primary", event.id),
            );
            assert_resolve(
                mapper,
                &ticker(peer, mic),
                effective,
                Some(peer_asset),
                &format!("{}: class B/peer", event.id),
            );
        },
        IdentityEventType::MergerDelist => {
            let old = event
                .old_symbol
                .as_deref()
                .unwrap_or_else(|| panic!("{}: missing old_symbol", event.id));
            let old_id = ticker(old, mic);
            assert_resolve(
                mapper,
                &old_id,
                before,
                Some(primary),
                &format!("{}: acquiree before", event.id),
            );
            assert_resolve(
                mapper,
                &old_id,
                effective,
                None,
                &format!("{}: acquiree on/after", event.id),
            );
            if let (Some(surv_sym), Some(surv_key)) = (
                event.surviving_symbol.as_deref(),
                event.surviving_canonical_key.as_deref(),
            ) {
                let surv_mic = event.surviving_mic.as_deref().unwrap_or(mic);
                let survivor = expect_asset(surv_key);
                assert_resolve(
                    mapper,
                    &ticker(surv_sym, surv_mic),
                    effective,
                    Some(survivor),
                    &format!("{}: survivor on/after", event.id),
                );
            }
        },
        IdentityEventType::Spinoff => {
            let parent = event
                .parent_symbol
                .as_deref()
                .unwrap_or_else(|| panic!("{}: missing parent_symbol", event.id));
            let child = event
                .child_symbol
                .as_deref()
                .unwrap_or_else(|| panic!("{}: missing child_symbol", event.id));
            let child_key = event
                .child_canonical_key
                .as_deref()
                .unwrap_or_else(|| panic!("{}: missing child_canonical_key", event.id));
            let child_asset = expect_asset(child_key);
            let parent_id = ticker(parent, parent_mic);
            let child_id = ticker(child, mic);

            // Child must not resolve before the spin; must resolve on/after.
            assert_resolve(
                mapper,
                &child_id,
                before,
                None,
                &format!("{}: child before", event.id),
            );
            assert_resolve(
                mapper,
                &child_id,
                effective,
                Some(child_asset),
                &format!("{}: child on/after", event.id),
            );

            // Parent resolves the day before. On the effective date the parent
            // may continue (typical spin) or end (UTX concurrent spins).
            assert_resolve(
                mapper,
                &parent_id,
                before,
                Some(primary),
                &format!("{}: parent before", event.id),
            );
            let parent_after = mapper
                .resolve_as_of(&parent_id, effective)
                .unwrap_or_else(|error| panic!("{}: parent after error: {error}", event.id));
            assert!(
                parent_after == Some(primary) || parent_after.is_none(),
                "{}: parent after must continue or delist, got {parent_after:?}",
                event.id
            );
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_loads_at_least_fifty_events() {
        let corpus = IdentityCorpus::load_embedded().unwrap();
        assert!(
            corpus.events().len() >= 50,
            "expected >= 50 events, got {}",
            corpus.events().len()
        );
        assert!(!corpus.mapper().is_empty());
    }

    #[test]
    fn every_corpus_event_resolves_as_of() {
        let corpus = IdentityCorpus::load_embedded().unwrap();
        for event in corpus.events() {
            assert_event_as_of(corpus.mapper(), event);
        }
    }

    #[test]
    fn fb_meta_present_and_synthetic_events_flagged() {
        let corpus = IdentityCorpus::load_embedded().unwrap();
        let fb = corpus
            .events()
            .iter()
            .find(|event| event.id == "fb-meta-2022")
            .expect("FB→META must be in the corpus");
        assert!(!fb.synthetic);
        assert_eq!(fb.old_symbol.as_deref(), Some("FB"));
        assert_eq!(fb.new_symbol.as_deref(), Some("META"));

        let synthetic = corpus.events().iter().filter(|e| e.synthetic).count();
        assert!(
            synthetic > 0,
            "corpus should mark events that needed synthetic FIGI rows"
        );
    }

    #[test]
    fn corpus_covers_required_event_kinds() {
        let corpus = IdentityCorpus::load_embedded().unwrap();
        let mut saw_rename = false;
        let mut saw_split = false;
        let mut saw_class = false;
        let mut saw_merger = false;
        let mut saw_spin = false;
        for event in corpus.events() {
            match event.event_type {
                IdentityEventType::TickerChange => saw_rename = true,
                IdentityEventType::Split => saw_split = true,
                IdentityEventType::ClassShare => saw_class = true,
                IdentityEventType::MergerDelist => saw_merger = true,
                IdentityEventType::Spinoff => saw_spin = true,
            }
        }
        assert!(saw_rename && saw_split && saw_class && saw_merger && saw_spin);
    }

    #[test]
    fn effective_dates_span_2015_to_2026() {
        let corpus = IdentityCorpus::load_embedded().unwrap();
        let min = corpus
            .events()
            .iter()
            .map(|e| e.effective_date)
            .min()
            .unwrap();
        let max = corpus
            .events()
            .iter()
            .map(|e| e.effective_date)
            .max()
            .unwrap();
        assert!(min <= Date::from_calendar_date(2016, time::Month::December, 31).unwrap());
        assert!(max >= Date::from_calendar_date(2025, time::Month::January, 1).unwrap());
    }
}
