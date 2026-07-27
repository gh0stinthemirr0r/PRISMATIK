# Wave 5 Gate — Turbo Floor vs Author-Ops

**Policy:** [`TURBO_GATE_POLICY.md`](../waves/TURBO_GATE_POLICY.md) · **Workbook:** [`Wave_5_Workbook.md`](../waves/Wave_5_Workbook.md)  
**Verdict:** **OPEN (turbo floors).** No claim of production live trading.

## Exit criteria / work items

| ID / Criterion | Class | Status |
|---|---|---|
| P7-QM-01..06 BrokerGateway / idempotency / paper | turbo-floor | floor |
| P7-SS-01 step-up | turbo-floor | floor |
| P7-SS-02 EmergencyStop | turbo-floor | floor |
| P7-OD-01 runbook stubs | turbo-floor | floor (`DOCS/user/execution-runbooks.md`) |
| Fill P0-DK-10 Machine B before signed public binary | author-ops residual | open (resolve at RC) |
| Prod cosign / SLSA L3 | author-ops residual | open |
| P7-EX-01 order ticket UI | author-ops residual | author/EX |
| Live trading readiness | author-ops residual | not claimed under turbo |
