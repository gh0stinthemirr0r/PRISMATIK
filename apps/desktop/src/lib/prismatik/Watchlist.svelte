<script lang="ts">
  /**
   * The tracked-instrument surface — how a desk actually follows stock and crypto.
   *
   * Equities and crypto share one list because the user thinks in positions, not
   * in providers. Search hits both connected planes at once; the `kind` badge is
   * the only place the provider split is visible.
   */
  import { Plus, Search, X, Loader2 } from 'lucide-svelte';
  import {
    market,
    fmtPrice,
    fmtPct,
    fmtCompact,
    fmtAge,
    NO_VALUE,
    type Instrument,
    type InstrumentSearchHit,
    type InstrumentKind,
  } from './market.svelte';
  import Sparkline from './Sparkline.svelte';
  import NoData from './NoData.svelte';

  type Filter = 'all' | InstrumentKind;

  const FILTERS: { id: Filter; label: string }[] = [
    { id: 'all', label: 'All' },
    { id: 'equity', label: 'Equities' },
    { id: 'crypto', label: 'Crypto' },
  ];

  let filter = $state<Filter>('all');
  let adding = $state(false);
  let query = $state('');
  let searching = $state(false);
  let hits = $state<InstrumentSearchHit[]>([]);
  let searchMessage = $state('');
  let busyId = $state<string | null>(null);

  const rows = $derived(
    filter === 'all' ? market.instruments : market.instruments.filter((i) => i.kind === filter),
  );

  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  function onQueryInput(): void {
    clearTimeout(searchTimer);
    const term = query.trim();
    if (term.length < 1) {
      hits = [];
      searchMessage = '';
      return;
    }
    // Provider search is rate-budgeted upstream; debounce rather than key-per-request.
    searchTimer = setTimeout(() => void runSearch(term), 280);
  }

  async function runSearch(term: string): Promise<void> {
    searching = true;
    try {
      const result = await market.search(term);
      hits = result.hits;
      searchMessage = result.message;
    } catch (error) {
      hits = [];
      searchMessage = error instanceof Error ? error.message : String(error);
    } finally {
      searching = false;
    }
  }

  async function track(hit: InstrumentSearchHit): Promise<void> {
    busyId = hit.providerId;
    try {
      await market.track(hit);
      hits = hits.map((h) => (h.providerId === hit.providerId ? { ...h, tracked: true } : h));
    } catch (error) {
      searchMessage = error instanceof Error ? error.message : String(error);
    } finally {
      busyId = null;
    }
  }

  async function untrack(inst: Instrument): Promise<void> {
    busyId = inst.providerId;
    try {
      await market.untrack(inst);
    } finally {
      busyId = null;
    }
  }

  function closeAdd(): void {
    adding = false;
    query = '';
    hits = [];
    searchMessage = '';
  }
</script>

