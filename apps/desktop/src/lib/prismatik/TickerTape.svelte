<script lang="ts">
  /**
   * Tape of tracked instruments. Only quoted rows scroll — an unquoted symbol
   * would otherwise read as a flat price rather than as an absent one.
   */
  import { market, fmtPrice, fmtPct, type Instrument } from './market.svelte';
</script>

{#snippet tapeItems(items: Instrument[])}
  {#each items as inst (inst.kind + inst.providerId)}
    <span class="pk-tape-item">
      <span class="sym">{inst.symbol}</span>
      <span class:flash-up={inst.flash === 'up'} class:flash-down={inst.flash === 'down'}>
        {fmtPrice(inst)}
      </span>
      <span class:pk-up={inst.up} class:pk-down={!inst.up}>{fmtPct(inst.changePct)}</span>
    </span>
  {/each}
{/snippet}

<div class="pk-tape" aria-label="Ticker tape">
  {#if market.quotedCount === 0}
    <div class="pk-tape-idle">
      {market.hasTracked ? market.feedMessage : 'No instruments tracked'}
    </div>
  {:else}
    {@const quoted = market.instruments.filter((i) => i.quoted)}
    <div class="pk-tape-track">
      {@render tapeItems(quoted)}
      {@render tapeItems(quoted)}
    </div>
  {/if}
</div>

<style>
  .pk-tape-idle {
    display: flex;
    height: 100%;
    align-items: center;
    padding-left: 12px;
    color: var(--p-dim);
    font-size: var(--fz-sm);
    letter-spacing: 0.05em;
  }
</style>
