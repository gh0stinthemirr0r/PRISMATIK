<script lang="ts">
  // Prismatik workbench · Author: Aaron Stovall · 0.1.0
  // Walk-forward validation is the primary action by design; the plain
  // backtest is deliberately secondary. The product leads with the honest
  // number.
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { startJob, pollJob, fmtUsd, authHeaders } from '$lib/api';
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
  let metaState = 'loading';
  let serverState = 'loading';
  let barLabel = '1 hour';
  let strategyLabel = 'Moving average crossover';

  $: barLabel =
    granularityS === 900 ? '15 min'
    : granularityS === 3600 ? '1 hour'
    : granularityS === 21600 ? '6 hour'
    : '1 day';

  $: strategyLabel =
    kind === 'ma' ? 'Moving average crossover'
    : kind === 'donchian' ? 'Donchian breakout'
    : kind === 'rsi' ? 'RSI mean reversion'
    : 'Buy and hold';

  onMount(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.repeat) return;
      if (!(event.ctrlKey || event.metaKey)) return;
      const key = event.key.toLowerCase();
      if (key === 'enter') {
        event.preventDefault();
        void run('walkforward');
      } else if (event.shiftKey && key === 'b') {
        event.preventDefault();
        void run('backtest');
      }
    };

    window.addEventListener('keydown', onKeyDown);

    void (async () => {
      try {
        const [metaResp, statusResp] = await Promise.all([
          fetch('/api/v1/meta', { headers: authHeaders() }),
          fetch('/api/v1/status', { headers: authHeaders() })
        ]);
        if (metaResp.ok) {
          const meta = await metaResp.json();
          alpacaConfigured = Boolean(meta.alpaca_configured);
          liveUnlocked = Boolean(meta.live_unlocked);
          ackPhrase = String(meta.live_ack_phrase_required ?? '');
          metaState = 'ready';
        } else {
          metaState = `locked (${metaResp.status})`;
        }
        if (statusResp.ok) {
          const status = await statusResp.json();
          serverState = `${status.jobs_queued} jobs · ${status.sessions_open} sessions · ${status.uptime_seconds}s uptime`;
        } else {
          serverState = `status locked (${statusResp.status})`;
        }
      } catch {
        metaState = 'offline';
        serverState = 'offline';
      }
    })();

    return () => window.removeEventListener('keydown', onKeyDown);
  });

  function spec(): StrategySpec {
    const params: Record<string, number> =
      kind === 'ma' ? { fast, slow }
      : kind === 'donchian' ? { lookback }
      : kind === 'rsi' ? { period, entry, exit: 55 }
      : {};
    return { kind, params, vol_target: useVolTarget ? volTarget : null };
  }

  function applyPreset(name: 'conservative' | 'momentum'): void {
    if (name === 'conservative') {
      symbol = 'BTC-USD';
      granularityS = 3600;
      days = 180;
      equity = 100;
      kind = 'ma';
      fast = 24;
      slow = 96;
      lookback = 48;
      period = 14;
      entry = 30;
      useVolTarget = false;
      volTarget = 0.3;
      folds = 5;
      return;
    }

    symbol = 'ETH-USD';
    granularityS = 900;
    days = 365;
    equity = 250;
    kind = 'donchian';
    fast = 24;
    slow = 96;
    lookback = 72;
    period = 14;
    entry = 28;
    useVolTarget = true;
    volTarget = 0.2;
    folds = 6;
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
      <p class="mono meta">{metaState}</p>
    </header>

    <div class="strip">
      <div class="chip"><span>Instrument</span>{symbol}</div>
      <div class="chip"><span>Strategy</span>{strategyLabel}</div>
      <div class="chip"><span>Bar</span>{barLabel}</div>
      <div class="chip"><span>Server</span>{serverState}</div>
    </div>

    <div class="preset-row">
      <button class="btn btn-quiet small-btn" on:click={() => applyPreset('conservative')}>
        Conservative preset
      </button>
      <button class="btn btn-quiet small-btn" on:click={() => applyPreset('momentum')}>
        Momentum preset
      </button>
    </div>

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
    <p class="mono shortcuts">Shortcuts: Ctrl/⌘ + Enter = validate · Ctrl/⌘ + Shift + B = backtest</p>
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
  .meta {
    color: var(--gilt);
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    margin: 0 0 10px;
  }
  .strip {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
  }
  .chip {
    border: 1px solid var(--gilt-faint);
    border-radius: var(--radius);
    padding: 10px 11px;
    background: var(--ink-2);
    font-family: var(--mono);
    font-size: 11px;
    display: grid;
    gap: 4px;
  }
  .chip span {
    color: var(--porcelain-2);
    text-transform: uppercase;
    letter-spacing: 0.18em;
    font-size: 10px;
  }
  .preset-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .small-btn {
    padding: 10px 12px;
    font-size: 11px;
  }
  .field span {
    display: block; font-family: var(--mono); font-size: 11px;
    letter-spacing: 0.18em; text-transform: uppercase;
    color: var(--porcelain-2); margin-bottom: 5px;
  }
  .pair { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .check { display: flex; gap: 9px; align-items: center; font-size: 13.5px; }
  .check input { width: auto; }
  .note { font-size: 11.5px; color: var(--porcelain-2); line-height: 1.5; }
  .shortcuts {
    margin: -2px 0 0;
    font-size: 10.5px;
    color: var(--gilt);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    line-height: 1.4;
  }
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
