<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface LiveArmingView {
    armedUntil: string | null;
    armedBy: string | null;
    broker: string | null;
    active: boolean;
  }

  interface TickRecord {
    at: string;
    ok: boolean;
    ordersSubmitted: number;
    ordersSkipped: number;
    ladderLevel: string;
    message: string;
  }

  interface SchedulerStatus {
    running: boolean;
    intervalSeconds: number;
    mode: string;
    recentTicks: TickRecord[];
    message: string;
  }

  interface AutonomousTraderView {
    enabled: boolean;
    mode: string;
    tradingLimitMicros: number;
    tradingUsedMicros: number;
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
  /**
   * How much consequence the loop is permitted. Live is not simply a setting:
   * the backend re-checks every gate on every decision and silently falls back
   * to paper when any of them fails, so this control expresses intent, not
   * permission.
   */
  let mode = $state<'advisory' | 'paper' | 'live'>('paper');
  let arming = $state<LiveArmingView | null>(null);
  let scheduler = $state<SchedulerStatus | null>(null);
  let armBroker = $state('alpaca');
  let armOperator = $state('');
  let armMinutes = $state(60);
  let armError = $state('');
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
      mode = (trader.mode as typeof mode) ?? 'paper';
      arming = await invoke<LiveArmingView>('get_live_arming');
      scheduler = await invoke<SchedulerStatus>('autonomy_loop_status');
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

  async function refreshAutonomy(): Promise<void> {
    try {
      arming = await invoke<LiveArmingView>('get_live_arming');
      scheduler = await invoke<SchedulerStatus>('autonomy_loop_status');
    } catch (e) {
      armError = String(e);
    }
  }

  async function armLive(): Promise<void> {
    armError = '';
    try {
      arming = await invoke<LiveArmingView>('arm_live_trading', {
        broker: armBroker.trim(),
        seconds: Math.max(60, Math.round(armMinutes * 60)),
        operator: armOperator.trim(),
      });
    } catch (e) {
      armError = String(e);
    }
  }

  async function disarmLive(): Promise<void> {
    armError = '';
    try {
      arming = await invoke<LiveArmingView>('disarm_live_trading');
    } catch (e) {
      armError = String(e);
    }
  }

  async function toggleLoop(): Promise<void> {
    armError = '';
    try {
      scheduler = await invoke<SchedulerStatus>(
        scheduler?.running ? 'stop_autonomy_loop' : 'start_autonomy_loop',
      );
    } catch (e) {
      armError = String(e);
    }
  }

  async function saveConfig() {
    saving = true;
    error = '';
    try {
      trader = await invoke<AutonomousTraderView>('configure_autonomous_trader', {
        config: {
          enabled,
          mode,
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
        <!-- Colour comes from a data attribute, not an inline style: the Tauri
             CSP is `style-src 'self'`, which drops style attributes silently —
             the badge would simply lose its colour in the packaged app. -->
        <span class="pk-ladder-badge" data-ladder={trader.ladderLevel}>
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
      <!-- Autonomy mode + loop -->
      <section class="pk-panel pk-autonomy">
        <div class="pk-panel-head"><span>Autonomy</span></div>
        <div class="pk-form">
          <div class="pk-modes" role="radiogroup" aria-label="Autonomy mode">
            {#each [['advisory', 'Advisory', 'Journals what it would do. Submits nothing.'], ['paper', 'Paper', 'Submits to the local paper OMS.'], ['live', 'Live', 'Submits to a real broker, behind every gate.']] as option (option[0])}
              <button
                class="pk-mode"
                role="radio"
                aria-checked={mode === option[0]}
                class:active={mode === option[0]}
                data-mode={option[0]}
                onclick={() => (mode = option[0] as typeof mode)}
              >
                <b>{option[1]}</b><small>{option[2]}</small>
              </button>
            {/each}
          </div>
          <p class="pk-hint">
            Mode is intent, not permission. Every gate — measured skill, armed breaker, drawdown
            ladder, budget, and an unexpired arming — is re-checked on every decision, and any
            failure drops that decision back to paper with the reason recorded.
          </p>

          {#if mode === 'live'}
            <div class="pk-arm" class:armed={arming?.active}>
              <div class="pk-arm-state">
                <strong>{arming?.active ? 'ARMED' : 'NOT ARMED'}</strong>
                <small>
                  {arming?.active
                    ? `${arming.broker} · expires ${new Date(arming.armedUntil ?? '').toLocaleTimeString()} · armed by ${arming.armedBy}`
                    : 'Live execution requires an explicit, expiring authorisation.'}
                </small>
              </div>
              {#if arming?.active}
                <button class="pk-disarm" onclick={() => void disarmLive()}>Disarm now</button>
              {:else}
                <div class="pk-arm-form">
                  <label><span>Broker</span><input bind:value={armBroker} /></label>
                  <label><span>Operator</span><input bind:value={armOperator} placeholder="your name" /></label>
                  <label><span>Minutes</span><input type="number" min="1" max="480" bind:value={armMinutes} /></label>
                  <button class="pk-arm-go" onclick={() => void armLive()}>Arm live</button>
                </div>
              {/if}
            </div>
          {/if}

          <!-- The most common reason the loop never trades, made visible on
               the surface where you decide to trade. -->
          {#if trader.tradingLimitMicros === 0}
            <div class="pk-budget none">
              <div>
                <strong>NO TRADING BUDGET</strong>
                <small>
                  Positions cannot be sized until a trading limit is allocated. Research, analysis
                  and advisory decisions continue regardless.
                </small>
              </div>
              <a href="/workspace/autonomy">Allocate budget →</a>
            </div>
          {:else}
            <div class="pk-budget">
              <div>
                <strong>
                  TRADING BUDGET ${((trader.tradingLimitMicros - trader.tradingUsedMicros) / 1e6).toFixed(2)}
                  <i>of ${(trader.tradingLimitMicros / 1e6).toFixed(2)} remaining</i>
                </strong>
                <small>Gross capital reservation available to autonomous orders.</small>
              </div>
              <a href="/workspace/autonomy">Adjust →</a>
            </div>
          {/if}

          <div class="pk-loop">
            <div>
              <strong class:on={scheduler?.running}>
                LOOP {scheduler?.running ? 'RUNNING' : 'STOPPED'}
              </strong>
              <small>{scheduler?.message ?? 'Scheduler state unknown'}</small>
            </div>
            <div class="pk-loop-actions">
              <button onclick={() => void toggleLoop()}>
                {scheduler?.running ? 'Stop loop' : 'Start loop'}
              </button>
              <button onclick={() => void refreshAutonomy()}>Refresh</button>
              <a href="/workspace/agent-log">Agent log →</a>
            </div>
          </div>

          {#if armError}<p class="pk-arm-error">{armError}</p>{/if}
        </div>
      </section>

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

  .pk-ladder-badge[data-ladder='NORMAL'] {
    border-color: color-mix(in srgb, var(--p-up) 40%, transparent);
    background: color-mix(in srgb, var(--p-up) 13%, transparent);
    color: var(--p-up);
  }
  .pk-ladder-badge[data-ladder='CAUTION'],
  .pk-ladder-badge[data-ladder='DE-RISK'] {
    border-color: color-mix(in srgb, #fbbf24 40%, transparent);
    background: color-mix(in srgb, #fbbf24 13%, transparent);
    color: #fbbf24;
  }
  .pk-ladder-badge[data-ladder='RESTRICT'],
  .pk-ladder-badge[data-ladder='FLATTEN'],
  .pk-ladder-badge[data-ladder='LOCKDOWN'] {
    border-color: color-mix(in srgb, var(--p-down) 45%, transparent);
    background: color-mix(in srgb, var(--p-down) 14%, transparent);
    color: var(--p-down);
  }

  .pk-modes {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 6px;
  }
  .pk-mode {
    display: grid;
    gap: 3px;
    padding: 10px;
    border: 1px solid var(--p-border);
    border-radius: 7px;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
    text-align: left;
  }
  .pk-mode b {
    color: var(--p-text);
    font: 700 0.72rem var(--font-mono);
  }
  .pk-mode small {
    font-size: 0.62rem;
    line-height: 1.4;
  }
  .pk-mode.active {
    border-color: var(--p-accent);
    background: color-mix(in srgb, var(--p-accent) 10%, transparent);
  }
  /* Live is visually distinct because its consequences are. */
  .pk-mode.active[data-mode='live'] {
    border-color: var(--p-down);
    background: color-mix(in srgb, var(--p-down) 12%, transparent);
  }
  .pk-mode.active[data-mode='live'] b {
    color: var(--p-down);
  }

  .pk-arm {
    display: grid;
    gap: 8px;
    padding: 11px;
    border: 1px solid color-mix(in srgb, var(--p-down) 45%, var(--p-border));
    border-radius: 7px;
    background: color-mix(in srgb, var(--p-down) 6%, transparent);
  }
  .pk-arm.armed {
    border-color: color-mix(in srgb, var(--p-up) 45%, var(--p-border));
    background: color-mix(in srgb, var(--p-up) 6%, transparent);
  }
  .pk-arm-state strong {
    font: 700 0.68rem var(--font-mono);
    letter-spacing: 0.1em;
  }
  .pk-arm.armed .pk-arm-state strong {
    color: var(--p-up);
  }
  .pk-arm-state small {
    display: block;
    margin-top: 3px;
    color: var(--p-dim);
    font-size: 0.64rem;
  }
  .pk-arm-form {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 8px;
  }
  .pk-arm-form label {
    display: grid;
    gap: 3px;
  }
  .pk-arm-form span {
    color: var(--p-dim);
    font: 600 8px var(--font-mono);
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  .pk-arm-form input {
    width: 120px;
    padding: 6px 8px;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    background: var(--p-panel-fill);
    color: var(--p-text);
    font-family: var(--font-mono);
    font-size: 0.72rem;
  }
  .pk-arm-go,
  .pk-disarm {
    padding: 7px 13px;
    border: 1px solid var(--p-down);
    border-radius: 6px;
    background: color-mix(in srgb, var(--p-down) 14%, transparent);
    color: var(--p-down);
    cursor: pointer;
    font: 700 0.62rem var(--font-mono);
    letter-spacing: 0.08em;
  }
  .pk-arm-error {
    margin: 0;
    color: var(--p-down);
    font-size: 0.68rem;
  }

  .pk-budget {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 11px;
    border: 1px solid var(--p-border);
    border-radius: 7px;
  }
  .pk-budget.none {
    border-color: color-mix(in srgb, #fbbf24 45%, var(--p-border));
    background: color-mix(in srgb, #fbbf24 7%, transparent);
  }
  .pk-budget strong {
    font: 700 0.66rem var(--font-mono);
    letter-spacing: 0.08em;
  }
  .pk-budget.none strong {
    color: #fbbf24;
  }
  .pk-budget i {
    color: var(--p-dim);
    font-style: normal;
    font-weight: 400;
  }
  .pk-budget small {
    display: block;
    margin-top: 3px;
    max-width: 60ch;
    color: var(--p-dim);
    font-size: 0.62rem;
    line-height: 1.45;
  }
  .pk-budget a {
    padding: 6px 11px;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    color: var(--p-accent);
    font: 700 0.6rem var(--font-mono);
    text-decoration: none;
    white-space: nowrap;
  }

  .pk-loop {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 11px;
    border: 1px solid var(--p-border);
    border-radius: 7px;
  }
  .pk-loop strong {
    font: 700 0.66rem var(--font-mono);
    letter-spacing: 0.1em;
  }
  .pk-loop strong.on {
    color: var(--p-up);
  }
  .pk-loop small {
    display: block;
    margin-top: 3px;
    color: var(--p-dim);
    font-size: 0.62rem;
  }
  .pk-loop-actions {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .pk-loop-actions button,
  .pk-loop-actions a {
    padding: 6px 11px;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    background: transparent;
    color: var(--p-accent);
    cursor: pointer;
    font: 700 0.6rem var(--font-mono);
    text-decoration: none;
    white-space: nowrap;
  }
</style>
