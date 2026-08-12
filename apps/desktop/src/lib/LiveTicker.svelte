<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getLiveQuote, getOrderBook } from '$lib/api';

  interface TickData {
    symbol: string;
    price: number;
    bid: number;
    ask: number;
    spread: number;
    timestamp_ms: number;
  }

  interface OrderBookData {
    symbol: string;
    bids: Array<{price: number; volume: number}>;
    asks: Array<{price: number; volume: number}>;
    spread: number;
  }

  let targetSymbol = 'BTC';
  let tickData: TickData | null = null;
  let orderBookData: OrderBookData | null = null;
  let loading = true;
  let error: string | null = null;
  let lastUpdate = '';

  async function refresh() {
    try {
      const [tick, book] = await Promise.all([
        getLiveQuote(targetSymbol),
        getOrderBook(targetSymbol)
      ]);

      tickData = {
        symbol: tick.symbol,
        price: tick.price,
        bid: tick.bid,
        ask: tick.ask,
        spread: tick.spread,
        timestamp_ms: Date.now(), // Convert to JS date for display
      };

      orderBookData = {
        symbol: book.symbol,
        bids: book.bids.map(l => ({ price: l.price, volume: l.volume })),
        asks: book.asks.map(l => ({ price: l.price, volume: l.volume })),
        spread: book.spread,
      };

      lastUpdate = new Date().toLocaleTimeString();
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Unknown error';
    } finally {
      loading = false;
    }
  }

  // Auto-refresh every second for live feel
  let intervalId: ReturnType<typeof setInterval>;
  onMount(() => {
    refresh();
    intervalId = setInterval(refresh, 1000);
  });

  onDestroy(() => {
    clearInterval(intervalId);
  });
</script>

<div class="live-ticker">
  <h3>Live Ticker — {targetSymbol}</h3>
  
  {#if loading}
    <p>Loading market data...</p>
  {:else if error}
    <div class="error">{error}</div>
  {:else}
    <div class="price-row">
      <span class="current-price">${tickData?.price.toFixed(2)}</span>
      <span class="bid-ask">Bid: ${tickData?.bid.toFixed(2)} | Ask: ${tickData?.ask.toFixed(2)}</span>
      <span class="spread">{tickData?.spread.toFixed(4)} spread</span>
    </div>
    
    {#if orderBookData}
      <div class="order-book">
        <h4>Order Book</h4>
        <div class="levels">
          {#each orderBookData.asks as ask (ask.price + '@' + ask.volume)}
            <div class="level ask">
              <span class="price">${ask.price.toFixed(2)}</span>
              <span class="volume">{ask.volume} vol</span>
            </div>
          {/each}
          
          <span class="mid-price">— ${tickData?.price.toFixed(2)} —</span>
          
          {#each orderBookData.bids as bid (bid.price + '@' + bid.volume)}
            <div class="level bid">
              <span class="price">${bid.price.toFixed(2)}</span>
              <span class="volume">{bid.volume} vol</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <p class="update-time">Updated: {lastUpdate}</p>
  {/if}
</div>

<style>
.live-ticker {
  padding: 16px;
  border: 1px solid #ddd;
  border-radius: 8px;
  font-family: 'Inter', sans-serif;
}

h3 {
  margin-top: 0;
  color: #2d3748;
}

.price-row {
  display: flex;
  gap: 16px;
  align-items: center;
  margin: 12px 0;
  font-size: 1.1em;
}

.current-price {
  font-weight: bold;
  color: #38a169; /* green for price */
  font-size: 1.3em;
}

.bid-ask {
  color: #718096;
}

.spread {
  color: #e53e3e; /* red for spread */
}

.order-book h4 {
  margin: 12px 0 8px 0;
  font-size: 0.9em;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.levels {
  display: flex;
  gap: 4px;
  align-items: center;
  margin-bottom: 8px;
}

.level {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 0.85em;
  text-align: right;
}

.level.ask {
  background-color: #fff5f5;
  color: #c53030;
}

.level.bid {
  background-color: #f0fff4;
  color: #276749;
}

.mid-price {
  font-weight: bold;
  padding: 8px 16px;
}

.update-time {
  margin-top: 8px;
  font-size: 0.8em;
  color: #a0aec0;
}

.error {
  color: #e53e3e;
  padding: 8px;
  background-color: #fff5f5;
  border-radius: 4px;
}
</style>
