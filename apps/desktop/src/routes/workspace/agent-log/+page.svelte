<script lang="ts">
  /**
   * The agent log — what the system did, and why.
   *
   * Autonomy without a legible record is just software you have to trust. This
   * surface exists so nothing the desk does is only knowable from the code:
   * every decision carries the evidence behind it, every abstain states its
   * reason, and outcomes are attached after the fact rather than claimed up
   * front.
   *
   * Abstains are shown alongside trades deliberately. A log that only recorded
   * the positions taken would flatter the system by hiding every time it
   * declined and was right — or declined and was wrong.
   */
  import { onMount } from 'svelte';
  import { RefreshCw, Loader2 } from 'lucide-svelte';
  import NoData from '$lib/prismatik/NoData.svelte';
  import { NO_VALUE } from '$lib/prismatik/market.svelte';

  interface DecisionRecord {
    id: string;
    symbol: string;
    source: string;
    action: string;
    rationale: string;
    regime: string | null;
    edgePpm: number;
    conviction: number;
    decidedPrice: number;
    decidedAt: string;
    resolvesAfter: string;
    resolvedPrice: number | null;
    realizedBps: number | null;
    correct: boolean | null;
  }

  interface RegimeTrackRecord {
    regime: string;
    decided: number;
    resolved: number;
    correct: number;
    meanRealizedBps: number;
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

  type Tab = 'decisions' | 'record' | 'ticks';

  let tab = $state<Tab>('decisions');
  let decisions = $state<DecisionRecord[]>([]);
  let trackRecord = $state<RegimeTrackRecord[]>([]);
  let scheduler = $state<SchedulerStatus | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke: call, isTauri } = await import('@tauri-apps/api/core');
    if (!isTauri()) throw new Error('desktop runtime required');
    return call<T>(command, args);
  }

  async function load(): Promise<void> {
    loading = true;
    error = null;
    try {
      const [rows, record, status] = await Promise.all([
        invoke<DecisionRecord[]>('list_decision_memory', { symbol: null }),
        invoke<RegimeTrackRecord[]>('decision_track_record'),
        invoke<SchedulerStatus>('autonomy_loop_status'),
      ]);
      decisions = rows;
      trackRecord = record;
      scheduler = status;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  /** Attach outcomes to decisions whose horizon has elapsed. */
  async function resolveOutcomes(): Promise<void> {
    loading = true;
    try {
      await invoke<number>('resolve_decision_memory');
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      loading = false;
    }
  }

  const pending = $derived(decisions.filter((d) => d.resolvedPrice === null).length);
  const scored = $derived(decisions.filter((d) => d.correct !== null));
  const hitRate = $derived(
    scored.length === 0
      ? null
      : (scored.filter((d) => d.correct === true).length / scored.length) * 100,
  );

  function edge(ppm: number): string {
    const pp = ppm / 10_000;
    return `${pp >= 0 ? '+' : '−'}${Math.abs(pp).toFixed(1)}pp`;
  }

  function when(iso: string): string {
    const d = new Date(iso);
    return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
  }

  onMount(() => {
    void load();
  });
</script>

<svelte:head><title>Agent log · PRISMATIK</title></svelte:head>

<main class="pk-log">
  <header>
    <div>
      <p class="eyebrow">TRANSPARENCY</p>
      <h1>Agent log</h1>
      <p class="lede">
        Every judgement the desk made, the evidence behind it, and what actually happened. Abstains
        are recorded alongside trades — a log of only the positions taken would hide every time the
        system declined and was right.
      </p>
    </div>
    <div class="actions">
      <button onclick={() => void load()} disabled={loading}>
        {#if loading}<span class="spin"><Loader2 size={13} /></span>{:else}<RefreshCw size={13} />{/if}
        Refresh
      </button>
      <button onclick={() => void resolveOutcomes()} disabled={loading}>Resolve due</button>
    </div>
  </header>

  <section class="metrics">
    <article>
      <span>DECISIONS</span><b>{decisions.length}</b><small>{pending} awaiting outcome</small>
    </article>
    <article>
      <span>DIRECTIONAL HIT RATE</span>
      <b>{hitRate === null ? NO_VALUE : `${hitRate.toFixed(0)}%`}</b>
      <small>{scored.length} scored · abstains excluded</small>
    </article>
    <article>
      <span>LOOP</span>
      <b class:on={scheduler?.running}>{scheduler?.running ? 'RUNNING' : 'STOPPED'}</b>
      <small>{scheduler?.message ?? NO_VALUE}</small>
    </article>
    <article>
      <span>MODE</span><b>{(scheduler?.mode ?? '—').toUpperCase()}</b>
      <small>consequence permitted</small>
    </article>
  </section>

  <nav class="tabs">
    <button class:active={tab === 'decisions'} onclick={() => (tab = 'decisions')}>
      Decisions ({decisions.length})
    </button>
    <button class:active={tab === 'record'} onclick={() => (tab = 'record')}>
      Track record by regime
    </button>
    <button class:active={tab === 'ticks'} onclick={() => (tab = 'ticks')}>
      Loop ticks ({scheduler?.recentTicks.length ?? 0})
    </button>
  </nav>

  {#if error}<p class="error">{error}</p>{/if}

  {#if tab === 'decisions'}
    {#if decisions.length === 0}
      <NoData
        title="No decisions recorded"
        detail="The trader records every evaluation — including abstains — once the loop runs."
      />
    {:else}
      <div class="table">
        <div class="tr th">
          <span>When</span><span>Symbol</span><span>Action</span><span>Regime</span>
          <span>Edge</span><span>Size x</span><span>Outcome</span><span>Rationale</span>
        </div>
        {#each decisions as d (d.id)}
          <div class="tr">
            <span class="dim mono">{when(d.decidedAt)}</span>
            <b>{d.symbol}</b>
            <span class="action" data-action={d.action}>{d.action}</span>
            <span class="dim">{d.regime ?? NO_VALUE}</span>
            <span class="mono">{edge(d.edgePpm)}</span>
            <span class="mono dim">{d.conviction.toFixed(2)}</span>
            <span class="mono">
              {#if d.realizedBps === null}
                <span class="dim">pending</span>
              {:else if d.correct === null}
                <span class="dim">{d.realizedBps.toFixed(0)}bps</span>
              {:else}
                <span class:up={d.correct} class:down={!d.correct}>
                  {d.realizedBps >= 0 ? '+' : ''}{d.realizedBps.toFixed(0)}bps
                  {d.correct ? '✓' : '✗'}
                </span>
              {/if}
            </span>
            <span class="rationale" title={d.rationale}>{d.rationale}</span>
          </div>
        {/each}
      </div>
    {/if}
  {:else if tab === 'record'}
    {#if trackRecord.length === 0}
      <NoData title="No track record yet" detail="Accrues as decisions resolve." />
    {:else}
      <div class="table">
        <div class="tr th record">
          <span>Regime</span><span>Decided</span><span>Resolved</span><span>Correct</span>
          <span>Hit rate</span><span>Mean move</span>
        </div>
        {#each trackRecord as r (r.regime)}
          <div class="tr record">
            <b>{r.regime}</b>
            <span class="mono">{r.decided}</span>
            <span class="mono">{r.resolved}</span>
            <span class="mono">{r.correct}</span>
            <span class="mono">
              {r.resolved === 0 ? NO_VALUE : `${((r.correct / r.resolved) * 100).toFixed(0)}%`}
            </span>
            <span class="mono" class:up={r.meanRealizedBps >= 0} class:down={r.meanRealizedBps < 0}>
              {r.resolved === 0 ? NO_VALUE : `${r.meanRealizedBps.toFixed(0)}bps`}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  {:else if !scheduler || scheduler.recentTicks.length === 0}
    <NoData
      title="No loop ticks"
      detail="Start the autonomy loop from the Trader surface to see its activity here."
    />
  {:else}
    <div class="table">
      <div class="tr th ticks">
        <span>When</span><span>State</span><span>Submitted</span><span>Skipped</span>
        <span>Ladder</span><span>Message</span>
      </div>
      {#each scheduler.recentTicks as t, i (t.at + i)}
        <div class="tr ticks">
          <span class="dim mono">{when(t.at)}</span>
          <span class:up={t.ok} class:down={!t.ok}>{t.ok ? 'ok' : 'fail'}</span>
          <span class="mono">{t.ordersSubmitted}</span>
          <span class="mono">{t.ordersSkipped}</span>
          <span class="dim mono">{t.ladderLevel || NO_VALUE}</span>
          <span class="rationale" title={t.message}>{t.message}</span>
        </div>
      {/each}
    </div>
  {/if}
</main>

<style>
  .pk-log {
    height: 100%;
    overflow: auto;
    padding: 28px clamp(18px, 3vw, 40px) 60px;
    color: var(--p-text);
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 22px;
  }
  .eyebrow {
    margin: 0;
    color: var(--p-accent);
    font: 700 9px var(--font-mono);
    letter-spacing: 0.18em;
  }
  h1 {
    margin: 8px 0 6px;
    font-size: clamp(1.8rem, 3vw, 2.6rem);
    letter-spacing: -0.04em;
  }
  .lede {
    max-width: 76ch;
    margin: 0;
    color: var(--p-dim);
    line-height: 1.55;
  }
  .actions {
    display: flex;
    height: fit-content;
    gap: 8px;
  }
  .actions button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border: 1px solid var(--p-border);
    border-radius: 6px;
    background: var(--p-panel-fill);
    color: var(--p-accent);
    cursor: pointer;
    font: 700 0.62rem var(--font-mono);
    letter-spacing: 0.08em;
    white-space: nowrap;
  }
  .actions button:disabled {
    cursor: default;
    opacity: 0.6;
  }
  .spin {
    display: inline-flex;
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }

  .metrics {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 8px;
    margin: 18px 0 14px;
  }
  .metrics article {
    display: grid;
    gap: 5px;
    padding: 13px;
    border: 1px solid var(--p-border);
    border-radius: 9px;
    background: var(--p-panel-fill);
  }
  .metrics span {
    color: var(--p-dim);
    font: 600 8px var(--font-mono);
    letter-spacing: 0.1em;
  }
  .metrics b {
    font: 700 1.05rem var(--font-mono);
  }
  .metrics b.on {
    color: var(--p-up);
  }
  .metrics small {
    color: var(--p-dim);
    font-size: 0.62rem;
  }

  .tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 12px;
  }
  .tabs button {
    padding: 7px 13px;
    border: 1px solid var(--p-border);
    border-radius: 6px;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
    font: 600 0.66rem var(--font-mono);
  }
  .tabs button.active {
    border-color: var(--p-accent);
    background: color-mix(in srgb, var(--p-accent) 10%, transparent);
    color: var(--p-accent);
  }

  .error {
    margin: 0 0 12px;
    padding: 10px 12px;
    border: 1px solid color-mix(in srgb, var(--p-down) 45%, transparent);
    border-radius: 7px;
    color: var(--p-down);
    font-size: 0.75rem;
  }

  .table {
    overflow: hidden;
    border: 1px solid var(--p-border);
    border-radius: 9px;
    background: var(--p-panel-fill);
  }
  .tr {
    display: grid;
    grid-template-columns: 150px 80px 76px 130px 74px 66px 118px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
    border-bottom: 1px solid var(--p-grid);
    font-size: 0.74rem;
  }
  .tr.record {
    grid-template-columns: 180px 90px 90px 90px 90px minmax(0, 1fr);
  }
  .tr.ticks {
    grid-template-columns: 150px 70px 90px 80px 100px minmax(0, 1fr);
  }
  .th {
    position: sticky;
    top: 0;
    background: var(--p-surface);
    color: var(--p-dim);
    font: 600 8px var(--font-mono);
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .dim {
    color: var(--p-dim);
  }
  .up {
    color: var(--p-up);
  }
  .down {
    color: var(--p-down);
  }
  .rationale {
    overflow: hidden;
    color: var(--p-dim);
    font-size: 0.68rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .action {
    font: 700 0.62rem var(--font-mono);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .action[data-action='long'] {
    color: var(--p-up);
  }
  .action[data-action='short'] {
    color: var(--p-down);
  }
  .action[data-action='abstain'],
  .action[data-action='advise'] {
    color: var(--p-dim);
  }
</style>
