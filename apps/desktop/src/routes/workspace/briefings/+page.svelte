<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface BriefingPosition {
    symbol: string;
    quantity: string;
    mark_price_micros: number | null;
    market_value_micros: string | null;
    unrealized_pnl_micros: string | null;
  }

  interface RiskFlag {
    severity: string;
    flag: string;
    detail: string;
  }

  interface BriefingTrade {
    symbol: string;
    side: string;
    quantity: string;
    price_micros: number;
    occurred_at: string;
    pnl_micros: number | null;
  }

  interface Briefing {
    mode: 'morning' | 'evening';
    generated_at: string;
    headline: string;
    positions: BriefingPosition[];
    risk_flags: RiskFlag[];
    trades_today: BriefingTrade[];
    best_trade: BriefingTrade | null;
    worst_trade: BriefingTrade | null;
    realized_pnl_micros: number | null;
    backtest_match: string | null;
    circuit_breaker_armed: boolean;
    message: string;
  }

  let mode = $state<'morning' | 'evening'>('morning');
  let briefing = $state<Briefing | null>(null);
  let loading = $state(false);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      briefing = await invoke<Briefing>('get_briefing', { mode });
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  // Load on mount.
  load();

  $effect(() => {
    // Re-fetch when mode changes.
    mode;
    load();
  });

  const fmtMicros = (m: number | null) =>
    m === null ? '—' : `$${(m / 1_000_000).toLocaleString('en-US', { maximumFractionDigits: 2 })}`;
  const fmtPnl = (m: string | null) => {
    if (m === null) return '—';
    const v = parseInt(m, 10) / 1_000_000;
    const sign = v >= 0 ? '+' : '';
    return `${sign}$${v.toLocaleString('en-US', { maximumFractionDigits: 2 })}`;
  };
  const pnlColor = (m: string | null) => {
    if (m === null) return 'var(--p-text-dim)';
    return parseInt(m, 10) >= 0 ? '#34d399' : '#f87171';
  };
</script>

<svelte:head><title>Briefings · PRISMATIK</title></svelte:head>

