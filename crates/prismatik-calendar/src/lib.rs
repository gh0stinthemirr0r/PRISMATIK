//! # prismatik-prismatik-calendar
//!
//! Layer 1 — Kernel extension
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — foundational calendar contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use reader::CalendarArtifactReader;
pub use session::{Interruption, Session, SessionKind};
pub use trait_def::{BarResolution, SessionCalendar, VenueCalendar};

pub mod reader;
pub mod session;
pub mod trait_def;
