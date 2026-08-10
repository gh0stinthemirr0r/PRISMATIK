<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface AgentTurn {
    role: string;
    label: string;
    analysis: string;
    stance: 'long' | 'short' | 'neutral';
    conviction: number;
    model: string;
    input_tokens: number | null;
    output_tokens: number | null;
  }

  interface AgentRecommendation {
    stance: 'long' | 'short' | 'neutral';
    conviction: number;
    rationale: string;
    suggested_entry: number | null;
    suggested_stop: number | null;
    suggested_target: number | null;
    risk_approved: boolean;
    risk_caveats: string[];
    agents_consulted: number;
  }

  interface AgentCouncilResult {
    subject: string;
    question: string;
    turns: AgentTurn[];
    recommendation: AgentRecommendation;
    evidence_count: number;
    evidence_ids: string[];
    generated_at: string;
    execution_eligible: boolean;
    message: string;
  }

  interface ModelRuntimeStatus {
    provider_id: string;
    active: boolean;
  }

  let subject = $state('BTC');
  let question = $state('Should we take a position in the next 24 hours?');
  let model = $state('gpt-4o');
  let result = $state<AgentCouncilResult | null>(null);
  let loading = $state(false);
  let error = $state('');
  let providers: ModelRuntimeStatus[] = $state([]);
  let anyProviderActive = $state(false);

  async function refreshProviders() {
    try {
      providers = await invoke<ModelRuntimeStatus[]>('model_runtime_status');
      anyProviderActive = providers.some((p) => p.active);
    } catch {}
  }

  onMount(() => {
    refreshProviders();
  });

  async function runCouncil() {
    loading = true;
    error = '';
    result = null;
    try {
      result = await invoke<AgentCouncilResult>('run_agent_council', {
        req: { subject, question, model, max_output_tokens: 500 },
      });
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  const stanceColor = (s: string) =>
    s === 'long' ? '#34d399' : s === 'short' ? '#f87171' : '#94a3b8';
  const stanceIcon = (s: string) => (s === 'long' ? '▲' : s === 'short' ? '▼' : '◆');
</script>

<svelte:head><title>Agent council · PRISMATIK</title></svelte:head>

<div class="pk-page">
  <div class="pk-page-head">
    <h1>Agent council</h1>
    <p>
      Six-role AI debate: technical + fundamental analysts → bull vs bear adversarial researchers →
      risk manager → portfolio manager. The recommendation is advisory only — it never auto-executes.
      Operator approves; the risk gate (1%/trade + 10% drawdown kill switch) enforces.
    </p>
  </div>

  {#if !anyProviderActive}
    <div class="pk-warn">
      No AI model provider connected. Connect one in <a href="/workspace/models">Models</a> first.
    </div>
  {/if}

  {#if error}
    <div class="pk-error">{error}</div>
  {/if}

  <section class="pk-panel">
    <div class="pk-panel-head"><span>Debate prompt</span></div>
    <div class="pk-form-row">
      <label>
        <span>Subject</span>
        <input bind:value={subject} placeholder="BTC, AAPL, ES…" />
      </label>
      <label class="pk-grow">
        <span>Question</span>
        <input bind:value={question} />
      </label>
      <label>
        <span>Model</span>
        <input bind:value={model} placeholder="gpt-4o, claude-sonnet-4…" />
      </label>
      <button class="pk-run" onclick={runCouncil} disabled={loading || !anyProviderActive}>
        {loading ? 'Debating…' : 'Convene council'}
      </button>
    </div>
  </section>

  {#if loading}
    <div class="pk-debate-loading">
      <div class="pk-debate-pulse"></div>
      <span>Analysts are reasoning over real governed evidence…</span>
    </div>
  {/if}

  {#if result}
    <section class="pk-panel pk-recommendation" class:long={result.recommendation.stance === 'long'} class:short={result.recommendation.stance === 'short'}>
      <div class="pk-panel-head"><span>Portfolio manager decision</span></div>
      <div class="pk-rec-body">
        <div class="pk-rec-stance" style="color: {stanceColor(result.recommendation.stance)}">
          <span class="pk-rec-icon">{stanceIcon(result.recommendation.stance)}</span>
          <span class="pk-rec-label">{result.recommendation.stance.toUpperCase()}</span>
          <span class="pk-rec-conv">{Math.round(result.recommendation.conviction * 100)}% conviction</span>
        </div>
        <p class="pk-rec-rationale">{result.recommendation.rationale}</p>
        {#if result.recommendation.suggested_entry !== null}
          <div class="pk-rec-levels">
            {#if result.recommendation.suggested_entry !== null}<span>Entry: <b>{result.recommendation.suggested_entry}</b></span>{/if}
            {#if result.recommendation.suggested_stop !== null}<span>Stop: <b>{result.recommendation.suggested_stop}</b></span>{/if}
            {#if result.recommendation.suggested_target !== null}<span>Target: <b>{result.recommendation.suggested_target}</b></span>{/if}
          </div>
        {/if}
        <div class="pk-rec-meta">
          <span>{result.recommendation.agents_consulted} agents consulted</span>
          <span>·</span>
          <span>{result.evidence_count} evidence citations</span>
          <span>·</span>
          <span class="pk-rec-risk" class:approved={result.recommendation.risk_approved} class:denied={!result.recommendation.risk_approved}>
            Risk: {result.recommendation.risk_approved ? 'APPROVED' : 'FLAGGED'}
          </span>
        </div>
        {#if result.recommendation.risk_caveats.length > 0}
          <ul class="pk-caveats">
            {#each result.recommendation.risk_caveats as caveat}<li>{caveat}</li>{/each}
          </ul>
        {/if}
        <p class="pk-exec-note">
          ⚠ Execution-eligible: <strong>false</strong>. This recommendation will never auto-trade.
          To act, place an order through <a href="/workspace/orders">Orders</a> — the risk gate enforces.
        </p>
      </div>
    </section>

    <section class="pk-panel">
      <div class="pk-panel-head"><span>Debate transcript · {result.turns.length} turns</span></div>
      <div class="pk-debate">
        {#each result.turns as turn}
          <article class="pk-turn" class:bull={turn.stance === 'long'} class:bear={turn.stance === 'short'}>
            <header>
              <span class="pk-turn-label">{turn.label}</span>
              <span class="pk-turn-stance" style="color: {stanceColor(turn.stance)}">{stanceIcon(turn.stance)} {turn.stance}</span>
              <span class="pk-turn-conv">{Math.round(turn.conviction * 100)}%</span>
              <span class="pk-turn-model">{turn.model}</span>
            </header>
            <div class="pk-turn-text">{turn.analysis}</div>
          </article>
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  .pk-page { padding: 24px; max-width: 1200px; }
  .pk-page-head h1 { font-size: 1.4rem; font-weight: 600; margin: 0 0 6px; color: var(--p-text); }
  .pk-page-head p { font-size: 0.8125rem; color: var(--p-text-dim); margin: 0 0 20px; max-width: 70ch; line-height: 1.5; }
  .pk-warn { background: rgba(250, 200, 80, 0.1); border: 1px solid rgba(250, 200, 80, 0.25); color: #fbbf24; padding: 10px 14px; border-radius: 4px; font-size: 0.8125rem; margin-bottom: 16px; }
  .pk-warn a { color: var(--p-accent); }
  .pk-error { background: rgba(255, 80, 80, 0.12); border: 1px solid rgba(255, 80, 80, 0.3); color: #ff9090; padding: 10px 14px; border-radius: 4px; font-size: 0.8125rem; margin-bottom: 16px; }
  .pk-panel { background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 8px; margin-bottom: 16px; }
  .pk-panel-head { padding: 12px 16px; border-bottom: 1px solid var(--p-border); font-family: var(--p-mono); font-size: 0.625rem; letter-spacing: 0.14em; text-transform: uppercase; color: var(--p-text-dim); }
  .pk-form-row { padding: 16px; display: flex; gap: 12px; align-items: flex-end; flex-wrap: wrap; }
  .pk-form-row label { display: flex; flex-direction: column; gap: 4px; font-size: 0.8125rem; }
  .pk-form-row label > span { color: var(--p-text-dim); font-size: 0.6875rem; text-transform: uppercase; letter-spacing: 0.08em; }
  .pk-form-row input { background: var(--p-surface2); border: 1px solid var(--p-border); color: var(--p-text); padding: 8px 10px; border-radius: 4px; font-family: var(--p-mono); font-size: 0.8125rem; }
  .pk-grow { flex: 1; min-width: 200px; }
  .pk-run { background: var(--p-accent); color: #000; border: none; padding: 9px 20px; border-radius: 4px; font-weight: 600; cursor: pointer; font-family: var(--p-mono); letter-spacing: 0.06em; text-transform: uppercase; font-size: 0.75rem; height: 36px; }
  .pk-run:hover:not(:disabled) { filter: brightness(1.1); }
  .pk-run:disabled { opacity: 0.4; cursor: not-allowed; }
  .pk-debate-loading { display: flex; align-items: center; gap: 12px; padding: 24px; color: var(--p-text-dim); font-size: 0.875rem; }
  .pk-debate-pulse { width: 10px; height: 10px; border-radius: 50%; background: var(--p-accent); animation: pulse 1.2s ease-in-out infinite; }
  @keyframes pulse { 0%, 100% { opacity: 0.3; } 50% { opacity: 1; } }
  .pk-recommendation { border-left: 3px solid var(--p-border); }
  .pk-recommendation.long { border-left-color: #34d399; }
  .pk-recommendation.short { border-left-color: #f87171; }
  .pk-rec-body { padding: 16px; }
  .pk-rec-stance { display: flex; align-items: baseline; gap: 10px; margin-bottom: 12px; }
  .pk-rec-icon { font-size: 1.5rem; }
  .pk-rec-label { font-size: 1.25rem; font-weight: 700; letter-spacing: 0.06em; }
  .pk-rec-conv { font-family: var(--p-mono); font-size: 0.875rem; color: var(--p-text-dim); }
  .pk-rec-rationale { font-size: 0.875rem; line-height: 1.6; color: var(--p-text); margin: 0 0 12px; white-space: pre-wrap; }
  .pk-rec-levels { display: flex; gap: 20px; font-family: var(--p-mono); font-size: 0.8125rem; color: var(--p-text-dim); margin-bottom: 12px; }
  .pk-rec-levels b { color: var(--p-text); }
  .pk-rec-meta { display: flex; gap: 8px; font-size: 0.6875rem; color: var(--p-text-dim); margin-bottom: 8px; }
  .pk-rec-risk.approved { color: #34d399; }
  .pk-rec-risk.denied { color: #fbbf24; }
  .pk-caveats { margin: 0 0 12px; padding-left: 16px; font-size: 0.75rem; color: #fbbf24; }
  .pk-exec-note { font-size: 0.75rem; color: var(--p-text-dim); padding: 10px; background: rgba(250, 200, 80, 0.06); border-radius: 4px; margin: 0; }
  .pk-exec-note a { color: var(--p-accent); }
  .pk-debate { padding: 8px; }
  .pk-turn { padding: 14px 16px; border-radius: 6px; margin-bottom: 8px; background: var(--p-surface2); border-left: 2px solid var(--p-border); }
  .pk-turn.bull { border-left-color: rgba(52, 211, 153, 0.5); }
  .pk-turn.bear { border-left-color: rgba(248, 113, 113, 0.5); }
  .pk-turn header { display: flex; align-items: center; gap: 10px; margin-bottom: 8px; font-size: 0.6875rem; }
  .pk-turn-label { font-weight: 600; color: var(--p-text); font-size: 0.75rem; }
  .pk-turn-stance { text-transform: uppercase; letter-spacing: 0.06em; font-family: var(--p-mono); }
  .pk-turn-conv { color: var(--p-text-dim); font-family: var(--p-mono); }
  .pk-turn-model { margin-left: auto; color: var(--p-text-dim); font-family: var(--p-mono); opacity: 0.6; }
  .pk-turn-text { font-size: 0.8125rem; line-height: 1.6; color: var(--p-text); white-space: pre-wrap; }
</style>
