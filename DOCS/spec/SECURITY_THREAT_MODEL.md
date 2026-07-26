# PRISMATIK Security Threat Model

**Document:** `spec/SECURITY_THREAT_MODEL.md`
**Status:** NORMATIVE — RFC 2119 keywords apply
**Companion to:** `PRISMATIK_Unified_Solution_Architecture_v1.0.md` §24 (security), §20.4 (plugin host), §14 (audit ledger)
**Date:** 2026-07-26

---

## 0. Purpose

This document specifies the **enumerated threat model (STRIDE per trust boundary), attack surface inventory, control mapping, and adversarial eval cadence** for PRISMATIK. Security is mentioned throughout the architecture; this document makes it concrete and testable.

The cardinal rule (v1.0 §9.1): **a process that can hold a credential cannot execute third-party code, and a process that executes third-party code cannot hold a credential.** Every box in the system topology satisfies this.

---

## 1. Trust Boundaries

PRISMATIK has six trust boundaries. Each is a transition where the trust level changes and where attacks MUST be considered.

```
┌─────────────────────────────────────────────────────────────────────┐
│  UNTRUSTED EXTERNAL                                                 │
│  (Internet, providers, hosted LLM APIs, broker APIs, MCP servers)   │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ ① Provider Network Boundary
┌──────────────────────────────▼──────────────────────────────────────┐
│  SEMI-TRUSTED: Ingestion Worker                                     │
│  Parses hostile external input. Crash isolation.                    │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ ② Ingestion-to-Core Boundary
┌──────────────────────────────▼──────────────────────────────────────┐
│  TRUSTED CORE (Rust)                                                │
│  Owns keys, policy, persistence, Determinism Kernel, audit ledger.  │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  SANDBOXED: Plugin Host (wasmtime, capability-scoped)        │   │
│  └──────────────────────────────────────────────────────────────┘   │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ ③ IPC Boundary (Tauri command/event)
┌──────────────────────────────▼──────────────────────────────────────┐
│  UNTRUSTED: WebView (Svelte)                                        │
│  Renders untrusted content. Holds no secrets. Compromisable.        │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ ④ IPC Boundary (sidecar boundary)
┌──────────────────────────────▼──────────────────────────────────────┐
│  UNTRUSTED: Sidecars (CCXT, QuantLib, MAPIE, LM Studio)             │
│  Third-party code. Output treated as provider data.                 │
└─────────────────────────────────────────────────────────────────────┘

                               ⑤ Storage Boundary (disk)
                               ⑥ Memory Boundary (process isolation)
```

---

## 2. STRIDE per Trust Boundary

### 2.1 ① Provider Network Boundary

