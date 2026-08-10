<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface AutonomousTraderView {
    enabled: boolean;
    intervalSeconds: number;
    strategy: string;
    useCloudAnalysis: boolean;
    cloudProvider: string;
    cloudModel: string;
    minConfidence: number;
    instruments: string[];
    ladderLevel: string;
    peakEquityMicros: number;
    lastLoopAt: string | null;
    totalOrdersSubmitted: number;
    totalOrdersSkipped: number;
  }

  interface TraderLoopResult {
    decisions: TraderDecision[];
    ordersSubmitted: number;
    ordersSkipped: number;
    ladderLevel: string;
    budgetRemainingMicros: number;
    budgetHalted: boolean;
    cloudAnalysisUsed: boolean;
    executedAt: string;
    message: string;
  }

  interface TraderDecision {
    instrument: string;
    action: string;
    reason: string;
    price: number;
    confidence: number;
    ladderLevel: string;
    budgetState: string;
  }

  interface RiskStateView {
    armed: boolean;
    tripped: boolean;
    maxRiskPerTradePct: number;
    drawdownHaltPct: number;
    rearmCount: number;
  }

  let trader = $state<AutonomousTraderView | null>(null);
  let risk = $state<RiskStateView | null>(null);
  let loopResult = $state<TraderLoopResult | null>(null);
  let loading = $state(true);
  let saving = $state(false);
  let running = $state(false);
  let error = $state('');

  // Editable config form state
  let enabled = $state(false);
  let intervalSeconds = $state(300);
  let strategy = $state('momentum');
  let useCloudAnalysis = $state(false);
  let cloudProvider = $state('openai');
  let cloudModel = $state('gpt-4o');
  let minConfidence = $state(0.05);
  let instruments = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      trader = await invoke<AutonomousTraderView>('get_autonomous_trader');
      risk = await invoke<RiskStateView>('get_risk_state');
      enabled = trader.enabled;
      intervalSeconds = trader.intervalSeconds;
      strategy = trader.strategy;
      useCloudAnalysis = trader.useCloudAnalysis;
      cloudProvider = trader.cloudProvider || 'openai';
      cloudModel = trader.cloudModel || 'gpt-4o';
      minConfidence = trader.minConfidence;
      instruments = trader.instruments.join(', ');
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    load();
    const interval = setInterval(load, 15_000);
    return () => clearInterval(interval);
  });

  async function saveConfig() {
    saving = true;
    error = '';
    try {
      trader = await invoke<AutonomousTraderView>('configure_autonomous_trader', {
        config: {
          enabled,
          intervalSeconds,
          strategy,
          useCloudAnalysis,
          cloudProvider,
          cloudModel,
          maxAnalysisCostMicros: 100_000,
          minConfidence,
          instruments: instruments.split(',').map((s) => s.trim()).filter((s) => s.length > 0),
          requireStop: true,
        },
      });
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function runOnce() {
    running = true;
    error = '';
    try {
      loopResult = await invoke<TraderLoopResult>('run_trader_loop');
    } catch (e) {
      error = String(e);
    } finally {
      running = false;
    }
  }

  async function rearm() {
    try {
      await invoke('rearm_trader_ladder', { currentEquityMicros: 100_000_000_000 });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function deEscalate() {
    try {
      await invoke('de_escalate_trader_ladder');
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  const ladderColor = (level: string) => {
    const map: Record<string, string> = {
      NORMAL: '#34d399', CAUTION: '#fbbf24', 'DE-RISK': '#f59e0b',
      RESTRICT: '#f87171', FLATTEN: '#ef4444', LOCKDOWN: '#dc2626',
    };
    return map[level] ?? '#94a3b8';
  };

  const fmtUsd = (micros: number) => `$${(micros / 1_000_000).toLocaleString('en-US', { maximumFractionDigits: 2 })}`;
</script>

<svelte:head><title>Autonomous trader · PRISMATIK</title></svelte:head>

<div class="pk-page">
  <div class="pk-page-head">
    <h1>Autonomous trader</h1>
    <p>
      The full autonomy engine. Configure the trading loop, risk gates, and budget controls in one place.
      The agent operates with minimal human interaction — all execution routes through the paper OMS and
      the 1%-risk + hierarchical-drawdown kill switch. Trading halts when budgets are exhausted; research continues.
    </p>
  </div>

  {#if error}
    <div class="pk-error">{error}</div>
  {/if}

  {#if loading}
    <div class="pk-empty">Loading autonomous trader…</div>
  {:else if trader}
    <!-- Status banner -->
    <div class="pk-status-banner" class:enabled={trader.enabled} class:disabled={!trader.enabled}>
      <div class="pk-status-left">
        <span class="pk-status-dot" class:on={trader.enabled}></span>
        <span class="pk-status-label">{trader.enabled ? 'AUTONOMOUS TRADING ACTIVE' : 'AUTONOMOUS TRADING DISABLED'}</span>
      </div>
      <div class="pk-status-right">
        <span class="pk-ladder-badge" style="background: {ladderColor(trader.ladderLevel)}22; color: {ladderColor(trader.ladderLevel)}; border-color: {ladderColor(trader.ladderLevel)}44">
          DRAWDOWN: {trader.ladderLevel}
        </span>
        {#if risk?.tripped}
          <span class="pk-circuit-badge tripped">⚡ BREAKER TRIPPED</span>
        {:else}
          <span class="pk-circuit-badge armed">✓ BREAKER ARMED</span>
        {/if}
      </div>
    </div>

    <div class="pk-grid">
      <!-- Configuration -->
      <section class="pk-panel">
        <div class="pk-panel-head"><span>Trading loop configuration</span></div>
        <div class="pk-form">
          <label class="pk-toggle">
            <input type="checkbox" bind:checked={enabled} />
            <span>Enable autonomous trading loop</span>
          </label>
          <p class="pk-hint">When enabled, the agent evaluates signals every interval and submits risk-gated paper orders. Disable to pause all automated trading.</p>

          <label>
            <span>Loop interval (seconds)</span>
            <input type="number" bind:value={intervalSeconds} min="30" step="30" />
          </label>

          <label>
            <span>Signal strategy</span>
            <select bind:value={strategy}>
              <option value="momentum">Momentum (200-SMA trend)</option>
              <option value="mean_reversion">Mean Reversion (RSI 2)</option>
              <option value="donchian">Donchian (55-bar breakout)</option>
              <option value="orb">Opening Range Breakout</option>
            </select>
          </label>

          <label>
            <span>Minimum confidence to act</span>
            <input type="number" bind:value={minConfidence} min="0" max="1" step="0.01" />
          </label>
          <p class="pk-hint">Below this confidence threshold, the agent abstains. Higher = more conservative.</p>

          <label>
            <span>Instruments (comma-separated, empty = all)</span>
            <input bind:value={instruments} placeholder="BTC, ETH, AAPL" />
          </label>

          <label class="pk-toggle">
            <input type="checkbox" bind:checked={useCloudAnalysis} />
            <span>Use cloud frontier model for analysis</span>
          </label>
          <p class="pk-hint">When enabled, each signal is validated by a cloud LLM analysis pass before execution. Requires a connected provider.</p>

          {#if useCloudAnalysis}
            <div class="pk-sub-form">
              <label>
                <span>Cloud provider</span>
                <select bind:value={cloudProvider}>
                  <option value="openai">OpenAI</option>
                  <option value="anthropic">Anthropic</option>
                  <option value="google">Google Gemini</option>
                  <option value="xai">xAI Grok</option>
                  <option value="deepseek">DeepSeek</option>
                  <option value="groq">Groq</option>
                  <option value="openrouter">OpenRouter</option>
                </select>
              </label>
              <label>
                <span>Model</span>
                <input bind:value={cloudModel} placeholder="gpt-4o, claude-sonnet-4, gemini-2.0-flash" />
              </label>
            </div>
          {/if}

          <button class="pk-save" onclick={saveConfig} disabled={saving}>
            {saving ? 'Saving…' : 'Save configuration'}
          </button>
        </div>
      </section>

      <!-- Risk + Budget Status -->
      <section class="pk-panel">
        <div class="pk-panel-head"><span>Risk gates & budget</span></div>
        <div class="pk-risk-grid">
          <div class="pk-risk-card">
            <div class="pk-risk-label">Max risk / trade</div>
            <div class="pk-risk-value">{((risk?.maxRiskPerTradePct ?? 0.01) * 100).toFixed(1)}%</div>
          </div>
          <div class="pk-risk-card">
            <div class="pk-risk-label">Drawdown halt</div>
            <div class="pk-risk-value">{((risk?.drawdownHaltPct ?? 0.10) * 100).toFixed(0)}%</div>
          </div>
          <div class="pk-risk-card">
            <div class="pk-risk-label">Orders submitted</div>
            <div class="pk-risk-value">{trader.totalOrdersSubmitted}</div>
          </div>
          <div class="pk-risk-card">
            <div class="pk-risk-label">Orders skipped</div>
            <div class="pk-risk-value">{trader.totalOrdersSkipped}</div>
          </div>
        </div>

        {#if trader.ladderLevel !== 'NORMAL'}
          <div class="pk-ladder-controls">
            <p class="pk-ladder-warn" style="color: {ladderColor(trader.ladderLevel)}">
              Drawdown ladder at {trader.ladderLevel}. New positions {trader.ladderLevel === 'LOCKDOWN' || trader.ladderLevel === 'FLATTEN' || trader.ladderLevel === 'RESTRICT' ? 'blocked' : 'reduced'}.
            </p>
            {#if trader.ladderLevel === 'LOCKDOWN'}
              <button class="pk-rearm" onclick={rearm}>🔓 Re-arm (human override)</button>
            {:else}
              <button class="pk-de-escalate" onclick={deEscalate}>↓ De-escalate one level</button>
            {/if}
          </div>
        {/if}

        {#if risk?.tripped}
          <div class="pk-tripped-banner">
            <span>⚡ Circuit breaker tripped. ALL trading halted. Human re-arm required.</span>
            <a href="/workspace/orders">Go to Orders to re-arm →</a>
          </div>
        {/if}
      </section>
    </div>

    <!-- Run + Results -->
    <section class="pk-panel">
      <div class="pk-panel-head">
        <span>Manual loop execution</span>
        <button class="pk-run-once" onclick={runOnce} disabled={running || !enabled}>
          {running ? 'Running…' : '▶ Run one iteration'}
        </button>
      </div>
      {#if loopResult}
        <div class="pk-loop-result" class:halted={loopResult.budgetHalted}>
          <p class="pk-loop-message">{loopResult.message}</p>
          <div class="pk-loop-stats">
            <span>Orders: <strong>{loopResult.ordersSubmitted}</strong></span>
            <span>Skipped: <strong>{loopResult.ordersSkipped}</strong></span>
            <span>Ladder: <strong style="color: {ladderColor(loopResult.ladderLevel)}">{loopResult.ladderLevel}</strong></span>
            <span>Budget: <strong>{loopResult.budgetHalted ? 'HALTED' : fmtUsd(loopResult.budgetRemainingMicros)}</strong></span>
            <span>Cloud: <strong>{loopResult.cloudAnalysisUsed ? 'YES' : 'NO'}</strong></span>
          </div>
          {#if loopResult.decisions.length > 0}
            <table class="pk-decisions">
              <thead>
                <tr><th>Instrument</th><th>Action</th><th>Price</th><th>Ladder</th><th>Budget</th><th>Reason</th></tr>
              </thead>
              <tbody>
                {#each loopResult.decisions as d (d.instrument + d.action)}
                  <tr>
                    <td class="pk-mono">{d.instrument}</td>
                    <td class="pk-mono" style="color: {d.action === 'long' ? '#34d399' : d.action === 'short' ? '#f87171' : '#94a3b8'}">{d.action.toUpperCase()}</td>
                    <td class="pk-mono">{d.price.toFixed(2)}</td>
                    <td class="pk-mono" style="color: {ladderColor(d.ladderLevel)}">{d.ladderLevel}</td>
                    <td class="pk-mono">{d.budgetState}</td>
                    <td class="pk-reason">{d.reason}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>
      {:else}
        <div class="pk-empty-small">Run a manual iteration to see the agent's decisions. The loop also runs automatically when enabled.</div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .pk-page { padding: 24px; max-width: 1200px; }
  .pk-page-head h1 { font-size: 1.4rem; font-weight: 600; margin: 0 0 6px; color: var(--p-text); }
  .pk-page-head p { font-size: 0.8125rem; color: var(--p-text-dim); margin: 0 0 20px; max-width: 70ch; line-height: 1.5; }
  .pk-error { background: rgba(255,80,80,0.12); border: 1px solid rgba(255,80,80,0.3); color: #ff9090; padding: 10px 14px; border-radius: 4px; font-size: 0.8125rem; margin-bottom: 16px; }
  .pk-empty { padding: 40px; text-align: center; color: var(--p-text-dim); }
  .pk-empty-small { padding: 24px; text-align: center; color: var(--p-text-dim); font-size: 0.8125rem; }
  .pk-status-banner { display: flex; justify-content: space-between; align-items: center; padding: 14px 18px; border-radius: 8px; margin-bottom: 16px; }
  .pk-status-banner.enabled { background: rgba(52,211,153,0.08); border: 1px solid rgba(52,211,153,0.2); }
  .pk-status-banner.disabled { background: rgba(148,163,184,0.06); border: 1px solid var(--p-border); }
  .pk-status-left { display: flex; align-items: center; gap: 10px; }
  .pk-status-dot { width: 9px; height: 9px; border-radius: 50%; background: var(--p-text-dim); }
  .pk-status-dot.on { background: #34d399; box-shadow: 0 0 8px rgba(52,211,153,0.5); animation: pulse 2s infinite; }
  @keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.6; } }
  .pk-status-label { font-family: var(--p-mono); font-size: 0.75rem; font-weight: 600; letter-spacing: 0.06em; }
  .pk-status-right { display: flex; gap: 8px; }
  .pk-ladder-badge { font-family: var(--p-mono); font-size: 0.625rem; padding: 3px 10px; border-radius: 3px; border: 1px solid; letter-spacing: 0.06em; }
  .pk-circuit-badge { font-family: var(--p-mono); font-size: 0.625rem; padding: 3px 10px; border-radius: 3px; border: 1px solid; }
  .pk-circuit-badge.armed { background: rgba(52,211,153,0.1); color: #34d399; border-color: rgba(52,211,153,0.3); }
  .pk-circuit-badge.tripped { background: rgba(248,113,113,0.1); color: #f87171; border-color: rgba(248,113,113,0.3); }
  .pk-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-bottom: 16px; }
  .pk-panel { background: var(--p-surface); border: 1px solid var(--p-border); border-radius: 8px; }
  .pk-panel-head { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--p-border); font-family: var(--p-mono); font-size: 0.625rem; letter-spacing: 0.14em; text-transform: uppercase; color: var(--p-text-dim); }
  .pk-form { padding: 16px; display: flex; flex-direction: column; gap: 14px; }
  .pk-form label { display: flex; flex-direction: column; gap: 4px; font-size: 0.8125rem; }
  .pk-form label > span { color: var(--p-text-dim); font-size: 0.6875rem; text-transform: uppercase; letter-spacing: 0.08em; }
  .pk-form select, .pk-form input { background: var(--p-surface2); border: 1px solid var(--p-border); color: var(--p-text); padding: 8px 10px; border-radius: 4px; font-family: var(--p-mono); font-size: 0.8125rem; }
  .pk-toggle { flex-direction: row !important; align-items: center; gap: 8px !important; cursor: pointer; }
  .pk-toggle input { accent-color: var(--p-accent); width: 16px; height: 16px; }
  .pk-toggle span { font-size: 0.8125rem !important; color: var(--p-text) !important; text-transform: none !important; letter-spacing: 0 !important; }
  .pk-hint { font-size: 0.6875rem; color: var(--p-text-dim); line-height: 1.5; margin: -8px 0 0; }
  .pk-sub-form { display: flex; gap: 10px; padding-left: 24px; border-left: 2px solid var(--p-accent); flex-direction: column; gap: 10px; }
  .pk-save { background: var(--p-accent); color: #000; border: none; padding: 10px; border-radius: 4px; font-weight: 600; cursor: pointer; font-family: var(--p-mono); letter-spacing: 0.06em; text-transform: uppercase; font-size: 0.75rem; }
  .pk-save:hover:not(:disabled) { filter: brightness(1.1); }
  .pk-save:disabled { opacity: 0.5; }
  .pk-risk-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1px; background: var(--p-border); }
  .pk-risk-card { background: var(--p-surface); padding: 14px; display: flex; flex-direction: column; gap: 4px; }
  .pk-risk-label { font-size: 0.625rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--p-text-dim); }
  .pk-risk-value { font-family: var(--p-mono); font-size: 1.25rem; font-weight: 600; color: var(--p-text); }
  .pk-ladder-controls { padding: 14px 16px; }
  .pk-ladder-warn { font-size: 0.8125rem; margin: 0 0 10px; font-weight: 500; }
  .pk-rearm { background: rgba(52,211,153,0.15); border: 1px solid rgba(52,211,153,0.4); color: #34d399; padding: 8px 16px; border-radius: 4px; cursor: pointer; font-family: var(--p-mono); font-size: 0.75rem; }
  .pk-rearm:hover { background: rgba(52,211,153,0.25); }
  .pk-de-escalate { background: rgba(251,191,36,0.12); border: 1px solid rgba(251,191,36,0.3); color: #fbbf24; padding: 8px 16px; border-radius: 4px; cursor: pointer; font-family: var(--p-mono); font-size: 0.75rem; }
  .pk-de-escalate:hover { background: rgba(251,191,36,0.2); }
  .pk-tripped-banner { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; background: rgba(248,113,113,0.08); border-top: 1px solid rgba(248,113,113,0.2); color: #f87171; font-size: 0.8125rem; }
  .pk-tripped-banner a { color: var(--p-accent); }
  .pk-run-once { background: var(--p-accent2); color: #fff; border: none; padding: 6px 14px; border-radius: 4px; cursor: pointer; font-family: var(--p-mono); font-size: 0.6875rem; }
  .pk-run-once:hover:not(:disabled) { filter: brightness(1.15); }
  .pk-run-once:disabled { opacity: 0.4; }
  .pk-loop-result { padding: 16px; }
  .pk-loop-result.halted { background: rgba(251,191,36,0.04); }
  .pk-loop-message { font-size: 0.875rem; color: var(--p-text); margin: 0 0 12px; line-height: 1.5; }
  .pk-loop-stats { display: flex; gap: 20px; font-size: 0.75rem; color: var(--p-text-dim); margin-bottom: 14px; flex-wrap: wrap; }
  .pk-loop-stats strong { color: var(--p-text); font-family: var(--p-mono); }
  .pk-decisions { width: 100%; border-collapse: collapse; }
  .pk-decisions th { text-align: left; padding: 8px 12px; font-size: 0.625rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--p-text-dim); border-bottom: 1px solid var(--p-border); }
  .pk-decisions td { padding: 8px 12px; font-size: 0.75rem; border-bottom: 1px solid var(--p-border); }
  .pk-mono { font-family: var(--p-mono); }
  .pk-reason { font-size: 0.6875rem; color: var(--p-text-dim); max-width: 300px; }
  @media (max-width: 800px) { .pk-grid { grid-template-columns: 1fr; } }
</style>
