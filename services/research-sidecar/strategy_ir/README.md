# StrategyIR research sidecar (P4-QM-15)

Python in this sidecar **emits** StrategyIR JSON (`schema_version` `1.0.0`); the Rust deterministic runtime **executes** that IR (Invariant I3) — Python is research-mode emission only, never an in-backtest authoring interpreter.
