"""Offline StrategyIR validator — stdlib hand checks (no network, no jsonschema).

Rejects documents missing required fields such as schema_version and capabilities.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Mapping, Sequence

STRATEGY_IR_SCHEMA_VERSION = "1.0.0"

_CAPABILITY_KEYS = (
    "can_access_network",
    "can_access_filesystem",
    "can_invoke_ai",
    "max_compute_per_bar_ms",
)


class ValidationError(ValueError):
    """StrategyIR failed structural validation."""


def _require_key(doc: Mapping[str, Any], key: str) -> Any:
    if key not in doc:
        raise ValidationError(f"missing required field: {key}")
    return doc[key]


def _require_bool(obj: Mapping[str, Any], key: str, path: str) -> None:
    if key not in obj:
        raise ValidationError(f"missing required field: {path}.{key}")
    if not isinstance(obj[key], bool):
        raise ValidationError(f"{path}.{key} must be a boolean")


def validate_strategy_ir(doc: Any) -> None:
    """Validate a StrategyIR document in-memory. Raises ValidationError on failure."""
    if not isinstance(doc, Mapping):
        raise ValidationError("StrategyIR must be a JSON object")

    schema_version = _require_key(doc, "schema_version")
    if not isinstance(schema_version, str) or not schema_version:
        raise ValidationError("schema_version must be a non-empty string")
    if schema_version != STRATEGY_IR_SCHEMA_VERSION and not schema_version.startswith(
        "1.0."
    ):
        raise ValidationError(
            f"unsupported schema_version: {schema_version!r} "
            f"(expected {STRATEGY_IR_SCHEMA_VERSION!r} or 1.0.x)"
        )

    for key in ("strategy_id", "name"):
        value = _require_key(doc, key)
        if not isinstance(value, str) or not value:
            raise ValidationError(f"{key} must be a non-empty string")

    if "description" in doc and doc["description"] is not None:
        if not isinstance(doc["description"], str):
            raise ValidationError("description must be a string or null")

    caps = _require_key(doc, "capabilities")
    if not isinstance(caps, Mapping):
        raise ValidationError("capabilities must be an object")
    for key in _CAPABILITY_KEYS:
        if key == "max_compute_per_bar_ms":
            if key not in caps:
                raise ValidationError(f"missing required field: capabilities.{key}")
            ms = caps[key]
            if not isinstance(ms, int) or isinstance(ms, bool) or ms < 0:
                raise ValidationError(
                    "capabilities.max_compute_per_bar_ms must be a non-negative integer"
                )
        else:
            _require_bool(caps, key, "capabilities")

    universe = _require_key(doc, "universe")
    if not isinstance(universe, Mapping):
        raise ValidationError("universe must be an object")
    kind = universe.get("kind")
    if kind == "static":
        members = universe.get("static_members")
        if not isinstance(members, list) or not all(isinstance(m, str) for m in members):
            raise ValidationError("universe.static_members must be an array of strings")
    elif kind == "dynamic_query":
        if not isinstance(universe.get("datafusion_plan"), str):
            raise ValidationError("universe.datafusion_plan must be a string")
        if not isinstance(universe.get("rebalance_frequency"), str):
            raise ValidationError("universe.rebalance_frequency must be a string")
    else:
        raise ValidationError(
            "universe.kind must be 'static' or 'dynamic_query'"
        )

    if "indicators" in doc:
        indicators = doc["indicators"]
        if not isinstance(indicators, list):
            raise ValidationError("indicators must be an array")
        for i, ind in enumerate(indicators):
            if not isinstance(ind, Mapping):
                raise ValidationError(f"indicators[{i}] must be an object")
            for field in ("kind", "alias"):
                if not isinstance(ind.get(field), str):
                    raise ValidationError(f"indicators[{i}].{field} must be a string")
            warmup = ind.get("warmup")
            if not isinstance(warmup, int) or isinstance(warmup, bool) or warmup < 0:
                raise ValidationError(
                    f"indicators[{i}].warmup must be a non-negative integer"
                )

    if "rules" in doc and not isinstance(doc["rules"], Mapping):
        raise ValidationError("rules must be an object")


def validate_json_text(text: str) -> None:
    """Parse JSON text and validate as StrategyIR."""
    try:
        doc = json.loads(text)
    except json.JSONDecodeError as exc:
        raise ValidationError(f"invalid JSON: {exc}") from exc
    validate_strategy_ir(doc)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Validate StrategyIR JSON (offline).")
    parser.add_argument(
        "path",
        nargs="?",
        type=Path,
        help="Path to a StrategyIR JSON file (stdin if omitted)",
    )
    args = parser.parse_args(list(argv) if argv is not None else None)

    if args.path is None:
        text = sys.stdin.read()
    else:
        text = args.path.read_text(encoding="utf-8")

    try:
        validate_json_text(text)
    except ValidationError as exc:
        print(f"INVALID: {exc}", file=sys.stderr)
        return 1
    print("OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
