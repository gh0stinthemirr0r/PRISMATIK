//! Calendar trait contracts.

use crate::Session;
use prismatik_determinism::ArtifactRef;
use time::OffsetDateTime;

/// Bar resolution for calendar boundary generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarResolution {
    /// Fixed-width time bars in seconds.
    TimeSeconds(u32),
}

/// Single-venue session calendar interface.
pub trait SessionCalendar: Send + Sync {
    /// Venue identifier.
    fn venue(&self) -> u16;
    /// Artifact backing this calendar.
    fn artifact(&self) -> &ArtifactRef;
    /// Returns true when venue is open at `at`.
    fn is_open(&self, at: OffsetDateTime) -> bool;
    /// Returns session that contains `at` if any.
    fn session_for(&self, at: OffsetDateTime) -> Option<Session>;
    /// Returns sessions overlapping `[from, to]`.
    fn sessions_between(&self, from: OffsetDateTime, to: OffsetDateTime) -> Vec<Session>;
    /// Returns bar boundaries for a session.
    fn bar_boundaries(&self, session: &Session, resolution: BarResolution) -> Vec<OffsetDateTime>;
}

/// Multi-venue calendar index.
pub trait VenueCalendar: Send + Sync {
    /// Returns a calendar by venue.
    fn calendar_for_venue(&self, venue: u16) -> Option<&dyn SessionCalendar>;
    /// Returns known venue identifiers.
    fn venues(&self) -> Vec<u16>;
}
