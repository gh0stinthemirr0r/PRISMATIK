<script lang="ts">
  /**
   * Time & sales.
   *
   * A print tape needs a trade-level stream. The connected providers poll
   * snapshots, so this panel shows the observation log it *can* honestly show:
   * each quote actually received for the selected instrument, with its source.
   */
  import { market, fmtNum, fmtAge } from './market.svelte';
  import NoData from './NoData.svelte';

  const inst = $derived(market.selected);
  /** Newest first — the history array is append-ordered. */
  const observations = $derived(inst ? [...inst.history].reverse().slice(0, 60) : []);
</script>

<section class="pk-panel">
  <div class="pk-panel-head">
    <span>Observations</span>
    <span class="pk-mono">{observations.length}</span>
  </div>
  <div class="pk-scroll pk-panel-body">
    {#if !inst}
      <NoData title="No instrument selected" compact />
    {:else if observations.length === 0}
      <NoData
        title="No quotes received"
        detail={market.feedMessage}
        compact
      />
    {:else}
      <div class="pk-obs-note">
        Quote snapshots from {inst.provider ?? 'provider'} · not a trade tape
      </div>
      {#each observations as price, i (i)}
        {@const prev = observations[i + 1]}
        <div class="pk-trade-row">
          <span class="t">{i === 0 ? fmtAge(inst.observedAt) : ''}</span>
          <span>{inst.symbol}</span>
          <span
            class:pk-up={prev !== undefined && price >= prev}
            class:pk-down={prev !== undefined && price < prev}
          >
            {fmtNum(price, inst.decimals)}
          </span>
          <span class="sz">
            {prev === undefined ? '' : fmtNum(((price - prev) / prev) * 100, 3) + '%'}
          </span>
        </div>
      {/each}
    {/if}
  </div>
</section>

<style>
  .pk-panel-body {
    flex: 1;
    min-height: 0;
  }
  .pk-obs-note {
    padding: 5px 10px;
    color: var(--p-dim);
    font-size: var(--fz-sm);
    opacity: 0.75;
  }
</style>
