<script lang="ts">
  /**
   * Instrument discovery.
   *
   * The rest of the terminal can only measure what you already thought to
   * track. This sweeps thousands of instruments and runs the same regime and
   * edge measurement over the top candidates, so the universe stops being a
   * hand-picked — and therefore biased — list.
   *
   * The source is unofficial and its prices rank candidates only. Nothing here
   * becomes evidence until the instrument is tracked and quoted by a provider
   * the app has terms with, which is why the screener price column is labelled
   * separately from everything the analysis produces.
   */
  import { onMount } from 'svelte';
  import { Search, Plus, Check, Loader2, TriangleAlert } from 'lucide-svelte';
  import { market, fmtCompact, fmtNum, NO_VALUE } from '$lib/prismatik/market.svelte';
  import NoData from '$lib/prismatik/NoData.svelte';

  interface ScreenedInstrument {
    symbol: string;
    qualified: string;
    exchange: string;
    screenerPrice: number;
    changePct: number;
    volume: number;
    marketCap: number | null;
    tracked: boolean;
    regime: string | null;
    edgePpm: number | null;
    actionable: boolean | null;
    note: string;
  }

  interface ScreenerResult {
    market: string;
    hits: ScreenedInstrument[];
    totalCount: number;
    analysed: number;
    undecodableRows: number;
    retrievedAt: string;
    message: string;
  }

  interface UnofficialSource {
    id: string;
    label: string;
    caveat: string;
    enabled: boolean;
  }

  const MARKETS = [
    { id: 'america', label: 'US equities' },
    { id: 'crypto', label: 'Crypto' },
    { id: 'forex', label: 'Forex' },
  ];

  const SORTS = [
    { id: 'turnover', label: 'Turnover' },
    { id: 'gainers', label: 'Gainers' },
    { id: 'losers', label: 'Losers' },
    { id: 'market_cap', label: 'Market cap' },
  ];

  const REGIME_LABELS: Record<string, string> = {
    calm_trending: 'Calm trend',
    calm_mean_revert: 'Calm revert',
    volatile_trending: 'Vol trend',
    volatile_mean_revert: 'Vol revert',
    crisis: 'Crisis',
  };

  let marketId = $state('america');
  let sort = $state('turnover');
  let limit = $state(50);
  let minTurnover = $state(50_000_000);
  let minPrice = $state(1);
  let analyse = $state(true);

  let result = $state<ScreenerResult | null>(null);
  let sources = $state<UnofficialSource[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let trackingBusy = $state<string | null>(null);

  const source = $derived(sources.find((s) => s.id === 'tradingview-screener') ?? null);
  const enabled = $derived(source?.enabled ?? false);

  async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke: call, isTauri } = await import('@tauri-apps/api/core');
    if (!isTauri()) throw new Error('desktop runtime required');
    return call<T>(command, args);
  }

  async function loadSources(): Promise<void> {
    try {
      sources = await invoke<UnofficialSource[]>('list_unofficial_sources');
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function toggleSource(next: boolean): Promise<void> {
    try {
      sources = await invoke<UnofficialSource[]>('set_unofficial_source', {
        source: 'tradingview-screener',
        enabled: next,
      });
      if (!next) result = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function run(): Promise<void> {
    loading = true;
    error = null;
    try {
      result = await invoke<ScreenerResult>('screen_instruments', {
        request: {
          market: marketId,
          sort,
          limit,
          minTurnover,
          minPrice,
          analyse,
        },
      });
    } catch (e) {
      result = null;
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function track(hit: ScreenedInstrument): Promise<void> {
    trackingBusy = hit.qualified;
    try {
      await market.track({
        kind: marketId === 'crypto' ? 'crypto' : 'equity',
        symbol: hit.symbol,
        // Providers key on the bare ticker, not the venue-qualified id.
        providerId: hit.symbol,
        name: hit.qualified,
        market: hit.exchange,
        decimals: marketId === 'crypto' ? 4 : 2,
        tracked: true,
      });
      if (result) {
        result.hits = result.hits.map((row) =>
          row.qualified === hit.qualified ? { ...row, tracked: true } : row,
        );
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      trackingBusy = null;
    }
  }

  function edgeLabel(ppm: number | null): string {
    if (ppm === null) return NO_VALUE;
    const pp = ppm / 10_000;
    return `${pp >= 0 ? '+' : '−'}${Math.abs(pp).toFixed(1)}pp`;
  }

  onMount(() => {
    void loadSources();
  });
</script>

<svelte:head><title>Screener · PRISMATIK</title></svelte:head>

<main class="pk-screener">
  <header>
    <div>
      <p class="eyebrow">INSTRUMENT DISCOVERY</p>
      <h1>Screener</h1>
      <p class="lede">
        Sweep a market, then measure the top candidates with the same regime and edge analysis the
        trader sizes on. Screener prices rank candidates; they are never cited as observations.
      </p>
    </div>
  </header>

  <section class="consent" class:on={enabled}>
    <span class="consent-icon"><TriangleAlert size={16} /></span>
    <div>
      <strong>{source?.label ?? 'TradingView screener'} — unofficial source</strong>
      <p>{source?.caveat ?? 'Loading source policy…'}</p>
    </div>
    <button class="toggle" class:on={enabled} onclick={() => void toggleSource(!enabled)}>
      {enabled ? 'Enabled' : 'Enable'}
    </button>
  </section>

  {#if enabled}
    <section class="controls">
      <label>
        <span>Market</span>
        <select bind:value={marketId}>
          {#each MARKETS as m (m.id)}<option value={m.id}>{m.label}</option>{/each}
        </select>
      </label>
      <label>
        <span>Rank by</span>
        <select bind:value={sort}>
          {#each SORTS as s (s.id)}<option value={s.id}>{s.label}</option>{/each}
        </select>
      </label>
      <label>
        <span>Min turnover</span>
        <input type="number" min="0" step="1000000" bind:value={minTurnover} />
      </label>
      <label>
        <span>Min price</span>
        <input type="number" min="0" step="0.5" bind:value={minPrice} />
      </label>
      <label>
        <span>Rows</span>
        <input type="number" min="1" max="200" bind:value={limit} />
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={analyse} />
        <span>Measure top hits</span>
      </label>
      <button class="run" onclick={() => void run()} disabled={loading}>
        {#if loading}<span class="spin"><Loader2 size={13} /></span>{:else}<Search size={13} />{/if}
        {loading ? 'Sweeping' : 'Run sweep'}
      </button>
    </section>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    {#if result}
      <p class="note">{result.message}</p>
      <div class="table">
        <div class="tr th">
          <span>Symbol</span><span>Venue</span><span>Screener price</span><span>Change</span>
          <span>Volume</span><span>Regime</span><span>Edge</span><span>Assessment</span><span></span>
        </div>
        {#each result.hits as hit (hit.qualified)}
          <div class="tr" class:actionable={hit.actionable === true}>
            <b>{hit.symbol}</b>
            <span class="dim">{hit.exchange}</span>
            <span class="mono">{fmtNum(hit.screenerPrice, 4)}</span>
            <span class="mono" class:up={hit.changePct >= 0} class:down={hit.changePct < 0}>
              {hit.changePct >= 0 ? '+' : ''}{hit.changePct.toFixed(2)}%
            </span>
            <span class="mono dim">{fmtCompact(hit.volume)}</span>
            <span class="regime" data-regime={hit.regime ?? 'none'}>
              {hit.regime ? (REGIME_LABELS[hit.regime] ?? hit.regime) : NO_VALUE}
            </span>
            <span class="mono">{edgeLabel(hit.edgePpm)}</span>
            <span class="note-cell" title={hit.note}>{hit.note}</span>
            <span>
              {#if hit.tracked}
                <span class="tracked"><Check size={12} /> Tracked</span>
              {:else}
                <button
                  class="track"
                  disabled={trackingBusy === hit.qualified || marketId === 'forex'}
                  title={marketId === 'forex'
                    ? 'No quote provider is wired for forex'
                    : `Track ${hit.symbol}`}
                  onclick={() => void track(hit)}
                >
                  <Plus size={12} /> Track
                </button>
              {/if}
            </span>
          </div>
        {/each}
      </div>
    {:else if !loading}
      <NoData
        title="No sweep run yet"
        detail="Choose a market and run a sweep. Measuring the top hits fetches real provider history for each, so it takes a moment."
      />
    {/if}
  {:else}
    <NoData
      title="Screener disabled"
      detail="This source is unofficial and off by default. Enable it above to sweep for candidates."
    />
  {/if}
</main>

<style>
  .pk-screener {
    height: 100%;
    overflow: auto;
    padding: 28px clamp(18px, 3vw, 40px) 60px;
    color: var(--p-text);
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
    max-width: 74ch;
    margin: 0;
    color: var(--p-dim);
    line-height: 1.55;
  }

  .consent {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    margin: 18px 0 14px;
    padding: 12px 14px;
    border: 1px solid color-mix(in srgb, #f59e0b 40%, var(--p-border));
    border-radius: 9px;
    background: color-mix(in srgb, #f59e0b 6%, transparent);
  }
  .consent.on {
    border-color: color-mix(in srgb, var(--p-up) 40%, var(--p-border));
    background: color-mix(in srgb, var(--p-up) 6%, transparent);
  }
  .consent-icon {
    display: flex;
    color: #f59e0b;
  }
  .consent.on .consent-icon {
    color: var(--p-up);
  }
  .consent strong {
    font: 700 0.72rem var(--font-mono);
    letter-spacing: 0.06em;
  }
  .consent p {
    margin: 4px 0 0;
    color: var(--p-dim);
    font-size: 0.72rem;
    line-height: 1.5;
  }
  .toggle {
    padding: 7px 14px;
    border: 1px solid var(--p-border);
    border-radius: 6px;
    background: var(--p-panel-fill);
    color: var(--p-text);
    cursor: pointer;
    font: 700 0.62rem var(--font-mono);
    letter-spacing: 0.1em;
  }
  .toggle.on {
    border-color: var(--p-up);
    color: var(--p-up);
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 10px;
    margin-bottom: 14px;
  }
  .controls label {
    display: grid;
    gap: 4px;
  }
  .controls label > span {
    color: var(--p-dim);
    font: 600 8px var(--font-mono);
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .controls select,
  .controls input[type='number'] {
    min-width: 120px;
    padding: 7px 9px;
    border: 1px solid var(--p-border);
    border-radius: 6px;
    background: var(--p-panel-fill);
    color: var(--p-text);
    font-family: var(--font-mono);
    font-size: 0.75rem;
  }
  .controls .check {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-bottom: 8px;
  }
  .controls .check span {
    font: 600 0.68rem var(--font-mono);
    letter-spacing: 0.04em;
    text-transform: none;
  }
  .run {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 8px 14px;
    border: 1px solid var(--p-accent);
    border-radius: 6px;
    background: color-mix(in srgb, var(--p-accent) 12%, transparent);
    color: var(--p-accent);
    cursor: pointer;
    font: 700 0.66rem var(--font-mono);
    letter-spacing: 0.08em;
  }
  .run:disabled {
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

  .error {
    margin: 0 0 12px;
    padding: 10px 12px;
    border: 1px solid color-mix(in srgb, var(--p-down) 45%, transparent);
    border-radius: 7px;
    color: var(--p-down);
    font-size: 0.75rem;
  }
  .note {
    margin: 0 0 10px;
    color: var(--p-dim);
    font-size: 0.72rem;
  }

  .table {
    overflow: hidden;
    border: 1px solid var(--p-border);
    border-radius: 9px;
    background: var(--p-panel-fill);
  }
  .tr {
    display: grid;
    grid-template-columns: 90px 90px 120px 90px 90px 110px 80px minmax(0, 1fr) 96px;
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
    border-bottom: 1px solid var(--p-grid);
    font-size: 0.74rem;
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
  .tr.actionable {
    background: color-mix(in srgb, var(--p-up) 7%, transparent);
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
  .note-cell {
    overflow: hidden;
    color: var(--p-dim);
    font-size: 0.68rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .regime {
    font: 600 0.66rem var(--font-mono);
  }
  .regime[data-regime='calm_trending'] {
    color: #34d399;
  }
  .regime[data-regime='calm_mean_revert'] {
    color: #4dc8ff;
  }
  .regime[data-regime='volatile_trending'] {
    color: #fbbf24;
  }
  .regime[data-regime='volatile_mean_revert'] {
    color: #f87171;
  }
  .regime[data-regime='crisis'] {
    color: #ef4444;
  }
  .regime[data-regime='none'] {
    color: var(--p-dim);
  }
  .track,
  .tracked {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font: 600 0.64rem var(--font-mono);
  }
  .track {
    padding: 4px 9px;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    background: transparent;
    color: var(--p-accent);
    cursor: pointer;
  }
  .track:hover:not(:disabled) {
    border-color: var(--p-accent);
  }
  .track:disabled {
    cursor: default;
    opacity: 0.4;
  }
  .tracked {
    color: var(--p-up);
  }
</style>
