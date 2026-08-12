<script lang="ts">
  import { onMount } from 'svelte';
  import { ArrowUpRight, Search } from 'lucide-svelte';
  import { command } from './command.svelte';
  import { platform } from './platform.svelte';
  import AgentRoom from './AgentRoom.svelte';

  /**
   * The deck has two modes. Search is the palette it always was. Room is where
   * specialists and critics argue in one transcript — put here because
   * convening a discussion is the same gesture as navigating to a surface:
   * you already have the keyboard, and you already know what you want to look
   * at.
   */
  let mode = $state<'search' | 'room'>('search');
  import { market, type Instrument } from './market.svelte';

  let query = $state('');
  let input = $state<HTMLInputElement>();
  const surfaces = [
    ['Intelligence fabric', '/workspace/intelligence', 'Resolution, logic, evidence and discovery'],
    ['Autonomous strategy lab', '/workspace/strategy', 'Typed agents, constraints and unsigned drafts'],
    ['Simulation chamber', '/workspace/simulation', 'Replay, scenarios and deterministic stress'],
    ['Risk observatory', '/workspace/portfolio', 'Exposure, limits and reconciliation'],
    ['Execution console', '/workspace/orders', 'Paper workflow and guarded orders'],
    ['Volatility laboratory', '/workspace/options', 'Chains, surfaces and flow'],
    ['Global macro', '/workspace/macro', 'Rates, releases and regimes'],
    ['Institutional filings', '/workspace/filings', 'Ownership and filing evidence'],
    ['Forecast calibration', '/workspace/calibration', 'Brier reliability and participant skill'],
    ['Provider plane', '/workspace/integrations', 'Credentials, health and source policy'],
    ['Prediction markets', '/workspace/prediction', 'Canonical contracts, venue spreads and resolution risk'],
    ['Feed control', '/workspace/feeds', 'Reviewed RSS sources, cadence and retention policy'],
  ] as const;

  const normalized = $derived(query.trim().toLowerCase());
  const instruments = $derived(market.instruments.filter((item) => `${item.symbol} ${item.name} ${item.market}`.toLowerCase().includes(normalized)).slice(0, 7));
  const filteredSurfaces = $derived(surfaces.filter((item) => `${item[0]} ${item[2]}`.toLowerCase().includes(normalized)).slice(0, 6));

  function chooseInstrument(instrument: Instrument) {
    market.select(instrument.symbol);
    command.open = false;
    query = '';
  }

  $effect(() => {
    if (command.open) requestAnimationFrame(() => input?.focus());
  });

  onMount(() => {
    const keydown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        command.open = !command.open;
      } else if (event.key === 'Escape') {
        command.open = false;
      }
    };
    window.addEventListener('keydown', keydown);
    return () => window.removeEventListener('keydown', keydown);
  });
</script>

{#if command.open}
  <button class="pk-command-backdrop" aria-label="Close command deck" onclick={() => (command.open = false)}></button>
  <section class="pk-command" aria-label="Command deck">
    <div class="pk-command-modes" role="tablist" aria-label="Command mode">
      <button role="tab" aria-selected={mode === 'search'} class:active={mode === 'search'} onclick={() => (mode = 'search')}>Search</button>
      <button role="tab" aria-selected={mode === 'room'} class:active={mode === 'room'} onclick={() => (mode = 'room')}>Room</button>
    </div>
    {#if mode === 'room'}
      <div class="pk-command-room"><AgentRoom /></div>
    {:else}
    <label><Search size={17} /><input bind:this={input} bind:value={query} placeholder="Search instruments, intelligence, strategies…" /><kbd>{platform.modifierLabel} K</kbd></label>
    <div class="pk-command-body">
      {#if instruments.length}
        <span class="pk-command-group">TRACKED INSTRUMENTS</span>
        {#each instruments as instrument (instrument.kind + instrument.providerId)}
          <button onclick={() => chooseInstrument(instrument)}><b>{instrument.symbol}</b><span>{instrument.name}</span><small>{instrument.market}</small></button>
        {/each}
      {/if}
      {#if filteredSurfaces.length}
        <span class="pk-command-group">SYSTEM SURFACES</span>
        {#each filteredSurfaces as surface (surface[1])}
          <a href={surface[1]}><b>{surface[0]}</b><span>{surface[2]}</span><ArrowUpRight size={14} /></a>
        {/each}
      {/if}
      {#if !instruments.length && !filteredSurfaces.length}<div class="pk-command-empty">No tracked instrument or system surface matches “{query}”.</div>{/if}
    </div>
    {/if}
    <footer>
      {#if mode === 'room'}<span>Participants answer each other</span><span>Every quoted voice is data</span>{:else}<span>↑↓ Navigate</span><span>↵ Open</span>{/if}
      <span>ESC Close</span><strong>PRISMATIK COMMAND FABRIC</strong>
    </footer>
  </section>
{/if}

<style>
  .pk-command-modes {
    display: flex;
    gap: 4px;
    padding: 8px 10px 0;
  }
  .pk-command-modes button {
    padding: 5px 12px;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
    font: 700 var(--fz-sm) var(--font-mono);
  }
  .pk-command-modes button.active {
    border-color: var(--p-accent);
    background: color-mix(in srgb, var(--p-accent) 10%, transparent);
    color: var(--p-accent);
  }
  .pk-command-room {
    display: flex;
    min-height: 0;
    /* The deck is a palette; a discussion needs room to be readable without
       becoming a full-height overlay. */
    height: 58vh;
    padding: 10px;
  }
</style>
