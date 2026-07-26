# PRISMATIK AI Router Internals Specification

**Document:** `spec/AI_ROUTER_INTERNALS.md`
**Status:** NORMATIVE — RFC 2119 keywords apply
**Companion to:** `spec/CRATE_ARCHITECTURE.md` §4.2 (`prismatik-ai-tools`), `PRISMATIK_v1.2_Reference_Consolidation_and_AI_Stack.md` §4 (the AI stack), `PRISMATIK_Unified_Solution_Architecture_v1.0.md` §20 (AI plane)
**Date:** 2026-07-26

---

## 0. Purpose

This document specifies the **internal mechanics of the AI router**: provider dispatch logic, capability-based routing rules, the recorded-effect replay contract (determinism for LLM calls), MCP server registration, and the AnalystScope enforcement machinery.

The cardinal rules:
- The LLM never holds write authority over anything it can also hallucinate about (v1.0 §20.1). Order submission is not a tool.
- Every inference call is a recorded effect; replay returns the recorded response, never re-invokes the model (v1.0 §12, v1.2 §5).
- Capability declaration drives routing, not model name (v1.2 §4).
- The local tier's `egress: Loopback` is enforced, not documented (v1.2 §4.5).

---

## 1. Architecture

```
                          ┌────────────────────────────────┐
                          │     AiRouter (entry point)     │
                          │  route(req) → AiResponse        │
                          └─────────────┬──────────────────┘
                                        │
                ┌───────────────────────┼───────────────────────┐
                │                       │                       │
        ┌───────▼────────┐      ┌───────▼────────┐      ┌───────▼────────┐
        │ Tier Selector  │      │ Recorded-Effect│      │ Tool Gateway   │
        │ (T0–T3)        │      │ Replay Cache   │      │ (capability-   │
        └───────┬────────┘      └───────┬────────┘      │  scoped)       │
                │                       │               └───────┬────────┘
                │                       │                       │
        ┌───────▼───────────────────────▼───────────────────────▼──────┐
        │                   Provider Dispatch                           │
        │  OAuth tier / API-key tier / Local tier (LM Studio)           │
        └───────┬───────────────────────┬───────────────────────┬──────┘
                │                       │                       │
        ┌───────▼──────┐        ┌───────▼──────┐        ┌───────▼──────┐
        │ OpenRouter   │        │ Anthropic    │        │ LM Studio    │
        │ Gemini       │        │ OpenAI       │        │ (loopback)   │
        │ Alpaca MCP   │        │ xAI Grok     │        │              │
        │ IBKR MCP     │        │ DeepSeek     │        │              │
        │ Coinbase MCP │        │ Cohere       │        │              │
        │              │        │ Groq         │        │              │
        │              │        │ Together/FW  │        │              │
        └──────────────┘        └──────────────┘        └──────────────┘
```

---

## 2. Provider Registration

