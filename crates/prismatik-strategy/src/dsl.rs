//! Strategy DSL parser contracts.

use crate::ir::{SchemaVersion, StrategyCapabilities, StrategyIR, StrategyIRError};

/// Parsed strategy DSL document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrategyDsl {
    /// Stable strategy id.
    pub id: String,
    /// Human-friendly strategy name.
    pub name: String,
    /// Reads-market-data flag.
    pub reads_market_data: bool,
    /// Emits-orders flag.
    pub emits_orders: bool,
}

/// Minimal DSL parser.
///
/// Expected syntax is one key/value assignment per line:
/// `id=...`, `name=...`, `reads_market_data=true|false`, `emits_orders=true|false`.
#[derive(Debug, Default, Clone, Copy)]
pub struct DslParser;

impl DslParser {
    /// Parse DSL text into a typed document.
    pub fn parse(&self, input: &str) -> Result<StrategyDsl, StrategyIRError> {
        let mut id: Option<String> = None;
        let mut name: Option<String> = None;
        let mut reads_market_data: Option<bool> = None;
        let mut emits_orders: Option<bool> = None;

        for (line_idx, raw) in input.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=').ok_or_else(|| {
                StrategyIRError::InvalidPayload(format!("line {} is not key=value", line_idx + 1))
            })?;
            let key = key.trim();
            let value = value.trim();
            match key {
                "id" => id = Some(value.to_owned()),
                "name" => name = Some(value.to_owned()),
                "reads_market_data" => {
                    reads_market_data = Some(parse_bool(value, line_idx + 1, key)?)
                },
                "emits_orders" => emits_orders = Some(parse_bool(value, line_idx + 1, key)?),
                _ => {
                    return Err(StrategyIRError::InvalidPayload(format!(
                        "line {} has unknown key '{}'",
                        line_idx + 1,
                        key
                    )));
                },
            }
        }

        Ok(StrategyDsl {
            id: require_field("id", id)?,
            name: require_field("name", name)?,
            reads_market_data: require_field("reads_market_data", reads_market_data)?,
            emits_orders: require_field("emits_orders", emits_orders)?,
        })
    }

    /// Parse DSL text and emit canonical `StrategyIR`.
    pub fn parse_to_ir(
        &self,
        input: &str,
        schema_version: SchemaVersion,
    ) -> Result<StrategyIR, StrategyIRError> {
        let dsl = self.parse(input)?;
        let json = serde_json::json!({
            "id": dsl.id,
            "name": dsl.name,
            "reads_market_data": dsl.reads_market_data,
            "emits_orders": dsl.emits_orders
        });
        let json = serde_json::to_string(&json)
            .map_err(|e| StrategyIRError::InvalidPayload(format!("failed to serialize IR: {e}")))?;

        let value: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| StrategyIRError::InvalidPayload(format!("invalid IR JSON: {e}")))?;
        let json = serde_json::to_string(&value).map_err(|e| {
            StrategyIRError::InvalidPayload(format!("failed to canonicalize IR JSON: {e}"))
        })?;

        Ok(StrategyIR {
            id: dsl.id,
            name: dsl.name,
            schema_version,
            json,
            capabilities: StrategyCapabilities {
                reads_market_data: dsl.reads_market_data,
                emits_orders: dsl.emits_orders,
            },
        })
    }
}

fn require_field<T>(name: &str, value: Option<T>) -> Result<T, StrategyIRError> {
    value.ok_or_else(|| StrategyIRError::InvalidPayload(format!("missing required field '{name}'")))
}

fn parse_bool(value: &str, line: usize, key: &str) -> Result<bool, StrategyIRError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(StrategyIRError::InvalidPayload(format!(
            "line {line} field '{key}' must be true|false"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_dsl_to_ir() {
        let parser = DslParser;
        let ir = parser
            .parse_to_ir(
                "id=mean-revert-1\nname=Mean Reversion\nreads_market_data=true\nemits_orders=true",
                SchemaVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
            )
            .expect("dsl should parse");
        assert_eq!(ir.id, "mean-revert-1");
        assert!(ir.capabilities.reads_market_data);
        assert!(ir.capabilities.emits_orders);
        assert!(ir.json.contains("\"id\":\"mean-revert-1\""));
    }

    #[test]
    fn rejects_invalid_bool() {
        let parser = DslParser;
        let err = parser
            .parse("id=x\nname=y\nreads_market_data=yes\nemits_orders=false")
            .expect_err("invalid bool should fail");
        let message = err.to_string();
        assert!(message.contains("true|false"));
    }
}
