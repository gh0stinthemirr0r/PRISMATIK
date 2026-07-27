//! Persisted first-run onboarding state.

use prismatik_storage::{PreferenceStore, RepositoryError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const FIRST_RUN_KEY: &str = "first_run_state";

/// Onboarding progression.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstRunState {
    /// Initial introduction.
    #[default]
    Welcome,
    /// Suitability and risk acknowledgement.
    Suitability,
    /// Choose deterministic demo data or live providers.
    DemoOrLive,
    /// Onboarding is finished.
    Complete,
}

/// First-run persistence errors.
#[derive(Debug, Error)]
pub enum FirstRunError {
    /// Preference storage failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// Stored preference was malformed.
    #[error("invalid first-run state: {0}")]
    Decode(#[from] serde_json::Error),
}

impl FirstRunState {
    /// Load state, defaulting to `Welcome` when no preference exists.
    pub fn load(store: &impl PreferenceStore) -> Result<Self, FirstRunError> {
        match store.get_pref(FIRST_RUN_KEY)? {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Ok(Self::Welcome),
        }
    }

    /// Persist this state.
    pub fn save(self, store: &impl PreferenceStore) -> Result<(), FirstRunError> {
        store.set_pref(FIRST_RUN_KEY, &serde_json::to_string(&self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_storage::SqliteBackend;

    #[test]
    fn first_run_state_round_trips_preferences() {
        let db = SqliteBackend::open_in_memory().unwrap();
        assert_eq!(FirstRunState::load(&db).unwrap(), FirstRunState::Welcome);
        FirstRunState::DemoOrLive.save(&db).unwrap();
        assert_eq!(FirstRunState::load(&db).unwrap(), FirstRunState::DemoOrLive);
    }
}