Providers are registered at startup via `InferenceProviderConfig`. Each provider carries:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferenceProvider {
    pub id: ProviderId,
    pub kind: InferenceProviderKind,
    pub auth: AuthMethod,
    pub endpoint: Url,
    pub model_id: String,
    pub model_digest: ContentHash,        // pinned weights
    pub context_window: usize,
    pub capabilities: InferenceCapabilities,
    pub determinism: DeterminismProfile,
    pub egress: EgressPolicy,
    pub cost_model: CostModel,
    pub max_tokens_per_call: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InferenceProviderKind {
    OAuth { provider: OAuthProvider, scopes: Vec<String> },
    ApiKey { provider: ApiKeyProvider },
    LmStudioLocal,
    OpenAiCompatible { provider_name: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthMethod {
    OAuth { token_source: TokenSource },
    ConsoleApiKey { key_vault_ref: SecretRef },
    Loopback,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferenceCapabilities {
    pub tool_use: bool,
    pub json_schema: bool,
    pub vision: bool,
    pub reasoning: bool,
    pub max_tool_rounds_per_step: u8,
    pub max_output_tokens: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeterminismProfile {
    pub temperature: f64,    // pinned; default 0.0 for deterministic
    pub seed: Option<u64>,   // if provider supports
    pub top_p: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EgressPolicy {
    Loopback,        // enforced for LM Studio; runtime checks socket target
    Internet { allowed_hosts: Vec<HostPattern> },
}
```

---

## 3. Tier Selection

The router selects tier based on workload, then picks a provider within that tier based on capability + cost + availability.

```rust
pub enum WorkloadTier {
    T0,  // deterministic: symbology, extraction, classification, tagging
    T1,  // local default: filing summary, evidence, chart compose, journal draft
    T2,  // local reasoning: multi-evidence synthesis, thesis critique, risk narrative
    T3,  // escalation: hard synthesis, adversarial critique, code gen
}

pub fn select_tier(workload: &Workload) -> WorkloadTier {
    match workload.kind {
        WorkloadKind::SymbologyNormalization => WorkloadTier::T0,
        WorkloadKind::FieldExtraction => WorkloadTier::T0,
        WorkloadKind::FilingSummarization => WorkloadTier::T1,
        WorkloadKind::EvidenceExtraction => WorkloadTier::T1,
        WorkloadKind::MultiEvidenceSynthesis => WorkloadTier::T2,
        WorkloadKind::ThesisCritique => WorkloadTier::T2,
        WorkloadKind::AdversarialCritique => WorkloadTier::T3,
        WorkloadKind::CodeGeneration => WorkloadTier::T3,
        // ... etc
    }
}
```

### 3.1 Per-Tier Defaults

| Tier | Default route | Fallback | Notes |
|---|---|---|---|
| T0 | LM Studio Phi-4-mini (local) | LM Studio Qwen3-8B (local) | Structured output mandatory |
| T1 | LM Studio Qwen3-14B Q4_K_M (local) | Groq Llama 4 Scout (hosted, fast) | |
| T2 | LM Studio Qwen3-30B-A3B MoE (local) | Claude Sonnet 4.5 (API key) | |
| T3 | Claude Opus 5 or GPT-5.5 Pro (hosted) | OpenRouter OAuth | **User opt-in per call** |

### 3.2 T3 Consent Surface

T3 is the per-call consent surface. The router returns `EscalationConsentRequested` with the exact payload summary:

```rust
pub enum RouteResult {
    Routed { provider: ProviderId, recorded_effect_id: EffectId },
    ConsentRequired { escalation: EscalationRequest },
    ProviderUnavailable { reason: String, fallback_recommended: Option<ProviderId> },
}

pub struct EscalationRequest {
    pub payload_summary: String,        // human-readable summary
    pub payload_size_bytes: usize,
    pub target_model: String,
    pub target_provider: ProviderId,
    pub estimated_cost: CostEstimate,
    pub data_leaving_machine: Vec<DataKind>,  // positions, thesis, etc.
}
```

The user reviews and approves (or denies). Approval produces a `StepUpToken` scoped to this single call.

---

## 4. Capability-Based Dispatch

The router matches workload-required capabilities against provider-declared capabilities. A mismatch degrades gracefully rather than failing.

```rust
pub fn dispatch(req: InferenceRequest) -> RouteResult {
    let tier = select_tier(&req.workload);
    let required_caps = requirements_for(&req.workload);

    for provider in providers_for_tier(tier) {
        if provider.capabilities.superset_of(required_caps) {
            if let Some(effect) = replay_cache.get(&req, &provider) {
                return RouteResult::Routed { provider: provider.id, recorded_effect_id: effect.id };
            }
            return route_to(provider, req);
        }
    }

    // No provider meets all capabilities; degrade
    if required_caps.json_schema {
        // Try free-form with post-validation
        return route_with_post_validation(req);
    }
    RouteResult::ProviderUnavailable { .. }
}
```

**Capability examples:**
- If workload requires `json_schema` and provider doesn't support it: fall back to free-form + post-validation against schema. Reject on validation failure.
- If workload requires `vision` and provider doesn't support it: route to a vision-capable provider (Gemini, GPT-5.5).
- If workload requires `tool_use` and provider doesn't support it: error (no safe degradation for tool use).

---

## 5. Recorded-Effect Replay (Determinism)

Every inference call is a recorded effect. Replay returns the recorded response, never re-invokes the model.

### 5.1 Effect Recording

```rust
pub struct InferenceEffect {
    pub effect_id: EffectId,
    pub run_id: RunId,
    pub provider_id: ProviderId,
    pub model_id: String,
    pub model_digest: ContentHash,        // MUST match at replay
    pub prompt_hash: ContentHash,         // canonical hash of prompt + params
    pub determinism_profile: DeterminismProfile,    // MUST match at replay
    pub request_params: serde_json::Value,
    pub recorded_response: Vec<u8>,       // the actual model output
    pub recorded_at: OffsetDateTime,
}
```

### 5.2 Replay Cache Lookup

```rust
impl ReplayCache {
    pub fn get(&self, req: &InferenceRequest, provider: &InferenceProvider) -> Option<&InferenceEffect> {
        let prompt_hash = canonical_hash(&req.prompt, &req.params);
        self.index.get(&ReplayKey {
            run_id: req.run_id,
            prompt_hash,
            model_digest: provider.model_digest,
            determinism: provider.determinism.clone(),
        })
    }
}
```

**Cache key includes `model_digest` + `determinism_profile` + prompt hash.** A model swap (LM Studio loads different weights) invalidates the cache. A temperature change invalidates the cache.

### 5.3 Replay Semantics

When a run is replayed (via `prismatik-cli replay --run-id <id>`):
1. The replay engine walks the run's recorded effects in order.
2. Each inference call is satisfied from the cache.
3. The model is NEVER re-invoked.
4. The output is byte-identical to the original run.

This is the determinism property for LLM calls. A model silently updated by the vendor would produce a different output on fresh invocation; the cache prevents that.

### 5.4 Model Swap Detection

If `model_digest` doesn't match the loaded model (LM Studio can swap models), replay detects and surfaces:

```rust
if loaded_model.digest != recorded.model_digest {
    return Err(ReplayError::ModelDigestMismatch {
        expected: recorded.model_digest,
        found: loaded_model.digest,
    });
}
```

---

## 6. Tool Gateway and Capability Classes

The tool gateway enforces the ReadOnly / RiskReducing / RiskIncreasing taxonomy (per v1.2 §3.4).

### 6.1 Tool Registration

```rust
pub struct ControlledTool {
    pub name: String,
    pub class: ToolClass,
    pub description: String,
    pub input_schema: JsonSchema,
    pub output_schema: JsonSchema,
    pub handler: ToolHandler,
}

pub enum ToolClass {
    ReadOnly,
    RiskReducing,
    RiskIncreasing,    // DENY BY DEFAULT
}
```

Every tool registration MUST declare a class. Risk-increasing tools are deny-by-default.

### 6.2 Dispatch with Class Enforcement

```rust
pub async fn invoke_tool(&self, name: &str, args: Json, caller: CallerKind) -> ToolResult {
    let tool = self.registry.get(name)?;
    match (tool.class, caller) {
        (ToolClass::ReadOnly, _) => self.invoke(tool, args).await,
        (ToolClass::RiskReducing, CallerKind::Agent) => self.invoke(tool, args).await,
        (ToolClass::RiskReducing, CallerKind::User) => self.invoke(tool, args).await,
        (ToolClass::RiskIncreasing, CallerKind::Agent) => {
            // DENY; require out-of-band approval
            Err(ToolError::ApprovalRequired { tool: name })
        }
        (ToolClass::RiskIncreasing, CallerKind::User) => {
            // User explicit invocation; route through approval gate
            self.invoke_with_step_up(tool, args).await
        }
    }
}
```

### 6.3 The Risk-Ordering Principle

**Agents may take risk-reducing actions directly; risk-increasing actions require the approval gate.** Examples:
- `stop_strategy` — risk-reducing — agent may call directly.
- `cancel_order` — risk-reducing (strictly) — agent may call directly.
- `propose_order_intent` — risk-increasing — agent may draft but cannot submit. The approval gate converts the intent to `RiskApprovedOrderIntent` (unconstructable outside risk kernel).

### 6.4 The Order Submission Boundary

The single load-bearing rule (v1.0 §20.1):

> Live order submission is not a tool. It is a deterministic workflow that the `draft_order` tool feeds into. The AI never calls submit.

```rust
// prismatik-ai-tools: the agent's only entry
pub async fn draft_order_intent(input: OrderIntentInput) -> OrderIntentDraft {
    // Returns a draft with NO submission path.
}

// prismatik-risk: only this crate can construct RiskApprovedOrderIntent
pub struct RiskApprovedOrderIntent(OrderIntent);
impl RiskApprovedOrderIntent {
    pub(crate) fn approve(intent: OrderIntent, catalog: &CheckCatalog) -> Result<Self, RiskError> { ... }
}

// prismatik-execution: accepts ONLY the approved newtype
pub async fn submit_order(approved: RiskApprovedOrderIntentJson) -> SubmissionResult { ... }
// There is NO submit_order(OrderIntent) signature.
```

The agent cannot bypass risk. This is enforced by the type system, not by policy.

---

## 7. AnalystScope Enforcement

Per v1.2 §3.3 (clean-room from ai-trading-claude, MIT): the technical analyst MUST NOT see the news the sentiment analyst sees.

### 7.1 The Common-Private Split

```rust
pub struct AnalystScope {
    pub common_facts: FactBundle,        // shared: asset identity, current quote, calendar
    pub private_evidence: EvidenceScope, // disjoint per analyst
}
```

The `common_facts` are facts (not interpretations): asset identity, current quote, calendar context. Cheap to share, no correlation risk because there is nothing to interpret.

The `private_evidence` is disjoint per analyst. The technical analyst's evidence query CANNOT retrieve news; the sentiment analyst's CANNOT retrieve price indicators. **Enforced at the evidence-query layer, not by convention.**

### 7.2 Per-Role Scopes

```rust
pub fn evidence_scope_for(role: AnalystRole) -> EvidenceScope {
    match role {
        AnalystRole::Technical => EvidenceScope {
            allowed_kinds: vec![DataKind::Bar, DataKind::Indicator, DataKind::Quote],
            forbidden_kinds: vec![DataKind::News, DataKind::Filing],
        },
        AnalystRole::Fundamental => EvidenceScope {
            allowed_kinds: vec![DataKind::Filing, DataKind::Fundamental, DataKind::InstitutionalHolding],
            forbidden_kinds: vec![DataKind::News],
        },
        AnalystRole::Sentiment => EvidenceScope {
            allowed_kinds: vec![DataKind::News, DataKind::SocialMedia, DataKind::AnalystRating],
            forbidden_kinds: vec![DataKind::Bar, DataKind::Indicator],
        },
        AnalystRole::Risk => EvidenceScope {
            allowed_kinds: vec![DataKind::Bar, DataKind::Volatility, DataKind::Position],
            forbidden_kinds: vec![DataKind::News],
        },
        AnalystRole::Thesis => EvidenceScope::all(),  // synthesis; sees everything post-fan-out
    }
}
```

### 7.3 Why This Matters

Without the split, five analysts reading the same discovery bundle "independently agree" — but their agreement is one opinion reported five times. The split makes agreement meaningful: "4 of 5 analysts converged on disjoint evidence" is a defensible claim; "4 of 5 agents agreed after reading the same article" is not.

### 7.4 Disagreement as First-Class Output

```rust
pub struct AnalystResponse {
    pub per_analyst: Vec<AnalystResult>,
    pub composite_score: CompositeScore,
    pub disagreement_summary: DisagreementSummary,  // NEVER averaged away
    pub evidence_refs: Vec<EvidenceRef>,
}
```

The `disagreement_summary` is reported alongside the composite, never aggregated away. Divergence is information; averaging it away destroys the most useful thing fan-out produces.

---

## 8. MCP Server Registration

### 8.1 Internal MCP Server (PRISMATIK as server)

`prismatik-ai-tools` exposes the controlled-tool catalogue as an MCP server (per v1.0 §20.3, rmcp-based). External MCP clients (Claude Desktop, etc.) connect and see only the tools they're entitled to.

```rust
#[derive(rmcp::Tool)]
pub struct PrismatikMcpServer { /* ... */ }

#[rmcp::tool]
impl PrismatikMcpServer {
    pub async fn search_assets(&self, query: AssetQuery) -> Vec<AssetSummary> { ... }
    pub async fn get_bars(&self, q: BarsQuery) -> BarsResult { ... }
    pub async fn draft_order_intent(&self, input: OrderIntentInput) -> OrderIntentDraft { ... }
    // ... etc, the controlled-tool catalogue
}
```

### 8.2 External MCP Client (PRISMATIK as client)

PRISMATIK consumes signed third-party MCP servers. Per v1.0 §20.3, external MCP servers require:
1. A signature verified before first use.
2. An explicit approval recorded in the audit ledger.
3. Capability scoping (the MCP server's tool surface is gated by `ToolGateway`).

### 8.3 MCP Auth (2026-07-28 RC alignment)

Per v1.2 §5.3: MCP servers act as OAuth 2.1 Resource Servers; clients as OAuth clients. Requires PKCE + Resource Indicators (RFC 8707).

```rust
pub struct McpServerRegistration {
    pub server_id: McpServerId,
    pub endpoint: Url,
    pub auth: McpAuthConfig,        // OAuth 2.1 RS
    pub signed_manifest: SignedManifest,
    pub approved_at: Option<OffsetDateTime>,
    pub approved_by: Option<ActorId>,
}
```

### 8.4 The Stateless Core

Per the 2026-07-28 RC (v1.2 §5.3): the protocol core is stateless. No `Mcp-Session-Id`. Any request can hit any server instance. PRISMATIK's MCP client and server both implement this stateless core.

---

## 9. Streaming and Backpressure

### 9.1 Token Streaming

LLM token streams use a bounded channel between trusted core and WebView (per `spec/IPC_CONTRACTS.md` §5.1). If the WebView can't keep up, the inference provider is paused (not buffered indefinitely).

```rust
// Internal channel (within trusted core): unbounded
// External channel (to WebView): bounded, backpressure
let (internal_tx, internal_rx) = tokio::sync::mpsc::unbounded_channel();
let (external_tx, external_rx) = tokio::sync::mpsc::bounded_channel(64);

// Forwarder applies backpressure
tokio::spawn(async move {
    while let Some(token) = internal_rx.recv().await {
        if external_tx.send(token).await.is_err() {
            // WebView dropped; pause inference
            break;
        }
    }
});
```

### 9.2 Cancellation

If the user cancels an in-flight request, the trusted core cancels the upstream provider call (where supported) and discards partial output. The cancellation is recorded in the audit log if the call was already a recorded effect.

---

## 10. Cost Governance

Each call's cost is computed BEFORE the call (per `BudgetGovernor`):

```rust
pub fn cost_of(req: &InferenceRequest, provider: &InferenceProvider) -> CostUnits {
    let tokens_in = req.prompt.estimated_tokens();
    let tokens_out = req.max_output_tokens;
    let rate_in = provider.cost_model.input_per_mtok;
    let rate_out = provider.cost_model.output_per_mtok;
    CostUnits((tokens_in * rate_in + tokens_out * rate_out) / 1_000_000)
}
```

The cost is recorded alongside the recorded effect. Cost accumulated per provider is visible in the admin console and observability dashboards.

---

## 11. Cross-References

| Topic | Document |
|---|---|
| AI stack (provider selection) | `PRISMATIK_v1.2_Reference_Consolidation_and_AI_Stack.md` §4 |
| AI plane boundary | `PRISMATIK_Unified_Solution_Architecture_v1.0.md` §20 |
| AI tools crate | `spec/CRATE_ARCHITECTURE.md` §4.2 |
| IPC for AI commands | `spec/IPC_CONTRACTS.md` §2.11 |
| Threat model (AI plane) | `spec/SECURITY_THREAT_MODEL.md` §3.5 |
| Adversarial evals | `spec/TESTING.md` §9 |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26 · Version 1.0*
