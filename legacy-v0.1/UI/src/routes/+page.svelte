<script lang="ts">
  // Prismatik workbench · Author: Aaron Stovall · 0.1.0
  // Walk-forward validation is the primary action by design; the plain
  // backtest is deliberately secondary. The product leads with the honest
  // number.
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { startJob, pollJob, fmtUsd } from '$lib/api';
  import type { RunResult, StrategySpec } from '$lib/types';
  import VerdictPlate from '$lib/components/VerdictPlate.svelte';
  import Chart from '$lib/components/Chart.svelte';
  import MetricsGrid from '$lib/components/MetricsGrid.svelte';
  import Honesty from '$lib/components/Honesty.svelte';
  import SessionPanel from '$lib/components/SessionPanel.svelte';

  let symbol = 'BTC-USD';
  let granularityS = 3600;
  let days = 180;
  let equity = 100;
  let kind = 'ma';
  let fast = 24;
  let slow = 96;
  let lookback = 48;
  let period = 14;
  let entry = 30;
  let useVolTarget = false;
  let volTarget = 0.3;
  let folds = 5;

  type Phase = 'idle' | 'running' | 'done' | 'error';
  let phase: Phase = 'idle';
  let runningWhat = '';
  let result: RunResult | null = null;
  let error = '';

  let alpacaConfigured = false;
  let liveUnlocked = false;
  let ackPhrase = '';

  onMount(async () => {
    try {
      const resp = await fetch('/api/v1/meta', {
        headers: { Authorization: `Bearer ${window.PRISMATIK?.token ?? ''}` }
      });
      if (resp.ok) {
        const meta = await resp.json();
        alpacaConfigured = Boolean(meta.alpaca_configured);
        liveUnlocked = Boolean(meta.live_unlocked);
        ackPhrase = String(meta.live_ack_phrase_required ?? '');
      }
    } catch {
      /* meta is advisory; the workbench still functions */
    }
  });

  function spec(): StrategySpec {
    const params: Record<string, number> =
      kind === 'ma' ? { fast, slow }
      : kind === 'donchian' ? { lookback }
      : kind === 'rsi' ? { period, entry, exit: 55 }
      : {};
    return { kind, params, vol_target: useVolTarget ? volTarget : null };
  }

  async function run(which: 'walkforward' | 'backtest'): Promise<void> {
    phase = 'running';
    runningWhat = which === 'walkforward' ? 'Validating out of sample' : 'Backtesting';
    error = '';
    result = null;
    try {
      const jobId = await startJob(which, {
        symbol,
        granularity_s: granularityS,
        days,
        initial_equity_usd: equity,
        strategy: spec(),
        folds
      });
      result = await pollJob(jobId);
      phase = 'done';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      phase = 'error';
    }
  }

  $: positive = result
    ? result.kind === 'backtest'
      ? result.beats_benchmark
      : result.oos_metrics.total_return > 0
    : false;
</script>

