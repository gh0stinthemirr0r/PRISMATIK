<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface SeedStrategyInfo {
    id: string;
    kind: 'orb' | 'momentum' | 'mean_reversion' | 'donchian';
    name: string;
    description: string;
    asset_class: string;
    best_regime: string;
  }

  interface EquityPointView {
    bar_index: number;
    equity: number;
    position: number;
  }

  interface BacktestResultView {
    strategy_id: string;
    bars_processed: number;
    starting_capital: number;
    ending_equity: number;
    total_return: number;
    annualized_return: number;
    sharpe: number;
    sortino: number;
    calmar: number;
    max_drawdown: number;
    profit_factor: number;
    win_rate: number;
    trade_count: number;
    fill_count: number;
    verdict: string;
    message: string;
    equity_curve_sampled: EquityPointView[];
  }

  interface BarInput {
    open: number;
    high: number;
    low: number;
    close: number;
    volume?: number;
    timestamp_secs?: number;
  }

  let strategies: SeedStrategyInfo[] = $state([]);
  let selectedStrategy: 'orb' | 'momentum' | 'mean_reversion' | 'donchian' = $state('momentum');
  let selectedCalendar: 'equity_us' | 'equity_future_us' | 'crypto_daily' | 'forex' = $state('equity_us');
  let barInterval = $state(86400);
  let startingCapital = $state(100000);
  let barCount = $state(500);
  let regimeType: 'trending' | 'mean_reverting' | 'flat' = $state('trending');
  let result: BacktestResultView | null = $state(null);
  let loading = $state(false);
  let error = $state('');

  onMount(async () => {
    try {
      strategies = await invoke<SeedStrategyInfo[]>('list_seed_strategies');
    } catch (e) {
      error = String(e);
    }
  });

  // Deterministic synthetic bar generator (clearly labeled — never mixed
  // with real governed data). Used for demo/visualization only.
  function generateBars(
    n: number,
    regime: 'trending' | 'mean_reverting' | 'flat',
  ): BarInput[] {
    // Mulberry32 PRNG — pure 32-bit integer math, deterministic.
    let state = 42 | 0;
    const rand = () => {
      state = (state + 0x6d2b79f5) | 0;
      let z = state;
      z = Math.imul(z ^ (z >>> 15), z | 1);
      z ^= z + Math.imul(z ^ (z >>> 7), z | 61);
      return ((z ^ (z >>> 14)) >>> 0) / 4294967296;
    };
    const normal = () => {
      const u1 = Math.max(rand(), 1e-12);
      const u2 = rand();
      return Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
    };

    let price = 100;
    const bars: BarInput[] = [];
    const regimeLen = 120;
    for (let i = 0; i < n; i++) {
      let drift = 0;
      if (regime === 'trending') {
        const phase = Math.floor(i / regimeLen) % 4;
        drift = phase === 0 || phase === 3 ? 0.0015 : phase === 1 ? 0 : -0.0012;
      } else if (regime === 'mean_reverting') {
        drift = 0.8 * ((100 - price) / 100);
      }
      const vol = regime === 'mean_reverting' ? 0.018 : 0.008;
      const ret = drift + vol * normal();
      price *= Math.max(1 + ret, 0.5);
      bars.push({
        open: price * 0.9998,
        high: price * 1.0015,
        low: price * 0.9985,
        close: price,
        volume: 1000,
      });
    }
    return bars;
  }

  async function runBacktest() {
    loading = true;
    error = '';
    result = null;
    try {
      const bars = generateBars(barCount, regimeType);
      result = await invoke<BacktestResultView>('run_backtest', {
        req: {
          strategy: selectedStrategy,
          bars,
          starting_capital: startingCapital,
          calendar: selectedCalendar,
          bar_interval_seconds: barInterval,
          slippage_bps: 2,
          commission_per_share_micros: 500,
        },
      });
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  const fmtPct = (v: number) => `${(v * 100).toFixed(2)}%`;
  const fmtUsd = (v: number) => `$${v.toLocaleString('en-US', { maximumFractionDigits: 0 })}`;
  const fmtNum = (v: number) => v.toFixed(3);

  // Chart scaling for equity curve.
  let chartPath = $derived.by(() => {
    if (!result || result.equity_curve_sampled.length < 2) return '';
    const pts = result.equity_curve_sampled;
    const w = 800;
    const h = 200;
    const minEq = Math.min(...pts.map((p) => p.equity));
    const maxEq = Math.max(...pts.map((p) => p.equity));
    const range = maxEq - minEq || 1;
    const scaleX = (i: number) => (i / (pts.length - 1)) * w;
    const scaleY = (eq: number) => h - ((eq - minEq) / range) * (h - 20) - 10;
    return pts.map((p, i) => `${i === 0 ? 'M' : 'L'}${scaleX(i).toFixed(1)},${scaleY(p.equity).toFixed(1)}`).join(' ');
  });
</script>

<svelte:head><title>Backtest laboratory · PRISMATIK</title></svelte:head>

<div class="pk-page">
  <div class="pk-page-head">
    <h1>Backtest laboratory</h1>
    <p>
      Run deterministic backtests of seed strategies with real Sharpe/Sortino/Calmar computation.
      Every result is reproducible — same inputs always produce byte-identical metrics.
    </p>
  </div>

  {#if error}
    <div class="pk-error">{error}</div>
  {/if}

  <div class="pk-grid">
    <section class="pk-panel">
      <div class="pk-panel-head"><span>Configuration</span></div>
      <div class="pk-form">
        <label>
          <span>Strategy</span>
          <select bind:value={selectedStrategy}>
            {#each strategies as s}
              <option value={s.kind}>{s.name}</option>
            {/each}
          </select>
        </label>
        {#if strategies.find((s) => s.kind === selectedStrategy)}
          <p class="pk-strat-desc">{strategies.find((s) => s.kind === selectedStrategy)!.description}</p>
          <p class="pk-strat-regime">
            <strong>Best regime:</strong> {strategies.find((s) => s.kind === selectedStrategy)!.best_regime}
          </p>
        {/if}

        <label>
          <span>Calendar / asset class</span>
          <select bind:value={selectedCalendar}>
            <option value="equity_us">US Equity (252 days/yr)</option>
            <option value="equity_future_us">US Equity Futures — ES/NQ (252 × 23h)</option>
            <option value="crypto_daily">Crypto (365 × 24h)</option>
            <option value="forex">Forex (252 × 24h)</option>
          </select>
        </label>

        <label>
          <span>Bar interval (seconds)</span>
          <input type="number" bind:value={barInterval} min="60" step="60" />
        </label>

        <label>
          <span>Starting capital ($)</span>
          <input type="number" bind:value={startingCapital} min="1000" step="1000" />
        </label>

        <label>
          <span>Bar count</span>
          <input type="number" bind:value={barCount} min="50" max="5000" step="50" />
        </label>

        <label>
          <span>Demo data regime</span>
          <select bind:value={regimeType}>
            <option value="trending">Trending (regime-shifting)</option>
            <option value="mean_reverting">Mean-reverting (OU)</option>
            <option value="flat">Flat / random walk</option>
          </select>
        </label>

        <button class="pk-run" onclick={runBacktest} disabled={loading}>
          {loading ? 'Running…' : 'Run backtest'}
        </button>
        <p class="pk-disclaimer">
          Demo bars are synthetic and clearly labeled. To backtest real governed history, paste a bar
          series or wire a provider through Integrations. Synthetic data never enters evidence chains.
        </p>
      </div>
    </section>

    <section class="pk-panel pk-results">
      <div class="pk-panel-head"><span>Result</span></div>
      {#if !result && !loading}
        <div class="pk-empty">Run a backtest to see metrics.</div>
      {:else if loading}
        <div class="pk-empty">Simulating…</div>
      {:else if result}
        <div class="pk-verdict" class:positive={result.verdict === 'positive_sharpe'} class:negative={result.verdict !== 'positive_sharpe'}>
          {result.verdict === 'positive_sharpe' ? '✓ POSITIVE SHARPE' : '✗ NON-POSITIVE SHARPE'}
        </div>
        <div class="pk-metrics">
          <div class="pk-metric"><span class="pk-metric-label">Sharpe</span><span class="pk-metric-value">{fmtNum(result.sharpe)}</span></div>
          <div class="pk-metric"><span class="pk-metric-label">Sortino</span><span class="pk-metric-value">{fmtNum(result.sortino)}</span></div>
          <div class="pk-metric"><span class="pk-metric-label">Calmar</span><span class="pk-metric-value">{fmtNum(result.calmar)}</span></div>
          <div class="pk-metric"><span class="pk-metric-label">Max DD</span><span class="pk-metric-value">{fmtPct(result.max_drawdown)}</span></div>
          <div class="pk-metric"><span class="pk-metric-label">Total Return</span><span class="pk-metric-value">{fmtPct(result.total_return)}</span></div>
          <div class="pk-metric"><span class="pk-metric-label">Annualized</span><span class="pk-metric-value">{fmtPct(result.annualized_return)}</span></div>
          <div class="pk-metric"><span class="pk-metric-label">Profit Factor</span><span class="pk-metric-value">{result.profit_factor > 1e15 ? '∞' : fmtNum(result.profit_factor)}</span></div>
          <div class="pk-metric"><span class="pk-metric-label">Win Rate</span><span class="pk-metric-value">{fmtPct(result.win_rate)}</span></div>
          <div class="pk-metric"><span class="pk-metric-label">Trades</span><span class="pk-metric-value">{result.trade_count}</span></div>
          <div class="pk-metric"><span class="pk-metric-label">Fills</span><span class="pk-metric-value">{result.fill_count}</span></div>
        </div>
        {#if result.equity_curve_sampled.length > 1}
          <div class="pk-chart">
            <svg viewBox="0 0 800 200" preserveAspectRatio="none">
              <path d={chartPath} fill="none" stroke="var(--p-accent)" stroke-width="1.5" />
            </svg>
            <div class="pk-chart-labels">
              <span>{fmtUsd(result.starting_capital)}</span>
              <span>Equity curve · {result.bars_processed} bars</span>
              <span>{fmtUsd(result.ending_equity)}</span>
            </div>
          </div>
        {/if}
        <p class="pk-message">{result.message}</p>
      {/if}
    </section>
  </div>
</div>

<style>
  .pk-page { padding: 24px; max-width: 1400px; }
  .pk-page-head h1 { font-size: 1.4rem; font-weight: 600; margin: 0 0 6px; color: var(--p-text); }
  .pk-page-head p { font-size: 0.8125rem; color: var(--p-text-dim); margin: 0 0 20px; max-width: 60ch; line-height: 1.5; }
  .pk-grid { display: grid; grid-template-columns: 340px 1fr; gap: 16px; }
  .pk-panel { background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 8px; display: flex; flex-direction: column; min-height: 400px; }
  .pk-panel-head { padding: 14px 16px; border-bottom: 1px solid var(--p-border); font-family: var(--p-mono); font-size: 0.625rem; letter-spacing: 0.14em; text-transform: uppercase; color: var(--p-text-dim); }
  .pk-form { padding: 16px; display: flex; flex-direction: column; gap: 14px; }
  .pk-form label { display: flex; flex-direction: column; gap: 4px; font-size: 0.8125rem; }
  .pk-form label > span { color: var(--p-text-dim); font-size: 0.6875rem; text-transform: uppercase; letter-spacing: 0.08em; }
  .pk-form select, .pk-form input { background: var(--p-surface2); border: 1px solid var(--p-border); color: var(--p-text); padding: 8px 10px; border-radius: 4px; font-family: var(--p-mono); font-size: 0.8125rem; }
  .pk-strat-desc { font-size: 0.75rem; color: var(--p-text-dim); line-height: 1.5; margin: 0; }
  .pk-strat-regime { font-size: 0.6875rem; color: var(--p-accent); margin: 0; }
  .pk-run { background: var(--p-accent); color: #000; border: none; padding: 10px; border-radius: 4px; font-weight: 600; cursor: pointer; font-family: var(--p-mono); letter-spacing: 0.06em; text-transform: uppercase; font-size: 0.75rem; }
  .pk-run:hover:not(:disabled) { filter: brightness(1.1); }
  .pk-run:disabled { opacity: 0.5; cursor: wait; }
  .pk-disclaimer { font-size: 0.625rem; color: var(--p-text-dim); line-height: 1.5; margin: 0; opacity: 0.7; }
  .pk-error { background: rgba(255, 80, 80, 0.12); border: 1px solid rgba(255, 80, 80, 0.3); color: #ff9090; padding: 10px 14px; border-radius: 4px; font-size: 0.8125rem; margin-bottom: 16px; }
  .pk-empty { flex: 1; display: grid; place-items: center; color: var(--p-text-dim); font-size: 0.875rem; }
  .pk-verdict { padding: 14px 16px; font-family: var(--p-mono); font-size: 0.875rem; font-weight: 600; letter-spacing: 0.08em; text-align: center; }
  .pk-verdict.positive { background: rgba(0, 220, 130, 0.1); color: #34d399; border-bottom: 1px solid rgba(52, 211, 153, 0.2); }
  .pk-verdict.negative { background: rgba(255, 100, 100, 0.1); color: #f87171; border-bottom: 1px solid rgba(248, 113, 113, 0.2); }
  .pk-metrics { display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 1px; background: var(--p-border); }
  .pk-metric { background: var(--p-surface1); padding: 14px 12px; display: flex; flex-direction: column; gap: 4px; }
  .pk-metric-label { font-size: 0.625rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--p-text-dim); }
  .pk-metric-value { font-family: var(--p-mono); font-size: 1.125rem; font-weight: 600; color: var(--p-text); }
  .pk-chart { padding: 16px; }
  .pk-chart svg { width: 100%; height: 200px; }
  .pk-chart-labels { display: flex; justify-content: space-between; font-family: var(--p-mono); font-size: 0.6875rem; color: var(--p-text-dim); margin-top: 6px; }
  .pk-message { padding: 12px 16px; font-size: 0.8125rem; color: var(--p-text-dim); line-height: 1.5; border-top: 1px solid var(--p-border); }
  @media (max-width: 900px) { .pk-grid { grid-template-columns: 1fr; } }
</style>
