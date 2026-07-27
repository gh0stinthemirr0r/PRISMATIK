# Security Policy

PRISMATIK is developed by Mythos Systems for research and execution workflows.
Please report suspected vulnerabilities responsibly and do not disclose them in
public issues.

## Reporting a vulnerability

Send a concise report to **security@mythos.systems** with:

- affected component and version or commit;
- reproduction steps or a minimal proof of concept;
- impact assessment, especially any exposure of credentials, order execution,
  market data, or audit records; and
- any suggested mitigation.

We will acknowledge a report within five business days and coordinate a
remediation and disclosure timeline with the reporter. Do not test against
accounts, data, or systems you do not own or have explicit permission to assess.

## Scope

High-priority concerns include authentication and authorization boundaries,
secret handling, unsafe deserialization, execution controls, dependency supply
chain risks, and tampering with audit or determinism guarantees.
