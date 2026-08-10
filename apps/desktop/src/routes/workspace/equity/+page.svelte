<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import UnavailableExperience from '$lib/UnavailableExperience.svelte';

  interface TerminalQuote {
    symbol: string;
    price: number;
    changePct?: number;
    volume?: number;
    provider: string;
    observedAt: string;
  }

  interface TerminalFeedSnapshot {
    mode: 'live' | 'degraded' | 'simulation';
    providers: string[];
    quotes: TerminalQuote[];
    retrievedAt: string;
    message: string;
  }

  let snapshot = $state<TerminalFeedSnapshot | null>(null);
  let loading = $state(true);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      snapshot = await invoke<TerminalFeedSnapshot>('get_terminal_feed');
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    load();
    const interval = setInterval(load, 15_000);
    return () => clearInterval(interval);
  });

  // Equity symbols from the terminal feed (Finnhub covers US equities).
  const equitySymbols = new Set(['AAPL', 'NVDA', 'TSLA', 'MSFT', 'AMD', 'AMZN', 'META', 'SPY', 'QQQ']);
  const equityQuotes = $derived(
    (snapshot?.quotes ?? []).filter((q) => equitySymbols.has(q.symbol.toUpperCase())),
  );
  const hasEquityData = $derived(equityQuotes.length > 0);
  const isLive = $derived(snapshot?.mode === 'live');

  const fmtPrice = (p: number) =>
    p >= 1000 ? p.toLocaleString('en-US', { maximumFractionDigits: 0 }) : p.toFixed(2);
  const fmtVol = (v?: number) => (v ? (v >= 1e6 ? `${(v / 1e6).toFixed(1)}M` : v >= 1e3 ? `${(v / 1e3).toFixed(0)}K` : String(v)) : '—');
  const fmtPct = (p?: number) => (p === undefined ? '—' : `${p >= 0 ? '+' : ''}${p.toFixed(2)}%`);
</script>

<svelte:head><title>Equity intelligence · PRISMATIK</title></svelte:head>

<div class="pk-page">
  <div class="pk-page-head">
    <h1>Equity intelligence</h1>
    <p>
      Live US equity quotes from governed providers. Session state, deep history, and fundamentals
      require a licensed equity data feed — connect one in <a href="/workspace/integrations">Integrations</a>.
    </p>
  </div>

  {#if error}
    <div class="pk-error">{error}</div>
  {/if}

  {#if loading}
    <div class="pk-empty">Loading equity quotes…</div>
  {:else if snapshot}
    <div class="pk-mode-banner" class:live={isLive} class:degraded={!isLive}>
      <span class="pk-mode-dot"></span>
      <span>Feed mode: <strong>{snapshot.mode.toUpperCase()}</strong> · {snapshot.providers.length} provider(s) connected</span>
      <span class="pk-dim" style="margin-left:auto">{snapshot.message}</span>
    </div>

    {#if hasEquityData}
      <section class="pk-panel">
        <div class="pk-panel-head"><span>US equity quotes ({equityQuotes.length})</span></div>
        <table class="pk-table">
          <thead>
            <tr><th>Symbol</th><th>Price</th><th>Change</th><th>Volume</th><th>Provider</th><th>Observed</th></tr>
          </thead>
          <tbody>
            {#each equityQuotes as q (q.symbol)}
              <tr>
                <td class="pk-mono pk-sym">{q.symbol}</td>
                <td class="pk-mono">${fmtPrice(q.price)}</td>
                <td class="pk-mono" style="color: {(q.changePct ?? 0) >= 0 ? '#34d399' : '#f87171'}">{fmtPct(q.changePct)}</td>
                <td class="pk-mono pk-dim">{fmtVol(q.volume)}</td>
                <td class="pk-mono pk-dim">{q.provider}</td>
                <td class="pk-mono pk-dim">{new Date(q.observedAt).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>
      <p class="pk-note">
        Quotes are real governed observations from {snapshot.providers.join(', ') || 'no provider'}.
        Deep history (25+ years), intraday bars, options chains, and fundamentals require licensed feeds
        (Polygon, Databento, Intrinio). Wire one in <a href="/workspace/integrations">Integrations</a>.
      </p>
    {:else}
      <UnavailableExperience
        active="equity"
        title="Equity intelligence"
        description="No live equity quotes available. Connect an equity provider (Finnhub, Polygon, Alpha Vantage) in Integrations to populate real quotes."
      />
    {/if}
  {/if}
</div>

<style>
  .pk-page { padding: 24px; max-width: 1200px; }
  .pk-page-head h1 { font-size: 1.4rem; font-weight: 600; margin: 0 0 6px; color: var(--p-text); }
  .pk-page-head p { font-size: 0.8125rem; color: var(--p-text-dim); margin: 0 0 20px; max-width: 65ch; line-height: 1.5; }
  .pk-page-head a { color: var(--p-accent); }
  .pk-error { background: rgba(255,80,80,0.12); border: 1px solid rgba(255,80,80,0.3); color: #ff9090; padding: 10px 14px; border-radius: 4px; font-size: 0.8125rem; margin-bottom: 16px; }
  .pk-empty { padding: 40px; text-align: center; color: var(--p-text-dim); }
  .pk-mode-banner { display: flex; align-items: center; gap: 10px; padding: 10px 16px; border-radius: 6px; margin-bottom: 16px; font-size: 0.75rem; font-family: var(--p-mono); }
  .pk-mode-banner.live { background: rgba(52,211,153,0.08); border: 1px solid rgba(52,211,153,0.2); color: #34d399; }
  .pk-mode-banner.degraded { background: rgba(251,191,36,0.06); border: 1px solid rgba(251,191,36,0.2); color: #fbbf24; }
  .pk-mode-dot { width: 7px; height: 7px; border-radius: 50%; background: currentColor; }
  .pk-panel { background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 8px; margin-bottom: 16px; }
  .pk-panel-head { padding: 12px 16px; border-bottom: 1px solid var(--p-border); font-family: var(--p-mono); font-size: 0.625rem; letter-spacing: 0.14em; text-transform: uppercase; color: var(--p-text-dim); }
  .pk-table { width: 100%; border-collapse: collapse; }
  .pk-table th { text-align: left; padding: 10px 16px; font-size: 0.625rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--p-text-dim); border-bottom: 1px solid var(--p-border); }
  .pk-table td { padding: 8px 16px; font-size: 0.8125rem; border-bottom: 1px solid var(--p-border); }
  .pk-sym { font-weight: 600; color: var(--p-text); }
  .pk-mono { font-family: var(--p-mono); }
  .pk-dim { color: var(--p-text-dim); }
  .pk-note { font-size: 0.75rem; color: var(--p-text-dim); line-height: 1.5; }
  .pk-note a { color: var(--p-accent); }
</style>
