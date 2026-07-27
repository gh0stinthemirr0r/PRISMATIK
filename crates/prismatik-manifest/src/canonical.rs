//! Canonical JSON serialization for manifests.

use crate::types::ManifestV1;
use prismatik_determinism::ContentHash;
use serde_json::Value;

/// Serialize a JSON value with object keys sorted recursively (byte-stable).
pub fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            let mut out = String::from("{");
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).expect("key"));
                out.push(':');
                out.push_str(&canonical_json(&map[*k]));
            }
            out.push('}');
            out
        },
        Value::Array(items) => {
            let mut out = String::from("[");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&canonical_json(item));
            }
            out.push(']');
            out
        },
        other => serde_json::to_string(other).expect("json atom"),
    }
}

/// Canonical bytes of a manifest **excluding** the `signature` field.
pub fn unsigned_canonical_bytes(manifest: &ManifestV1) -> Result<Vec<u8>, serde_json::Error> {
    let mut value = serde_json::to_value(manifest)?;
    if let Value::Object(ref mut map) = value {
        map.remove("signature");
    }
    Ok(canonical_json(&value).into_bytes())
}

/// Digest of the unsigned canonical form.
pub fn unsigned_canonical_digest(manifest: &ManifestV1) -> Result<ContentHash, serde_json::Error> {
    Ok(ContentHash::from_bytes(&unsigned_canonical_bytes(
        manifest,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_json_sorts_keys() {
        let a = json!({"b": 1, "a": 2});
        let b = json!({"a": 2, "b": 1});
        assert_eq!(canonical_json(&a), canonical_json(&b));
        assert_eq!(canonical_json(&a), r#"{"a":2,"b":1}"#);
    }
}