<div class="frame">
  <aside class="rail">
    <header>
      <h1>Prismatik</h1>
      <p class="mono tagline">Mythos Systems · v{typeof window !== 'undefined' ? window.PRISMATIK?.version : ''}</p>
    </header>

    <label class="field"><span>Instrument</span>
      <input bind:value={symbol} placeholder="BTC-USD" spellcheck="false" />
    </label>
    <div class="pair">
      <label class="field"><span>Bar</span>
        <select bind:value={granularityS}>
          <option value={900}>15 min</option>
          <option value={3600}>1 hour</option>
          <option value={21600}>6 hour</option>
          <option value={86400}>1 day</option>
        </select>
      </label>
      <label class="field"><span>History (days)</span>
        <input type="number" bind:value={days} min="7" max="1500" />
      </label>
    </div>
    <label class="field"><span>Starting capital</span>
      <input type="number" bind:value={equity} min="1" step="10" />
    </label>

    <label class="field"><span>Strategy</span>
      <select bind:value={kind}>
        <option value="ma">Moving average crossover</option>
        <option value="donchian">Donchian breakout</option>
        <option value="rsi">RSI mean reversion</option>
        <option value="hold">Buy and hold</option>
      </select>
    </label>
    {#if kind === 'ma'}
      <div class="pair" transition:fade={{ duration: 120 }}>
        <label class="field"><span>Fast</span><input type="number" bind:value={fast} min="2" /></label>
        <label class="field"><span>Slow</span><input type="number" bind:value={slow} min="3" /></label>
      </div>
    {:else if kind === 'donchian'}
      <label class="field" transition:fade={{ duration: 120 }}><span>Lookback</span>
        <input type="number" bind:value={lookback} min="2" />
      </label>
    {:else if kind === 'rsi'}
      <div class="pair" transition:fade={{ duration: 120 }}>
        <label class="field"><span>Period</span><input type="number" bind:value={period} min="2" /></label>
        <label class="field"><span>Entry RSI</span><input type="number" bind:value={entry} min="1" max="50" /></label>
      </div>
    {/if}

    <label class="check">
      <input type="checkbox" bind:checked={useVolTarget} />
      <span>Volatility-targeted sizing</span>
    </label>
    {#if useVolTarget}
      <label class="field" transition:fade={{ duration: 120 }}><span>Target annual vol</span>
        <input type="number" bind:value={volTarget} min="0.05" max="1" step="0.05" />
      </label>
    {/if}
    <label class="field"><span>Walk-forward folds</span>
      <input type="number" bind:value={folds} min="2" max="12" />
    </label>

    <button class="btn btn-primary" on:click={() => run('walkforward')} disabled={phase === 'running'}>
      Validate out of sample
    </button>
    <button class="btn btn-quiet" on:click={() => run('backtest')} disabled={phase === 'running'}>
      Plain backtest
    </button>
    <p class="mono note">
      The primary action is the honest one. A plain backtest flatters; the walk-forward
      is the number that has to survive contact with unseen data.
    </p>
  </aside>

  <main class="stage">
    {#if phase === 'idle'}
      <div class="empty" in:fade>
        <p class="mono">No verdict yet.</p>
        <p>Configure an instrument and strategy, then validate it out of sample.
           Results net of {fmtUsd(0.65).replace('$0.65', '0.65%')} round-trip costs by default.</p>
      </div>
    {:else if phase === 'running'}
      <div class="empty" in:fade>
        <div class="scriber" aria-hidden="true"></div>
        <p class="mono" role="status">{runningWhat} — fetching real venue data…</p>
      </div>
    {:else if phase === 'error'}
      <div class="empty" in:fade>
        <p class="neg mono" role="alert">{error}</p>
        <p class="mono dim">Nothing was fabricated in place of missing data. Adjust and rerun.</p>
      </div>
    {:else if result}
      <div class="results" in:fade={{ duration: 250 }}>
        <VerdictPlate verdict={result.verdict} {positive} />
        {#if result.kind === 'backtest'}
          <Chart series={result.equity} ghost={result.benchmark_equity}
                 label="Strategy (gilt) vs buy and hold (ghost) · {result.bars} bars · net of costs" />
          <MetricsGrid metrics={result.metrics} benchmark={result.benchmark_metrics} />
        {:else}
          <Chart series={result.oos_equity}
                 label="Stitched out-of-sample equity · {result.n_folds} folds · net of costs" />
          <MetricsGrid metrics={result.oos_metrics} />
          <Honesty {result} />
        {/if}
      </div>
    {/if}

    <SessionPanel {symbol} {granularityS} strategy={spec()} {equity}
                  {alpacaConfigured} {liveUnlocked} {ackPhrase} />
  </main>
</div>

<style>
  .frame {
    display: grid;
    grid-template-columns: 320px 1fr;
    gap: 28px;
    max-width: 1280px;
    margin: 0 auto;
    padding: 28px 24px 64px;
  }
  @media (max-width: 900px) { .frame { grid-template-columns: 1fr; } }
  .rail { display: grid; gap: 13px; align-content: start; }
  h1 {
    font-family: var(--display);
    font-weight: 300;
    font-size: 34px;
    letter-spacing: 0.02em;
    margin: 0;
    color: var(--porcelain);
  }
  h1::after { content: '.'; color: var(--gilt); }
  .tagline { color: var(--porcelain-2); font-size: 11px; letter-spacing: 0.2em; text-transform: uppercase; margin: 4px 0 8px; }
  .field span {
    display: block; font-family: var(--mono); font-size: 11px;
    letter-spacing: 0.18em; text-transform: uppercase;
    color: var(--porcelain-2); margin-bottom: 5px;
  }
  .pair { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .check { display: flex; gap: 9px; align-items: center; font-size: 13.5px; }
  .check input { width: auto; }
  .note { font-size: 11.5px; color: var(--porcelain-2); line-height: 1.5; }
  .stage { display: grid; gap: 20px; align-content: start; }
  .results { display: grid; gap: 20px; }
  .empty {
    border: 1px dashed var(--gilt-dim);
    border-radius: var(--radius);
    padding: 56px 28px;
    text-align: center;
    color: var(--porcelain-2);
    display: grid;
    gap: 8px;
    justify-items: center;
  }
  .scriber {
    width: 120px; height: 1px; background: var(--gilt);
    animation: scribe 1.1s ease-in-out infinite alternate;
    transform-origin: left;
  }
  @keyframes scribe { from { transform: scaleX(0.15); opacity: 0.5; } to { transform: scaleX(1); opacity: 1; } }
  .dim { color: var(--porcelain-2); }
</style>
