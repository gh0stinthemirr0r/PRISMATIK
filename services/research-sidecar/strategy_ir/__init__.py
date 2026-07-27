"""StrategyIR research-sidecar emitter (P4-QM-15).

Python emits StrategyIR JSON; the Rust runtime executes it (Invariant I3).
"""

from .emit import (
    STRATEGY_IR_SCHEMA_VERSION,
    default_capabilities,
    emit_minimal,
    emit_strategy_ir,
)
from .validate import ValidationError, validate_strategy_ir

__all__ = [
    "STRATEGY_IR_SCHEMA_VERSION",
    "ValidationError",
    "default_capabilities",
    "emit_minimal",
    "emit_strategy_ir",
    "validate_strategy_ir",
]
