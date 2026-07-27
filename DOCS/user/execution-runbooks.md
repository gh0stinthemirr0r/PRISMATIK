# Execution runbooks (P7-OD-01 floor)

**Scope:** Wave 5 controlled-execution ops floor. Stubs only — expand before any live retail ship.  
**Related:** `prismatik-execution::EmergencyStop`, reconciliation quarantine path, ADR-0015 (Temporal deferred).

---

## 1. Emergency stop

**When:** Suspected runaway automation, duplicate orders, broker ambiguity storm, or operator “stop everything.”

**Steps (operator):**

1. Engage **Emergency stop** (UI or trusted-core command). Expected effects: `cancel_all`, `flatten` where configured, `halt_automation`.
2. Confirm working orders cancelled at the venue (broker console / gateway poll). Do not submit new intents.
3. Leave live gate disabled until reconciliation completes and audit shows stop engagement.
4. If stop does not halt within the stated time bound, kill the process and treat all open instruments as quarantined until manual reconcile.

**Do not:** Retry `Unknown` submissions or re-enable automation to “clear” the stop.

---

## 2. Reconciliation playbook

**When:** `SubmissionResult::Unknown`, fill/position drift, or post-disconnect restart.

**Steps:**

1. Instrument (or account) enters **quarantine** — no new intents for that scope.
2. Run `BrokerGateway::reconcile` (or continuous reconciler tick). Compare venue snapshot vs local belief.
3. Classify divergence: benign auto-heal vs irreconcilable.
4. Append resolution (or failure) to the audit ledger. Release quarantine only after a clean resolution.
5. If reconcile fails: keep quarantine, notify user, escalate as incident (below).

**Do not:** Blindly retry the original submit; that is how duplicates happen.

---

## 3. Incident stubs

| ID | Trigger | Immediate action | Escalate when |
|---|---|---|---|
| INC-ES-01 | Emergency stop engaged | Follow §1; capture audit slice | Stop incomplete or flatten mismatch |
| INC-RC-01 | Quarantine > SLA / reconcile fail | Keep quarantine; notify user | Irreconcilable cash/position delta |
| INC-UK-01 | Submit → `Unknown` | Quarantine + reconcile (§2) | Second Unknown same session |
| INC-BR-01 | Broker `Auth`/`Config` permanent | Disable account; no auto-retry | Keys rotated but still failing |
| INC-LT-01 | Live gate / step-up anomaly | Disable live; preserve ledger | Suspected privilege bypass |

Post-incident: short timeline, ledger hashes, broker order IDs, and whether automation stayed halted. Full RCA templates land with production OD, not this floor.
