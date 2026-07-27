<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, PriceChart, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";
  import type { CandlestickPoint } from "@prismatik/chart-contracts";

  type EquityPayload = {
    symbol: string; currency: string; provider: string; retrievedAt: string;
    bars: Array<CandlestickPoint & { volume: number }>;
  };
  type Session = {
    venue: string; opensAt: string; closesAt: string; state: string;
    provider: string; retrievedAt: string;
  };

  const fallback: EquityPayload = {
    symbol: "AAPL", currency: "USD", provider: "ui-static-fallback",
    retrievedAt: "2026-07-25T20:00:00Z",
    bars: [
      { time: 1753449600, open: 213.88, high: 215.42, low: 212.61, close: 214.95, volume: 48921300 },
      { time: 1753536000, open: 215.12, high: 216.75, low: 214.3, close: 216.22, volume: 51304100 },
      { time: 1753622400, open: 216.02, high: 217.21, low: 214.92, close: 215.68, volume: 46872900 },
      { time: 1753708800, open: 215.94, high: 218.04, low: 215.41, close: 217.73, volume: 54180600 },
    ],
  };
  let equity = $state<EquityPayload>(fallback);
  let session = $state<Session | null>(null);
  let provenance = $state("static fallback");
  const latest = $derived(equity.bars.at(-1));

  onMount(async () => {
    try {
      [equity, session] = await Promise.all([
        invoke<EquityPayload>("get_equity_bars", { symbol: "AAPL" }),
        invoke<Session>("get_next_session", { venue: "XNYS" }),
      ]);
      provenance = "trusted core";
    } catch {
      provenance = "browser fallback";
    }
  });
</script>

<svelte:head><title>Equity · PRISMATIK</title></svelte:head>
<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}<div class="rail-label">Experiences</div><ExperiencesNav active="equity" />{/snippet}
  {#snippet status()}<EvidenceChip status="confirmed" label={provenance} />{/snippet}
  <div class="canvas">
    <header><div><h1>Equity dashboard</h1><p>AAPL quote and daily bars from the offline experience adapter.</p></div><EvidenceChip status="confirmed" label={equity.provider} /></header>
    <section class="metrics">
      <article><span>Last</span><strong>${latest?.close.toFixed(2) ?? "—"}</strong><EvidenceChip status="confirmed" label="close" /><StaleDataMarker eventTime={equity.retrievedAt} maxAge={86_400_000} /></article>
      <article><span>Volume</span><strong>{latest?.volume.toLocaleString() ?? "—"}</strong><EvidenceChip status="confirmed" label="consolidated fixture" /><StaleDataMarker eventTime={equity.retrievedAt} maxAge={86_400_000} /></article>
      <article><span>Session</span><strong>{session?.state ?? "scheduled"}</strong><EvidenceChip status="uncertain" label={session?.provider ?? "calendar stub"} />{#if session}<StaleDataMarker eventTime={session.retrievedAt} maxAge={86_400_000} />{/if}</article>
    </section>
    <section class="grid">
      <div class="panel"><div class="label">AAPL · daily OHLC</div><PriceChart candles={equity.bars} height={360} /></div>
      <aside class="panel"><div class="label">Next XNYS session</div><strong>{session ? new Date(session.opensAt).toLocaleString() : "Mon 09:30 ET"}</strong><p>{session ? `Closes ${new Date(session.closesAt).toLocaleTimeString()}` : "Calendar command unavailable; static schedule shown."}</p><EvidenceChip status="uncertain" label="calendar scaffold" /></aside>
    </section>
  </div>
</WorkspaceShell>

<style>
  .rail-label,.label{margin:0 0 var(--space-3);color:var(--color-text-tertiary);font-size:var(--font-size-xs);letter-spacing:.06em;text-transform:uppercase}
  .canvas{padding:var(--space-5) var(--space-6)} header{display:flex;justify-content:space-between;align-items:end;margin-bottom:var(--space-5)} h1{margin:0;font-size:var(--font-size-2xl)} header p,.panel p{color:var(--color-text-secondary);font-size:var(--font-size-sm)}
  .metrics{display:grid;grid-template-columns:repeat(3,1fr);border-block:1px solid var(--color-border-default);margin-bottom:var(--space-5)} article{display:grid;gap:var(--space-2);padding:var(--space-4);border-right:1px solid var(--color-border-default)} article span{color:var(--color-text-tertiary);font-size:var(--font-size-xs);text-transform:uppercase} article strong{font:600 var(--font-size-xl) var(--font-mono)}
  .grid{display:grid;grid-template-columns:minmax(0,2fr) minmax(240px,1fr);gap:var(--space-6)} .panel{min-width:0;padding-top:var(--space-4);border-top:1px solid var(--color-border-default)} .panel>strong{font-family:var(--font-mono)}
  @media(max-width:900px){.metrics,.grid{grid-template-columns:1fr}}
</style>
