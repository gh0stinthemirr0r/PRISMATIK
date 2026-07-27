//! Calendar session models.

use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

/// Session type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    /// Regular trading session.
    Regular,
    /// Pre-market session.
    PreMarket,
    /// Post-market session.
    PostMarket,
    /// Holiday/closed day marker.
    Holiday,
}

/// Intraday market interruption.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interruption {
    /// Interruption start time.
    pub start: OffsetDateTime,
    /// Interruption end time.
    pub end: OffsetDateTime,
    /// Human-readable reason.
    pub reason: String,
}

/// A single trading session from the pinned calendar artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Venue code.
    pub venue: u16,
    /// Trading date.
    pub date: Date,
    /// Session open timestamp.
    pub open: OffsetDateTime,
    /// Session close timestamp.
    pub close: OffsetDateTime,
    /// Optional breaks (e.g., lunch).
    pub breaks: Vec<(OffsetDateTime, OffsetDateTime)>,
    /// Whether this was an early close.
    pub early_close: bool,
    /// Intraday interruptions.
    pub interruptions: Vec<Interruption>,
    /// Session class.
    pub kind: SessionKind,
}

impl Session {
    /// Returns true when `at` is inside this session and not inside a break.
    pub fn is_open_at(&self, at: OffsetDateTime) -> bool {
        if at < self.open || at >= self.close || self.kind == SessionKind::Holiday {
            return false;
        }
        !self
            .breaks
            .iter()
            .any(|(start, end)| at >= *start && at < *end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_open_respects_breaks() {
        let date = Date::from_calendar_date(2026, time::Month::January, 2).expect("valid date");
        let open = OffsetDateTime::UNIX_EPOCH;
        let close = open + time::Duration::hours(8);
        let break_start = open + time::Duration::hours(2);
        let break_end = open + time::Duration::hours(3);
        let session = Session {
            venue: 1,
            date,
            open,
            close,
            breaks: vec![(break_start, break_end)],
            early_close: false,
            interruptions: Vec::new(),
            kind: SessionKind::Regular,
        };
        assert!(session.is_open_at(open + time::Duration::minutes(30)));
        assert!(!session.is_open_at(open + time::Duration::hours(2) + time::Duration::minutes(5)));
        assert!(!session.is_open_at(close));
    }
}
