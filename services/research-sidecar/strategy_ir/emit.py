"""Emit StrategyIR JSON documents (schema_version 1.0.0).

Matches crates/prismatik-strategy StrategyIr floor types (P4-QM-01).
"""

from __future__ import annotations

import json
from typing import Any, Mapping, MutableMapping, Optional, Sequence

STRATEGY_IR_SCHEMA_VERSION = "1.0.0"


def default_capabilities() -> dict[str, Any]:
    """Deny-by-default capability set (network/fs/AI off)."""
    return {
        "can_access_network": False,
        "can_access_filesystem": False,
        "can_invoke_ai": False,
        "max_compute_per_bar_ms": 50,
    }


def emit_strategy_ir(
    strategy_id: str,
    name: str,
    *,
    description: Optional[str] = None,
    capabilities: Optional[Mapping[str, Any]] = None,
    universe: Optional[Mapping[str, Any]] = None,
    indicators: Optional[Sequence[Mapping[str, Any]]] = None,
    rules: Optional[Mapping[str, Any]] = None,
) -> dict[str, Any]:
    """Build a StrategyIR document as a plain dict ready for JSON serialization."""
    caps: MutableMapping[str, Any] = dict(default_capabilities())
    if capabilities is not None:
        caps.update(capabilities)

    doc: dict[str, Any] = {
        "schema_version": STRATEGY_IR_SCHEMA_VERSION,
        "strategy_id": strategy_id,
        "name": name,
        "description": description,
        "capabilities": dict(caps),
        "universe": dict(universe)
        if universe is not None
        else {"kind": "static", "static_members": []},
        "indicators": [dict(i) for i in (indicators or ())],
        "rules": dict(rules) if rules is not None else {"entries": [], "exits": []},
    }
    return doc


def emit_minimal(strategy_id: str, name: str) -> dict[str, Any]:
    """Minimal static-universe strategy (mirrors StrategyIr::minimal)."""
    return emit_strategy_ir(strategy_id, name)


def to_json(doc: Mapping[str, Any], *, indent: int | None = 2) -> str:
    """Serialize a StrategyIR document to JSON text."""
    return json.dumps(doc, indent=indent, sort_keys=False)


if __name__ == "__main__":
    print(to_json(emit_minimal("06d2e7e0-0000-4000-8000-000000000001", "floor")))