| Threat | Example | Control |
|---|---|---|
| **Spoofing** | Fake provider endpoint (MITM) | TLS cert validation; provider allowlist |
| **Tampering** | Provider returns malicious payload to inject data | Ingestion parser isolation; output validated against schema |
| **Repudiation** | Provider denies having sent data | Raw layer is append-only with `evidence_hash` and `retrieved_at`; non-repudiable |
| **Information disclosure** | Provider API key leaked | Keys in OS keychain, never in env beyond moment of use; `redact_secrets` on all logs |
| **Denial of service** | Provider rate-limits us | GCRA governor; degrade gracefully |
| **Elevation of privilege** | n/a (we're the client) | n/a |

### 2.2 ② Ingestion-to-Core Boundary

| Threat | Example | Control |
|---|---|---|
| **Tampering** | Malformed provider payload crashes parser | Ingestion worker is a separate process; crash isolation |
| **Information disclosure** | Parser writes secret data to disk | Secrets never enter the data plane; redaction by type |
| **DoS** | Pathological input hangs parser | Fuel metering / timeouts on parsing |

### 2.3 ③ IPC Boundary (Trusted Core ↔ WebView)

**This is the highest-risk boundary.** The WebView renders untrusted content (chart data, news, AI responses) and can be compromised via XSS or Tauri IPC bypass.

| Threat | Example | Control |
|---|---|---|
| **Spoofing** | Compromised WebView impersonates a legitimate caller | Bearer token per session; injected into WebView |
| **Tampering** | Compromised WebView sends malformed command to alter state | Every command validates input; capability tier enforced |
| **Repudiation** | Malicious action disclaimed | Every state-changing command audited in the ledger |
| **Information disclosure** | Compromised WebView exfiltrates data via IPC | No command returns raw secrets; sensitive fields redacted; CSP strict (no inline/eval); **Tauri ≥2.12 per CVE-2026-42184** |
| **DoS** | Compromised WebView floods IPC | Per-stream rate caps; backpressure on streaming |
| **Elevation of privilege** | Compromised WebView calls StepUp command | Step-up requires passkey re-auth (5 min TTL); biometric/hardware-backed |

**The CVE-2026-42184 control is critical.** Tauri 2.0–2.11 has a flaw in `is_local_url()` causing remote URLs to be classified as trusted local origins on Windows/Android → authentication bypass of the IPC trust boundary. Wave 0 DoD criterion 13 mandates ≥2.12. The IPC boundary MUST be treated as untrusted with defense-in-depth even post-patch.

### 2.4 ④ Sidecar Boundary

| Threat | Example | Control |
|---|---|---|
| **Tampering** | Compromised sidecar returns malicious output | Output validated as provider data; never authoritative; provenance-stamped |
| **Information disclosure** | Sidecar exfiltrates credentials | Sidecars hold NO credentials; communicate over typed, schema-validated local socket |
| **Elevation of privilege** | Sidecar attempts to write to data plane | Sidecars CANNOT write to data plane by construction (no DB access) |

### 2.5 ⑤ Storage Boundary (Disk)

| Threat | Example | Control |
|---|---|---|
| **Tampering** | Attacker modifies audit ledger on disk | Merkle hash chain detects retroactive edits (Wave 0 DoD 7); WORM mirror or external anchor recommended (Wave 6+) |
| **Tampering** | Attacker modifies calendar/codebook artifact | Hash + signature verification on every load; mismatch = security event |
| **Information disclosure** | Attacker reads credentials at rest | OS keychain only (never files); sealed credential envelope (AES-GCM) |
| **Information disclosure** | Attacker reads user's positions/journal | Disk encryption (rely on OS: FileVault/BitLocker/LUKS); user's responsibility for physical security |

### 2.6 ⑥ Memory Boundary (Process Isolation)

| Threat | Example | Control |
|---|---|---|
| **Elevation of privilege** | WASM plugin escapes sandbox | Cranelift-only (no Winch per RUSTSEC-2026-0095); signals-based-traps on; guard pages on; fuel + epoch interruption; no relaxed SIMD |
| **Information disclosure** | Process memory dump exposes keys | `zeroize` on drop; `secrecy` wrappers; secrets not logged |

---

## 3. Attack Surface Inventory

### 3.1 Network-Facing

- **Inbound:** NONE. The desktop profile binds to loopback ONLY (`127.0.0.1:8787`). No external network listeners.
- **Outbound (with credentials):** provider APIs, broker APIs, hosted LLM APIs.
- **Outbound (no credentials):** public market data, public filings.

**The desktop profile has no inbound attack surface.** The Team Cloud and Enterprise profiles add inbound (Postgres, NATS, OIDC) — those are Wave 6 concerns with their own threat model.

### 3.2 IPC Surface

Every Tauri command is an attack surface entry. See `spec/IPC_CONTRACTS.md` for the complete catalog. Each command MUST:
- Validate inputs deterministically.
- Enforce capability tier.
- Audit state-changing operations.

### 3.3 Parser Surface

External data parsing is high-risk. Parsers handle:
- Provider JSON responses (CoinGecko, Alpaca, Unusual Whales, etc.)
- SEC filings (XBRL, HTML)
- Model artifacts (Parquet, ONNX, GGUF)
- Plugin modules (WASM)

Each parser runs in isolation (separate process for ingestion worker; sandbox for plugins). A parser crash is recoverable; a parser compromise is contained.

### 3.4 Cryptographic Surface

- AES-GCM envelope encryption (`prismatik-security`).
- BLAKE3 hashing (`prismatik-determinism`).
- Ed25519 + ML-DSA-65 dual signatures (`prismatik-manifest`).
- OS keychain integration.

Cryptographic primitives MUST come from audited crates (`rustcrypto` family). No hand-rolled crypto.

### 3.5 AI Plane Surface

- LLM-generated content reaches the UI (AI tool calls).
- LLM tool-calling boundary (per OWASP ASI01/ASI02).
- Hosted LLM API sees user payload (T3 escalation consent surface).

---

## 4. Control Mapping

### 4.1 Per-Invariant Controls

| Invariant | Control |
|---|---|
| **I1** Evidence Precedes Conclusion | Type system; `Concludes` trait requires non-empty evidence; `test_no_orphan_conclusions` |
| **I2** Probability, Never Prophecy | Type system; `ModelPrediction` has no public constructor without `CalibrationRecord` |
| **I3** Determinism Under Seed | Determinism Kernel; madsim DST; golden corpus |
| **I4** Structural Capability Enforcement | WASM host capability scoping; capability-class tool taxonomy; `RiskApprovedOrderIntent` unconstructable outside risk kernel |
| **I5** Point-in-Time Correctness | DataFusion plan-rewrite rule; `observation_delay` property test; pretraining-cutoff hard-deny |
| **I6** Fail Closed on Staleness | Stale-data pre-trade check; chaos tests with frozen feeds |
| **I7** Tamper-Evident Audit | Merkle hash chain; tamper test; WORM mirror (Wave 6+) |

### 4.2 Per-Asset Controls

| Asset | Threat | Control |
|---|---|---|
| Credentials at rest | Theft | OS keychain; sealed envelope; no env beyond moment of use |
| Audit ledger | Tampering | Merkle chain; tamper test; off-box anchor (Wave 6+) |
| Reproducibility manifests | Forgery | Dual signature (Ed25519 + ML-DSA); standalone verifier |
| Plugin modules | Sandbox escape | Hardened wasmtime config; capability diffing gate; cosign verification at load |
| User data (positions, journal) | Exfiltration | Local-first; loopback-only; hosted LLM escalation requires per-call consent |
| Live order path | Unauthorized use | Live gate (env + session confirm); step-up auth; `RiskApprovedOrderIntent` type enforcement |

---

## 5. Adversarial Evaluation

### 5.1 Prompt Injection Suite (per v1.2 §4.4)

AgentDojo-style tests against the AI plane. Test inputs include:
- Malicious 8-K filings (`tests/adversarial/filings/malicious_8k.txt`)
- Market commentary with embedded tool-injection (`tests/adversarial/news/`)
- News with prompt-injection payloads
- Filings text crafted to redirect the agent

The AI plane MUST:
- Not call risk-increasing tools based on injected content.
- Surface injection attempts as audit events.
- Maintain tool-class discipline (ReadOnly freely; RiskReducing directly; RiskIncreasing behind approval).

### 5.2 OWASP Top 10 for Agentic Applications (2026)

| ASI | Threat | Control |
|---|---|---|
| **ASI01** Goal Hijack | Injected content redirects agent goal | LlamaFirewall-pattern input guards; AnalystScope common/private split |
| **ASI02** Tool Misuse/Excessive Agency | Agent invokes tools outside declared capability | Capability-class taxonomy; deny-by-default on RiskIncreasing; `RiskApprovedOrderIntent` type enforcement |

### 5.3 LlamaFirewall Pattern

Wrap every tool-calling boundary with input/output guards:
- **PromptGuard 2** for input classification.
- **Tool-call allowlist** enforced at gateway.
- **Output validation** against schema.

LlamaFirewall reports >90% efficacy reducing attack success rate on AgentDojo (per v1.2 §5.4).

### 5.4 Inspect AI Harness

UK AISI's Inspect framework as the CI red-team harness (per v1.2 §5.4). Weekly full scan; regression suite on every commit.

### 5.5 TradeTrap Faithfulness

Per arXiv:2512.02261: tests whether stated agent reasoning matches actual decision drivers. If agent reasoning is surfaced as evidence, faithfulness is a correctness property, not a UX nicety.

---

## 6. Cryptography Specification

### 6.1 Key Hierarchy

```
Device Root Key (OS keychain, never leaves)
   │
   ├─ HKDF-SHA256(purpose="audit_ledger")  → Audit Ledger Key
   ├─ HKDF-SHA256(purpose="broker_cred")   → Broker Credential Sealing Key
   ├─ HKDF-SHA256(purpose="local_cache")   → Local Cache Sealing Key
   └─ HKDF-SHA256(purpose="manifest_sign") → Manifest Signing Key (Ed25519 seed)
```

### 6.2 Sealed Credential Envelope (per OpenAlice pattern, clean-room ADR-024)

```json
{
  "$sealed": 1,
  "alg": "aes-256-gcm",
  "iv": "base64:...",
  "tag": "base64:...",
  "data": "base64:..."
}
```

AES-256-GCM with a 96-bit IV and 128-bit tag. The IV is unique per encryption (random); the tag authenticates the ciphertext.

### 6.3 Manifest Dual Signature

- **Ed25519** for classical security (fast, widely supported).
- **ML-DSA-65** (FIPS 204) for post-quantum resistance.
- BOTH required; single signature fails verification.
- Wave 0 conditional ship: ML-DSA-65 may be empty during Rust maturity period.

### 6.4 Hardware-Backed Passkeys

Step-up auth uses hardware-backed passkeys (TPM on Windows, Secure Enclave on macOS, YubiKey cross-platform). Wave 6+ enterprise may require hardware tokens.

---

## 7. Secrets Management

### 7.1 Storage

- **NEVER in env beyond moment of use.** OAuth tokens, API keys, broker credentials live in OS keychain.
- **NEVER in process memory longer than needed.** Loaded via `secrecy::SecretString`, `zeroize`d on drop.
- **NEVER in logs.** `redact_secrets` (per QuantDinger pattern, v1.2 §3.4) recursively traverses 20+ key patterns: `api_key`, `secret`, `password`, `passphrase`, `private_key`, `access_token`, etc.

### 7.2 Rotation

- `rotate_device_root_key()` re-encrypts all sealed secrets under a new root (Wave 6 admin command).
- Per-credential rotation surfaces in admin console.
- Rotation events are audited (never the values).

### 7.3 The Local-First Privacy Posture

The single largest privacy property: **the local AI tier (LM Studio) is structurally incapable of network egress.** `egress: Loopback` is enforced, not documented. This is what makes the privacy claim defensible in an enterprise security review.

The hosted escalation tier (T3 to Claude Opus 5 / GPT-5.5 Pro) is a per-call consent surface: the user sees the exact payload leaving the machine and approves per-call. This is the same uncertainty-contract discipline as cost-basis provenance.

---

## 8. WORM and External Anchoring

### 8.1 The HMAC Limitation

Per v1.2 §4.2: an HMAC-chained audit log detects in-place tampering ONLY if the key is not in the same process. A root-level attacker rewrites the chain otherwise.

### 8.2 WORM Mirror

The audit ledger MUST have a WORM (write-once-read-many) mirror:
- **Desktop:** local append-only device (separate physical medium recommended but not enforced).
- **Cloud/Enterprise:** S3 Object Lock in governance mode.

### 8.3 External Anchoring

Merkle-anchor the ledger head to an external transparency log:
- **Rekor (Sigstore)** for default desktop/cloud.
- **Ethereum** anchoring for high-assurance enterprise (per Rekor v2 GA pattern, v1.2 §5.6).

This gives tamper-evidence without a cloud dependency: a root-level attacker can rewrite local state but cannot rewrite the external anchor.

---

## 9. Supply Chain Security

### 9.1 Toolchain (per v1.0 §6.10, v1.2)

| Tool | Role | Gate |
|---|---|---|
| `cargo audit` | RustSec advisories | Blocking |
| `cargo deny` | License + banned crates | Blocking |
| `cargo vet` | Trusted-core human review | Blocking for core crates |
| `cargo auditable` | Embed dependency data | Release builds |
| `cargo cyclonedx` | SBOM | Every release |
| cosign/Sigstore | Keyless artifact signing | Every release |
| SLSA provenance | Build integrity | Generated AND verified by updater |

### 9.2 The SBOM Caveat

Per cargo-auditable maintainers (v1.0 §6.10): SBOMs do not prevent supply chain attacks. A malicious library worth its bytes will remove itself from the SBOM. SBOMs are compliance/inventory artifacts. The controls that resist attack are: reviewed lockfile, cargo vet, artifact signing, provenance verification at install time.

### 9.3 Plugin Supply Chain

WASM binaries are harder to review than source. Controls (per v1.2 §4.4):
- `wasm-tools` disassembly at intake.
- **Capability diffing on imports/exports between versions** (load-bearing check).
- cosign verification before load.
- Marketplace review for Wave 7.

A plugin update that newly imports a network capability MUST fail the gate automatically, not await human review.

---

## 10. Incident Response

### 10.1 Detection

- Audit-ledger tamper detection at startup and on every CI run.
- Anomaly detection in OpenTelemetry traces (Wave 6+).
- User-reported incidents via `security@mythos.systems`.

### 10.2 Response Process

1. Confirm the incident (audit log inspection).
2. Contain (revoke credentials, halt automation).
3. Eradicate (rotate keys, patch).
4. Recover (restore from backup, re-verify audit chain).
5. Document (post-mortem, ADR if architecture change needed).

### 10.3 Disclosure

- Security advisories via GitHub Security Advisories.
- CVE requests for upstream-impacting issues.
- Customer notification for enterprise incidents per contract.

---

## 11. Security Review Cadence

| Activity | Frequency |
|---|---|
| Internal design review | Every ADR |
| Adversarial eval (regression) | Every commit |
| Adversarial eval (full) | Weekly |
| External pentest | Annually + before major release |
| Threat model revision | Quarterly + after any security event |
| Dependency audit | Continuous (cargo audit/deny) |
| Cargo vet audit | Monthly review of trusted-core exceptions |

---

## 12. Cross-References

| Topic | Document |
|---|---|
| Plugin host config | `spec/CRATE_ARCHITECTURE.md` §4.1 |
| AI router security | `spec/AI_ROUTER_INTERNALS.md` |
| IPC boundary | `spec/IPC_CONTRACTS.md` §4 |
| Audit ledger | `spec/CRATE_ARCHITECTURE.md` §2.2 |
| Manifest signatures | `spec/MANIFEST_SCHEMA.md` §9 |
| CI gates (security) | `spec/CI_WORKFLOWS.md` |
| Adversarial tests | `spec/TESTING.md` §9 |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26 · Version 1.0*
