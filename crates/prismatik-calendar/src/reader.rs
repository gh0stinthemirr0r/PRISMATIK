//! Calendar artifact reader.

use crate::{BarResolution, Session, SessionCalendar};
use prismatik_determinism::ArtifactRef;
use thiserror::Error;
use time::OffsetDateTime;

/// Errors raised while loading/using calendar artifacts.
#[derive(Debug, Error)]
pub enum CalendarError {
    /// Artifact hash/signature validation failed.
    #[error("calendar artifact validation failed: {0}")]
    Validation(String),
    /// Input was invalid.
    #[error("invalid calendar input: {0}")]
    InvalidInput(String),
}

/// In-memory reader backed by a pinned calendar artifact.
#[derive(Clone, Debug)]
pub struct CalendarArtifactReader {
    venue: u16,
    artifact: ArtifactRef,
    sessions: Vec<Session>,
}

impl CalendarArtifactReader {
    /// Build a reader from validated artifact metadata and loaded sessions.
    pub fn new(venue: u16, artifact: ArtifactRef, mut sessions: Vec<Session>) -> Self {
        sessions.sort_by_key(|s| s.open);
        Self {
            venue,
            artifact,
            sessions,
        }
    }

    /// Returns all loaded sessions.
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }
}

impl SessionCalendar for CalendarArtifactReader {
    fn venue(&self) -> u16 {
        self.venue
    }

    fn artifact(&self) -> &ArtifactRef {
        &self.artifact
    }

    fn is_open(&self, at: OffsetDateTime) -> bool {
        self.session_for(at)
            .as_ref()
            .is_some_and(|session| session.is_open_at(at))
    }

    fn session_for(&self, at: OffsetDateTime) -> Option<Session> {
        self.sessions
            .iter()
            .find(|session| session.open <= at && at < session.close)
            .cloned()
    }

    fn sessions_between(&self, from: OffsetDateTime, to: OffsetDateTime) -> Vec<Session> {
        self.sessions
            .iter()
            .filter(|session| session.close >= from && session.open <= to)
            .cloned()
            .collect()
    }

    fn bar_boundaries(&self, session: &Session, resolution: BarResolution) -> Vec<OffsetDateTime> {
        match resolution {
            BarResolution::TimeSeconds(seconds) => {
                let step = i64::from(seconds.max(1));
                let mut out = Vec::new();
                let mut cursor = session.open;
                while cursor <= session.close {
                    out.push(cursor);
                    cursor += time::Duration::seconds(step);
                }
                if out.last().copied() != Some(session.close) {
                    out.push(session.close);
                }
                out
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SessionKind;
    use prismatik_determinism::{
        ArtifactId, ArtifactKind, ContentHash, DualSignature, SemanticVersion,
    };
    use time::Date;

    #[test]
    fn reader_finds_sessions_and_boundaries() {
        let start = OffsetDateTime::UNIX_EPOCH;
        let session = Session {
            venue: 1,
            date: Date::from_calendar_date(2026, time::Month::January, 2).expect("valid date"),
            open: start,
            close: start + time::Duration::hours(1),
            breaks: Vec::new(),
            early_close: false,
            interruptions: Vec::new(),
            kind: SessionKind::Regular,
        };
        let reader = CalendarArtifactReader::new(
            1,
            ArtifactRef {
                artifact_id: ArtifactId::new("cal-test"),
                kind: ArtifactKind::Calendar,
                version: SemanticVersion::new(1, 0, 0),
                content_hash: ContentHash::from_bytes(b"calendar"),
                signature: DualSignature::default(),
            },
            vec![session.clone()],
        );
        assert!(reader.is_open(start + time::Duration::minutes(30)));
        let in_range = reader.sessions_between(start, start + time::Duration::hours(2));
        assert_eq!(in_range.len(), 1);
        let bars = reader.bar_boundaries(&session, BarResolution::TimeSeconds(900));
        assert!(bars.len() >= 5);
        assert_eq!(bars.first().copied(), Some(session.open));
        assert_eq!(bars.last().copied(), Some(session.close));
    }
}
