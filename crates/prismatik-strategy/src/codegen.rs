//! Strategy codegen contracts.

use crate::ir::{SchemaVersion, StrategyCapabilities, StrategyIR, StrategyIRError};

/// Strategy codegen input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodegenInput {
    /// Stable strategy id.
    pub id: String,
    /// Human-friendly strategy name.
    pub name: String,
    /// Source language tag (`visual`, `rust`, `python`, ...).
    pub source_kind: String,
    /// Source payload text.
    pub source_payload: String,
    /// Inferred capability flags.
    pub capabilities: StrategyCapabilities,
}

/// Strategy IR code generator.
#[derive(Debug, Default, Clone, Copy)]
pub struct Codegen;

impl Codegen {
    /// Build canonical IR from an input source document.
    pub fn emit_ir(
        &self,
        input: CodegenInput,
        schema_version: SchemaVersion,
    ) -> Result<StrategyIR, StrategyIRError> {
        let CodegenInput {
            id,
            name,
            source_kind,
            source_payload,
            capabilities,
        } = input;

        if id.trim().is_empty() {
            return Err(StrategyIRError::InvalidPayload(
                "empty strategy id".to_owned(),
            ));
        }
        if name.trim().is_empty() {
            return Err(StrategyIRError::InvalidPayload(
                "empty strategy name".to_owned(),
            ));
        }
        if source_kind.trim().is_empty() {
            return Err(StrategyIRError::InvalidPayload(
                "empty source kind".to_owned(),
            ));
        }

        let json = serde_json::json!({
            "id": id,
            "name": name,
            "source_kind": source_kind,
            "source_payload": source_payload,
            "capabilities": {
                "reads_market_data": capabilities.reads_market_data,
                "emits_orders": capabilities.emits_orders
            }
        });
        let json = serde_json::to_string(&json).map_err(|e| {
            StrategyIRError::InvalidPayload(format!("failed to encode IR JSON: {e}"))
        })?;
        let json_value: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
            StrategyIRError::InvalidPayload(format!("failed to parse IR JSON: {e}"))
        })?;
        let json = serde_json::to_string(&json_value).map_err(|e| {
            StrategyIRError::InvalidPayload(format!("failed to canonicalize IR JSON: {e}"))
        })?;

        Ok(StrategyIR {
            id,
            name,
            schema_version,
            json,
            capabilities,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_ir_with_source_envelope() {
        let codegen = Codegen;
        let ir = codegen
            .emit_ir(
                CodegenInput {
                    id: "s1".to_owned(),
                    name: "Strategy 1".to_owned(),
                    source_kind: "python".to_owned(),
                    source_payload: "print('x')".to_owned(),
                    capabilities: StrategyCapabilities {
                        reads_market_data: true,
                        emits_orders: false,
                    },
                },
                SchemaVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
            )
            .expect("codegen should succeed");
        assert_eq!(ir.id, "s1");
        assert!(!ir.capabilities.emits_orders);
        assert!(ir.json.contains("\"source_kind\":\"python\""));
    }
}
