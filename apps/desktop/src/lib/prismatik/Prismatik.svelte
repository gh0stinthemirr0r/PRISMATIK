<script lang="ts">
  /**
   * Market terminal layout.
   *
   * The right column is tabbed rather than a three-panel stack: with honest
   * empty states, stacking meant three simultaneous "nothing here" panels
   * competing for the same vertical space. One panel at a time keeps whichever
   * context the desk actually wants at full height.
   */
  import Watchlist from './Watchlist.svelte';
  import MainChart from './MainChart.svelte';
  import TradingViewChart from './TradingViewChart.svelte';
  import OrderBook from './OrderBook.svelte';
  import TradesFeed from './TradesFeed.svelte';
  import NewsFeed from './NewsFeed.svelte';
  import Visualizations from './Visualizations.svelte';

  type Aside = 'observations' | 'news' | 'correlation' | 'depth';

  const TABS: { id: Aside; label: string }[] = [
    { id: 'observations', label: 'Tape' },
    { id: 'news', label: 'News' },
    { id: 'correlation', label: 'Correlation' },
    { id: 'depth', label: 'Depth' },
  ];

  let aside = $state<Aside>('observations');

  /**
   * Which renderer draws the price. The TradingView engine gives a proper
   * time axis, zoom/pan and the regime ribbon; the native canvas keeps the
   * on-chart indicator overlays. Both read the same governed candles, so
   * switching changes presentation only, never the data.
   */
  type Engine = 'tradingview' | 'native';
  let engine = $state<Engine>('tradingview');
</script>

<div class="pk-main">
  <Watchlist />
  <div class="pk-chart-column">
    <div class="pk-engine-switch" role="tablist" aria-label="Chart engine">
      <button
        class="pk-btn"
        role="tab"
        aria-selected={engine === 'tradingview'}
        class:active={engine === 'tradingview'}
        onclick={() => (engine = 'tradingview')}
        title="TradingView Lightweight Charts with the regime ribbon"
      >
        TradingView
      </button>
      <button
        class="pk-btn"
        role="tab"
        aria-selected={engine === 'native'}
        class:active={engine === 'native'}
        onclick={() => (engine = 'native')}
        title="Native canvas chart with on-chart indicator overlays"
      >
        Indicators
      </button>
    </div>
    {#if engine === 'tradingview'}
      <TradingViewChart />
    {:else}
      <MainChart />
    {/if}
  </div>
  <div class="pk-right-stack">
    <div class="pk-aside-tabs" role="tablist" aria-label="Context panel">
      {#each TABS as tab (tab.id)}
        <button
          class="pk-btn"
          role="tab"
          aria-selected={aside === tab.id}
          class:active={aside === tab.id}
          onclick={() => (aside = tab.id)}
        >
          {tab.label}
        </button>
      {/each}
    </div>
    <div class="pk-aside-body">
      {#if aside === 'observations'}
        <TradesFeed />
      {:else if aside === 'news'}
        <NewsFeed />
      {:else if aside === 'correlation'}
        <Visualizations only={['gravity']} />
      {:else}
        <OrderBook />
      {/if}
    </div>
  </div>
</div>

<style>
  .pk-chart-column {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
  }
  .pk-engine-switch {
    display: flex;
    flex: none;
    gap: 2px;
    padding: 5px 6px;
    border-bottom: 1px solid var(--p-border);
  }
  .pk-right-stack {
    display: flex;
    min-height: 0;
    flex-direction: column;
  }
  .pk-aside-tabs {
    display: flex;
    flex: none;
    gap: 2px;
    padding: 5px 6px;
    border-bottom: 1px solid var(--p-border);
  }
  .pk-aside-tabs .pk-btn {
    flex: 1;
  }
  .pk-aside-body {
    display: flex;
    flex: 1;
    min-height: 0;
    flex-direction: column;
  }
</style>