<aside class="pk-panel pk-track">
  <div class="pk-track-head">
    <div class="pk-tabs">
      {#each FILTERS as f (f.id)}
        <button class="pk-btn" class:active={filter === f.id} onclick={() => (filter = f.id)}>
          {f.label}
        </button>
      {/each}
    </div>
    <button
      class="pk-track-add"
      class:active={adding}
      onclick={() => (adding ? closeAdd() : (adding = true))}
      title="Track an instrument"
      aria-label="Track an instrument"
    >
      {#if adding}<X size={13} />{:else}<Plus size={13} />{/if}
    </button>
  </div>

  {#if adding}
    <div class="pk-track-search">
      <div class="pk-track-field">
        {#if searching}
          <span class="spin"><Loader2 size={12} /></span>
        {:else}
          <Search size={12} />
        {/if}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="text"
          placeholder="Symbol or name — AAPL, bitcoin, SPY…"
          bind:value={query}
          oninput={onQueryInput}
          autofocus
        />
      </div>
      {#if searchMessage}<p class="pk-track-msg">{searchMessage}</p>{/if}
      {#if hits.length > 0}
        <div class="pk-track-hits pk-scroll">
          {#each hits as hit (hit.kind + hit.providerId)}
            <button
              class="pk-track-hit"
              disabled={hit.tracked || busyId === hit.providerId}
              onclick={() => void track(hit)}
            >
              <span class="pk-kind" data-kind={hit.kind}>{hit.kind === 'crypto' ? 'C' : 'E'}</span>
              <span class="pk-track-hit-id">
                <b>{hit.symbol}</b>
                <small>{hit.name}</small>
              </span>
              <span class="pk-track-hit-venue">{hit.market}</span>
              {#if hit.tracked}
                <span class="pk-track-hit-state">Tracked</span>
              {:else}
                <Plus size={12} />
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <div class="pk-scroll pk-track-body">
    {#if !market.ready}
      <NoData title="Loading tracked instruments" compact />
    {:else if market.instruments.length === 0}
      <NoData
        title="Nothing tracked yet"
        detail="PRISMATIK quotes only what you choose to follow. Add an equity or a crypto asset to start the feed."
        action="Connect a market provider"
        href="/workspace/integrations"
      />
    {:else if rows.length === 0}
      <NoData title="No {filter} instruments tracked" compact />
    {:else}
      {#each rows as inst (inst.kind + inst.providerId)}
        <div
          class="pk-wl-row"
          class:selected={market.selectedSymbol === inst.symbol}
          class:unquoted={!inst.quoted}
          role="button"
          tabindex="0"
          onclick={() => market.select(inst.symbol)}
          onkeydown={(e) => e.key === 'Enter' && market.select(inst.symbol)}
        >
          <div class="pk-wl-id">
            <div class="pk-wl-sym">
              <span class="pk-kind" data-kind={inst.kind}>{inst.kind === 'crypto' ? 'C' : 'E'}</span>
              {inst.symbol}
            </div>
            <div class="pk-wl-name">{inst.name}</div>
          </div>

          <Sparkline {inst} />

          <div class="pk-wl-right">
            <div
              class="pk-wl-price"
              class:flash-up={inst.flash === 'up'}
              class:flash-down={inst.flash === 'down'}
            >
              {fmtPrice(inst)}
            </div>
            {#if inst.quoted}
              <div class="pk-wl-chg" class:pk-up={inst.up} class:pk-down={!inst.up}>
                {fmtPct(inst.changePct)}
              </div>
              <div class="pk-wl-vol" title={inst.provider ?? ''}>
                {inst.volume === null ? NO_VALUE : `VOL ${fmtCompact(inst.volume)}`}
              </div>
            {:else}
              <div class="pk-wl-chg pk-wl-gap">no quote</div>
              <div class="pk-wl-vol">{fmtAge(inst.observedAt)}</div>
            {/if}
          </div>

          <button
            class="pk-wl-drop"
            title={`Stop tracking ${inst.symbol}`}
            aria-label={`Stop tracking ${inst.symbol}`}
            disabled={busyId === inst.providerId}
            onclick={(e) => {
              e.stopPropagation();
              void untrack(inst);
            }}
          >
            <X size={11} />
          </button>
        </div>
      {/each}
    {/if}
  </div>
</aside>

<style>
  .pk-track {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .pk-track-head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-right: 6px;
  }
  .pk-track-head .pk-tabs {
    flex: 1;
    min-width: 0;
  }
  .pk-track-add {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    flex: none;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
  }
  .pk-track-add:hover,
  .pk-track-add.active {
    border-color: var(--p-accent);
    color: var(--p-accent);
  }

  .pk-track-search {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 6px;
    border-bottom: 1px solid var(--p-border);
  }
  .pk-track-field {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 8px;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    color: var(--p-dim);
  }
  .pk-track-field:focus-within {
    border-color: var(--p-accent);
  }
  .pk-track-field input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    color: var(--p-text);
    font-family: var(--font-mono);
    font-size: var(--fz-sm);
    outline: none;
  }
  .spin {
    display: inline-flex;
    animation: pk-spin 0.9s linear infinite;
  }
  @keyframes pk-spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
  .pk-track-msg {
    margin: 0;
    color: var(--p-dim);
    font-size: var(--fz-sm);
    line-height: 1.4;
  }
  .pk-track-hits {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 210px;
  }
  .pk-track-hit {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 7px;
    padding: 5px 7px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
    text-align: left;
  }
  .pk-track-hit:hover:not(:disabled) {
    background: color-mix(in srgb, var(--p-accent) 9%, transparent);
    color: var(--p-accent);
  }
  .pk-track-hit:disabled {
    cursor: default;
    opacity: 0.55;
  }
  .pk-track-hit-id {
    display: flex;
    min-width: 0;
    flex-direction: column;
  }
  .pk-track-hit-id b {
    color: var(--p-text);
    font-size: var(--fz-sm);
  }
  .pk-track-hit-id small,
  .pk-track-hit-venue,
  .pk-track-hit-state {
    overflow: hidden;
    font-size: var(--fz-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pk-track-hit-venue {
    max-width: 9ch;
    opacity: 0.7;
  }

  .pk-kind {
    display: inline-grid;
    width: 13px;
    height: 13px;
    flex: none;
    border-radius: 3px;
    font-size: 8px;
    font-weight: 700;
    place-items: center;
  }
  .pk-kind[data-kind='equity'] {
    background: color-mix(in srgb, var(--p-accent) 22%, transparent);
    color: var(--p-accent);
  }
  .pk-kind[data-kind='crypto'] {
    background: color-mix(in srgb, var(--p-accent2) 22%, transparent);
    color: var(--p-accent2);
  }

  .pk-track-body {
    flex: 1;
    min-height: 0;
  }
  .pk-wl-id {
    min-width: 0;
  }
  .pk-wl-sym {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .pk-wl-row.unquoted .pk-wl-price {
    color: var(--p-dim);
  }
  .pk-wl-gap {
    color: var(--p-dim);
    font-style: italic;
    opacity: 0.75;
  }
  .pk-wl-drop {
    display: grid;
    width: 17px;
    height: 17px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
    opacity: 0;
    place-items: center;
    transition: opacity 0.12s ease;
  }
  .pk-wl-row:hover .pk-wl-drop,
  .pk-wl-drop:focus-visible {
    opacity: 1;
  }
  .pk-wl-drop:hover {
    background: color-mix(in srgb, var(--p-down) 16%, transparent);
    color: var(--p-down);
  }
</style>
