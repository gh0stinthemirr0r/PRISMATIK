<script lang="ts">
  /**
   * Level 2 depth.
   *
   * Neither connected provider supplies an order book: CoinGecko is aggregate
   * market data and the Finnhub tier in use returns candles. Rather than derive
   * a plausible ladder from the last trade — which is what this panel used to
   * do — it states exactly what would be required to fill it.
   */
  import { market } from './market.svelte';
  import NoData from './NoData.svelte';
</script>

<section class="pk-panel">
  <div class="pk-panel-head">
    <span>Order Book</span>
    <span class="pk-mono">{market.selected?.symbol ?? '—'}</span>
  </div>
  <div class="pk-panel-body">
    <NoData
      title="No depth feed"
      detail="Level 2 requires a venue websocket. The connected providers serve aggregate quotes and candles only."
      action="Review market providers"
      href="/workspace/integrations"
    />
  </div>
</section>

<style>
  .pk-panel-body {
    display: flex;
    flex: 1;
    min-height: 0;
  }
</style>
