//! Redacted JSON wrapper for audit details.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON detail payload with pre-redacted content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RedactedJson(pub Value);

impl Default for RedactedJson {
    fn default() -> Self {
        Self(Value::Object(Default::default()))
    }
}