<div class="pk-page">
  <div class="pk-page-head">
    <h1>Daily briefings</h1>
    <p>
      Morning: open positions, unrealized P&L, risk flags, circuit-breaker state.
      Evening: trades executed today, best/worst trade, backtest-match guidance.
      Computed from real governed state — never fabricated.
    </p>
  </div>

  <div class="pk-mode-tabs">
    <button class:active={mode === 'morning'} onclick={() => (mode = 'morning')}>☀️ Morning (pre-open)</button>
    <button class:active={mode === 'evening'} onclick={() => (mode = 'evening')}>🌙 Evening (post-close)</button>
  </div>

  {#if error}
    <div class="pk-error">{error}</div>
  {/if}

  {#if loading}
    <div class="pk-empty">Compiling briefing…</div>
  {:else if briefing}
    <div class="pk-headline">{briefing.headline}</div>

    <div class="pk-breaker" class:armed={briefing.circuit_breaker_armed} class:tripped={!briefing.circuit_breaker_armed}>
      <span class="pk-breaker-dot"></span>
      <span>{briefing.circuit_breaker_armed ? 'Circuit breaker ARMED — trading permitted' : 'Circuit breaker TRIPPED — trading halted, human re-arm required'}</span>
    </div>

    {#if briefing.risk_flags.length > 0}
      <section class="pk-panel">
        <div class="pk-panel-head"><span>Risk flags ({briefing.risk_flags.length})</span></div>
        <div class="pk-flags">
          {#each briefing.risk_flags as flag}
            <div class="pk-flag" class:critical={flag.severity === 'critical'} class:warning={flag.severity === 'warning'}>
              <span class="pk-flag-sev">{flag.severity.toUpperCase()}</span>
              <span class="pk-flag-name">{flag.flag}</span>
              <span class="pk-flag-detail">{flag.detail}</span>
            </div>
          {/each}
        </div>
      </section>
    {/if}

    {#if briefing.positions.length > 0}
      <section class="pk-panel">
        <div class="pk-panel-head"><span>Open positions ({briefing.positions.length})</span></div>
        <table class="pk-table">
          <thead><tr><th>Symbol</th><th>Qty</th><th>Mark</th><th>Market Value</th><th>Unrealized P&L</th></tr></thead>
          <tbody>
            {#each briefing.positions as pos}
              <tr>
                <td class="pk-mono">{pos.symbol}</td>
                <td class="pk-mono">{pos.quantity}</td>
                <td class="pk-mono">{fmtMicros(pos.mark_price_micros)}</td>
                <td class="pk-mono">{fmtMicros(pos.market_value_micros ? parseInt(pos.market_value_micros, 10) : null)}</td>
                <td class="pk-mono" style="color: {pnlColor(pos.unrealized_pnl_micros)}">{fmtPnl(pos.unrealized_pnl_micros)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>
    {:else}
      <div class="pk-empty-panel">No open positions.</div>
    {/if}

    {#if mode === 'evening'}
      {#if briefing.trades_today.length > 0}
        <section class="pk-panel">
          <div class="pk-panel-head"><span>Trades today ({briefing.trades_today.length})</span></div>
          <table class="pk-table">
            <thead><tr><th>Symbol</th><th>Side</th><th>Qty</th><th>Price</th><th>Time</th></tr></thead>
            <tbody>
              {#each briefing.trades_today as trade}
                <tr class:best={briefing.best_trade?.symbol === trade.symbol && briefing.best_trade?.occurred_at === trade.occurred_at} class:worst={briefing.worst_trade?.symbol === trade.symbol && briefing.worst_trade?.occurred_at === trade.occurred_at}>
                  <td class="pk-mono">{trade.symbol}</td>
                  <td class="pk-mono {trade.side}">{trade.side.toUpperCase()}</td>
                  <td class="pk-mono">{trade.quantity}</td>
                  <td class="pk-mono">{fmtMicros(trade.price_micros)}</td>
                  <td class="pk-mono pk-dim">{new Date(trade.occurred_at).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' })}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </section>
      {:else}
        <div class="pk-empty-panel">No trades executed today.</div>
      {/if}

      {#if briefing.backtest_match}
        <section class="pk-panel pk-bt-match">
          <div class="pk-panel-head"><span>Backtest match</span></div>
          <p>{briefing.backtest_match}</p>
        </section>
      {/if}
    {/if}

    <div class="pk-realized">
      <span>Total unrealized P&L:</span>
      <span class="pk-mono" style="color: {briefing.realized_pnl_micros === null ? 'var(--p-text-dim)' : briefing.realized_pnl_micros >= 0 ? '#34d399' : '#f87171'}">
        {briefing.realized_pnl_micros === null ? 'withheld (unmarked positions)' : fmtMicros(briefing.realized_pnl_micros)}
      </span>
    </div>

    <p class="pk-message">{briefing.message}</p>
  {/if}
</div>

<style>
  .pk-page { padding: 24px; max-width: 1000px; }
  .pk-page-head h1 { font-size: 1.4rem; font-weight: 600; margin: 0 0 6px; color: var(--p-text); }
  .pk-page-head p { font-size: 0.8125rem; color: var(--p-text-dim); margin: 0 0 20px; max-width: 65ch; line-height: 1.5; }
  .pk-mode-tabs { display: flex; gap: 4px; margin-bottom: 20px; background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 6px; padding: 4px; width: fit-content; }
  .pk-mode-tabs button { background: none; border: none; color: var(--p-text-dim); padding: 8px 18px; border-radius: 4px; cursor: pointer; font-size: 0.8125rem; font-weight: 500; transition: all 0.15s; }
  .pk-mode-tabs button.active { background: var(--p-surface3); color: var(--p-text); }
  .pk-error { background: rgba(255, 80, 80, 0.12); border: 1px solid rgba(255, 80, 80, 0.3); color: #ff9090; padding: 10px 14px; border-radius: 4px; font-size: 0.8125rem; margin-bottom: 16px; }
  .pk-empty { padding: 40px; text-align: center; color: var(--p-text-dim); }
  .pk-headline { font-size: 1rem; font-weight: 600; color: var(--p-text); margin-bottom: 16px; padding: 12px 16px; background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 6px; }
  .pk-breaker { display: flex; align-items: center; gap: 10px; padding: 10px 16px; border-radius: 6px; margin-bottom: 16px; font-size: 0.8125rem; font-family: var(--p-mono); }
  .pk-breaker.armed { background: rgba(52, 211, 153, 0.08); color: #34d399; border: 1px solid rgba(52, 211, 153, 0.2); }
  .pk-breaker.tripped { background: rgba(248, 113, 113, 0.1); color: #f87171; border: 1px solid rgba(248, 113, 113, 0.3); }
  .pk-breaker-dot { width: 8px; height: 8px; border-radius: 50%; background: currentColor; }
  .pk-panel { background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 8px; margin-bottom: 16px; }
  .pk-panel-head { padding: 12px 16px; border-bottom: 1px solid var(--p-border); font-family: var(--p-mono); font-size: 0.625rem; letter-spacing: 0.14em; text-transform: uppercase; color: var(--p-text-dim); }
  .pk-flags { padding: 8px; display: flex; flex-direction: column; gap: 6px; }
  .pk-flag { display: flex; align-items: center; gap: 12px; padding: 10px 12px; border-radius: 4px; font-size: 0.8125rem; }
  .pk-flag.critical { background: rgba(248, 113, 113, 0.08); }
  .pk-flag.warning { background: rgba(251, 191, 36, 0.06); }
  .pk-flag-sev { font-family: var(--p-mono); font-size: 0.625rem; letter-spacing: 0.1em; font-weight: 700; min-width: 70px; }
  .pk-flag.critical .pk-flag-sev { color: #f87171; }
  .pk-flag.warning .pk-flag-sev { color: #fbbf24; }
  .pk-flag-name { font-family: var(--p-mono); color: var(--p-text-dim); min-width: 160px; }
  .pk-flag-detail { color: var(--p-text); flex: 1; }
  .pk-table { width: 100%; border-collapse: collapse; }
  .pk-table th { text-align: left; padding: 10px 16px; font-size: 0.625rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--p-text-dim); border-bottom: 1px solid var(--p-border); font-weight: 500; }
  .pk-table td { padding: 10px 16px; font-size: 0.8125rem; border-bottom: 1px solid var(--p-border); }
  .pk-mono { font-family: var(--p-mono); }
  .pk-dim { color: var(--p-text-dim); }
  .pk-table tr.best { background: rgba(52, 211, 153, 0.05); }
  .pk-table tr.worst { background: rgba(248, 113, 113, 0.05); }
  .pk-table .buy { color: #34d399; }
  .pk-table .sell { color: #f87171; }
  .pk-empty-panel { padding: 24px; text-align: center; color: var(--p-text-dim); font-size: 0.875rem; background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 8px; margin-bottom: 16px; }
  .pk-bt-match p { padding: 14px 16px; font-size: 0.8125rem; line-height: 1.6; color: var(--p-text-dim); margin: 0; }
  .pk-realized { display: flex; justify-content: space-between; align-items: center; padding: 16px; background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 8px; margin-bottom: 16px; font-size: 0.875rem; }
  .pk-realized .pk-mono { font-size: 1.125rem; font-weight: 600; }
  .pk-message { font-size: 0.8125rem; color: var(--p-text-dim); line-height: 1.5; }
</style>
