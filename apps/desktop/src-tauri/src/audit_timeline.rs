//! Bounded, redacted operational timeline assembled from verified native state.

use std::collections::BTreeMap;

use prismatik_determinism::{Clock, SystemClock};
use serde::Serialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const TIMELINE_LIMIT: usize = 500;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuditEvent {
    pub(crate) id: String,
    pub(crate) occurred_at: String,
    pub(crate) domain: &'static str,
    pub(crate) severity: &'static str,
    pub(crate) state: String,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) evidence_id: Option<String>,
    pub(crate) route: &'static str,
    pub(crate) durable: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuditTimelineView {
    as_of: String,
    events: Vec<AuditEvent>,
    total_available: usize,
    source_counts: BTreeMap<String, usize>,
    redaction_posture: &'static str,
}

#[tauri::command]
pub(crate) fn get_audit_timeline() -> Result<AuditTimelineView, String> {
    let now = SystemClock::new().now();
    let mut events = Vec::new();
    events.extend(crate::evidence_store::audit_events()?);
    events.extend(crate::forecast_candidates::audit_events()?);
    events.extend(crate::paper_oms::audit_events()?);
    events.extend(crate::feed_runtime::audit_events()?);
    events.extend(crate::autonomous_research::audit_events()?);
    events.extend(crate::strategy_authoring::audit_events()?);
    events.retain(|event| parse_event_time(event).is_some_and(|time| time <= now));
    sort_events(&mut events);
    let total_available = events.len();
    let mut source_counts = BTreeMap::new();
    for event in &events {
        *source_counts.entry(event.domain.to_owned()).or_default() += 1;
    }
    events.truncate(TIMELINE_LIMIT);
    Ok(AuditTimelineView {
        as_of: now.to_string(),
        events,
        total_available,
        source_counts,
        redaction_posture: "metadata_only_no_credentials_no_raw_provider_payloads",
    })
}

fn sort_events(events: &mut [AuditEvent]) {
    events.sort_by(|a, b| {
        parse_event_time(b)
            .cmp(&parse_event_time(a))
            .then_with(|| a.id.cmp(&b.id))
    });
}

fn parse_event_time(event: &AuditEvent) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(&event.occurred_at, &Rfc3339).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, at: &str) -> AuditEvent {
        AuditEvent {
            id: id.into(),
            occurred_at: at.into(),
            domain: "test",
            severity: "info",
            state: "verified".into(),
            title: "event".into(),
            summary: "redacted".into(),
            evidence_id: None,
            route: "/workspace/journal",
            durable: true,
        }
    }

    #[test]
    fn timeline_order_is_newest_first_and_stable_on_ties() {
        let mut events = vec![
            event("b", "2026-01-01T00:00:00Z"),
            event("a", "2026-01-01T00:00:00Z"),
            event("new", "2026-01-02T00:00:00Z"),
        ];
        sort_events(&mut events);
        assert_eq!(
            events
                .iter()
                .map(|event| event.id.as_str())
                .collect::<Vec<_>>(),
            vec!["new", "a", "b"]
        );
    }
}
