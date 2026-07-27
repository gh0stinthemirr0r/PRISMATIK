<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, PriceChart, StaleDataMarker } from "@prismatik/ui";
  import type { CandlestickPoint } from "@prismatik/chart-contracts";

  type CoinDetail = {
    coingeckoId: string;
    symbol: string;
    name: string;
    marketCapRank: number | null;
    homepage: string | null;
    description: string | null;
    categories: string[];
    provider: string;
    retrievedAt: string;
  };
  type Chart = {
    provider: string;
    retrievedAt: string;
    interval: string;
    candles: CandlestickPoint[];
  };
  type Quote = {
    coingeckoId: string;
    price: string;
    change24hPct: string | null;
    provider: string;
    retrievedAt: string;
  };

  let detail = $state<CoinDetail | null>(null);
  let chart = $state<Chart | null>(null);
  let quote = $state<Quote | null>(null);
  let error = $state<string | null>(null);
  const assetId = $derived($page.params.id);
  const evidenceIds = $derived(
    [quote, chart, detail]
      .filter((item): item is Quote | Chart | CoinDetail => item !== null)
      .map((item) => `${item.provider}:${item.retrievedAt}`),
  );

  onMount(async () => {
    try {
      const [nextDetail, nextChart, markets] = await Promise.all([
        invoke<CoinDetail>("get_coin_detail", { coingeckoId: assetId }),
        invoke<Chart>("get_crypto_ohlc", { coingeckoId: assetId, days: 30 }),
        invoke<Quote[]>("get_crypto_markets"),
      ]);
      detail = nextDetail;
      chart = nextChart;
      quote = markets.find((candidate) => candidate.coingeckoId === assetId) ?? null;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  });
</script>

<svelte:head><title>{detail?.symbol?.toUpperCase() ?? assetId} · PRISMATIK</title></svelte:head>

<main class="asset-page">
  <a class="back" href="/workspace?panel=watchlist">← Back to workspace</a>
  {#if error}
    <section class="error">
      <h1>Asset unavailable</h1>
      <p>{error}</p>
    </section>
  {:else if !detail || !chart}
    <section class="loading" aria-busy="true">Loading {assetId} evidence…</section>
  {:else}
    <header>
      <div>
        <div class="symbol">{detail.symbol.toUpperCase()}</div>
        <h1>{detail.name}</h1>
        <p>
          {#if detail.marketCapRank != null}Rank {detail.marketCapRank} · {/if}
          {detail.categories.slice(0, 3).join(" · ")}
        </p>
      </div>
      {#if quote}
        <div class="quote">
          <strong>{Number(quote.price).toLocaleString()}</strong>
          <span>{quote.change24hPct ?? "—"}%</span>
          <StaleDataMarker eventTime={quote.retrievedAt} maxAge={60_000} />
        </div>
      {/if}
    </header>

    <div class="layout">
      <section class="chart">
        <div class="section-title">OHLC · {chart.interval} · {chart.provider}</div>
        <PriceChart candles={chart.candles} height={420} />
        {#if detail.description}<p class="description">{detail.description}</p>{/if}
      </section>
      <aside>
        <div class="section-title">Evidence drawer</div>
        <p>Provider and retrieval time form synthetic Wave 1 evidence identifiers.</p>
        <ol>
          {#each evidenceIds as id}
            <li>
              <EvidenceChip status="confirmed" label="retrieved" />
              <code>{id}</code>
            </li>
          {/each}
        </ol>
      </aside>
    </div>
  {/if}
</main>

<style>
  .asset-page {
    min-height: 100vh;
    padding: var(--space-6);
    background: var(--color-surface-0);
    color: var(--color-text-primary);
  }
  .back { color: var(--color-brand-primary); font-size: var(--font-size-sm); text-decoration: none; }
  header {
    display: flex;
    justify-content: space-between;
    gap: var(--space-6);
    align-items: end;
    padding: var(--space-6) 0 var(--space-5);
    border-bottom: 1px solid var(--color-border-default);
  }
  .symbol { color: var(--color-brand-primary); font-family: var(--font-mono); }
  h1 { margin: var(--space-1) 0; font-size: var(--font-size-3xl); }
  header p, aside p, .description { color: var(--color-text-secondary); }
  .quote { display: grid; text-align: right; }
  .quote strong { font-family: var(--font-mono); font-size: var(--font-size-2xl); }
  .quote span { color: var(--color-text-secondary); }
  .layout { display: grid; grid-template-columns: minmax(0, 1fr) 320px; gap: var(--space-6); padding-top: var(--space-5); }
  .chart, aside { min-width: 0; }
  aside { padding-left: var(--space-5); border-left: 1px solid var(--color-border-default); }
  .section-title {
    margin-bottom: var(--space-3);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    letter-spacing: .06em;
    text-transform: uppercase;
  }
  ol { display: grid; gap: var(--space-4); margin: var(--space-5) 0 0; padding: 0; list-style: none; }
  li { display: grid; gap: var(--space-2); }
  code { overflow-wrap: anywhere; color: var(--color-text-secondary); font-size: var(--font-size-xs); }
  .loading, .error { padding: var(--space-8) 0; }
  @media (max-width: 900px) {
    .layout { grid-template-columns: 1fr; }
    aside { padding: var(--space-5) 0 0; border-left: 0; border-top: 1px solid var(--color-border-default); }
  }
</style>
