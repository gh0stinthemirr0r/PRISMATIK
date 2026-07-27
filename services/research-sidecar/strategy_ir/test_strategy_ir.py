"""Offline tests for StrategyIR emit + validate (P4-QM-15).

Run from repo root:
  python -m pytest services/research-sidecar/strategy_ir/test_strategy_ir.py -q

Or without pytest:
  python services/research-sidecar/strategy_ir/test_strategy_ir.py
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

# Allow `python path/to/test_strategy_ir.py` without installing the package.
_HERE = Path(__file__).resolve().parent
if str(_HERE.parent) not in sys.path:
    sys.path.insert(0, str(_HERE.parent))

from strategy_ir.emit import (  # noqa: E402
    STRATEGY_IR_SCHEMA_VERSION,
    emit_minimal,
    emit_strategy_ir,
)
from strategy_ir.validate import ValidationError, validate_strategy_ir  # noqa: E402

SCHEMA_PATH = _HERE / "strategy_ir.schema.json"


class StrategyIrEmitTests(unittest.TestCase):
    def test_emit_minimal_schema_version_1_0_0(self) -> None:
        doc = emit_minimal("06d2e7e0-0000-4000-8000-000000000001", "floor")
        self.assertEqual(doc["schema_version"], STRATEGY_IR_SCHEMA_VERSION)
        self.assertEqual(doc["schema_version"], "1.0.0")
        self.assertIn("capabilities", doc)
        self.assertFalse(doc["capabilities"]["can_access_network"])
        validate_strategy_ir(doc)

    def test_emit_round_trips_json(self) -> None:
        doc = emit_strategy_ir(
            "dyn",
            "dynamic",
            description="prep",
            universe={
                "kind": "dynamic_query",
                "datafusion_plan": "cGxhbg==",
                "rebalance_frequency": "1d",
            },
            indicators=[{"kind": "sma", "alias": "sma_20", "warmup": 20}],
        )
        back = json.loads(json.dumps(doc))
        validate_strategy_ir(back)
        self.assertEqual(back["schema_version"], "1.0.0")


FIXTURE_PATH = (
    _HERE.parents[2]
    / "crates"
    / "prismatik-strategy"
    / "tests"
    / "fixtures"
    / "momentum_sma20_strategy_ir.json"
)


class StrategyIrDod10ParityTests(unittest.TestCase):
    """Wave 3 DoD #10 floor — Python emitter matches committed golden fixture."""

    def test_emit_matches_momentum_golden_structurally(self) -> None:
        emitted = emit_strategy_ir(
            "06d2e7e0-0000-4000-8000-000000000010",
            "Momentum",
            indicators=[{"kind": "sma", "alias": "sma_20", "warmup": 19}],
        )
        validate_strategy_ir(emitted)
        self.assertTrue(FIXTURE_PATH.is_file(), f"missing golden {FIXTURE_PATH}")
        golden = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
        validate_strategy_ir(golden)

        for key in ("name", "schema_version", "capabilities", "universe", "indicators"):
            self.assertEqual(
                emitted[key],
                golden[key],
                f"canonical field `{key}` must match golden fixture",
            )
        self.assertEqual(emitted["name"], "Momentum")
        self.assertEqual(emitted["schema_version"], "1.0.0")
        self.assertFalse(emitted["capabilities"]["can_access_network"])
        self.assertEqual(emitted["universe"]["kind"], "static")
        self.assertEqual(emitted["indicators"][0]["kind"], "sma")
        self.assertEqual(emitted["indicators"][0]["alias"], "sma_20")


class StrategyIrValidateNegativeTests(unittest.TestCase):
    def test_rejects_missing_schema_version(self) -> None:
        doc = emit_minimal("id", "name")
        del doc["schema_version"]
        with self.assertRaises(ValidationError) as ctx:
            validate_strategy_ir(doc)
        self.assertIn("schema_version", str(ctx.exception))

    def test_rejects_missing_capabilities(self) -> None:
        doc = emit_minimal("id", "name")
        del doc["capabilities"]
        with self.assertRaises(ValidationError) as ctx:
            validate_strategy_ir(doc)
        self.assertIn("capabilities", str(ctx.exception))

    def test_rejects_incomplete_capabilities(self) -> None:
        doc = emit_minimal("id", "name")
        del doc["capabilities"]["can_access_network"]
        with self.assertRaises(ValidationError) as ctx:
            validate_strategy_ir(doc)
        self.assertIn("capabilities.can_access_network", str(ctx.exception))

    def test_schema_file_declares_required_fields(self) -> None:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        required = set(schema["required"])
        self.assertIn("schema_version", required)
        self.assertIn("capabilities", required)
        self.assertEqual(schema["properties"]["schema_version"]["const"], "1.0.0")


if __name__ == "__main__":
    unittest.main()
