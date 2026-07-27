//! Currency code wrapper.

use serde::{Deserialize, Serialize};

/// ISO-style currency code.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CurrencyCode(String);

impl CurrencyCode {
    /// Create currency code in uppercase form.
    pub fn new(code: impl AsRef<str>) -> Self {
        Self(code.as_ref().trim().to_ascii_uppercase())
    }

    /// Return code string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
