<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";
  import PerspectiveViewer from "$lib/perspective/PerspectiveViewer.svelte";
  import type { PerspectiveRow } from "$lib/perspective/bootstrap";

  type Filing = {
    cik: string;
    issuer: string;
    form: string;
    filedAt: string;
    period: string;
    headline: string;
    valueUsd: number;
    provider: string;
    retrievedAt: string;
  };
  type Insider = {
    cik: string;
    issuer: string;
    insider: string;
    title: string;
    transactionCode: string;
    shares: number;
    price: number;
    filedAt: string;
    provider: string;
    retrievedAt: string;
  };

  let filings = $state<Filing[]>([]);
  let insiders = $state<Insider[]>([]);
  let error = $state<string | null>(null);
  let issuerFilter = $state("");

  const ownership = $derived(
    filings
      .filter((row) => row.form.includes("13F"))
      .slice()
      .sort((a, b) => b.valueUsd - a.valueUsd),
  );
  const ownershipRows = $derived<PerspectiveRow[]>(
    ownership.map((row) => ({
      issuer: row.issuer,
      period: row.period,
      form: row.form,
      headline: row.headline,
      value_usd: row.valueUsd,
      provider: row.provider,
      filed_at: row.filedAt,
      cik: row.cik,
    })),
  );
  const filteredInsiders = $derived(
    insiders.filter((row) =>
      issuerFilter ? row.issuer.toLowerCase().includes(issuerFilter.toLowerCase()) : true,
    ),
  );

  onMount(async () => {
    try {
      [filings, insiders] = await Promise.all([
        invoke<Filing[]>("get_filing_summaries", { cik: null }),
        invoke<Insider[]>("get_insider_transactions", { cik: null }),
      ]);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  });
</script>

<svelte:head><title>Filings · PRISMATIK</title></svelte:head>
<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}<div class="rail-label">Experiences</div><ExperiencesNav active="filings" />{/snippet}
  {#snippet status()}<EvidenceChip status={error ? "uncertain" : "confirmed"} label="SEC embedded" />{/snippet}
  <div class="canvas">
    <header>
      <div>
        <h1>Institutional intelligence</h1>
        <p>13F ownership via Perspective, insider activity grid, and filing evidence — offline fixtures.</p>
      </div>
      <label>
        Filter insider issuer
        <input bind:value={issuerFilter} placeholder="Apple" />
      </label>
    </header>
    {#if error}<div class="error">{error}</div>{/if}

    <section>
      <div class="section-head">
        <h2>13F ownership</h2>
        <EvidenceChip status="confirmed" label="Perspective datagrid" />
      </div>
      <PerspectiveViewer rows={ownershipRows} height={360} title="13F ownership" />
    </section>

    <section>
      <h2>Insider activity</h2>
      <div class="table insider">
        <div class="tr th"><span>Issuer</span><span>Insider</span><span>Code</span><span>Shares @ price</span><span>Filed</span><span>Evidence</span></div>
        {#each filteredInsiders as row}
          <div class="tr">
            <strong>{row.issuer}</strong>
            <span>{row.insider} · {row.title}</span>
            <span>{row.transactionCode}</span>
            <span class="number">{row.shares.toLocaleString()} @ ${row.price.toFixed(2)}</span>
            <span>{row.filedAt}</span>
            <div class="meta"><EvidenceChip status="confirmed" label={row.provider}/><StaleDataMarker eventTime={row.retrievedAt} maxAge={86_400_000}/></div>
          </div>
        {/each}
      </div>
    </section>
  </div>
</WorkspaceShell>

<style>
  .rail-label,h2{color:var(--color-text-tertiary);font-size:var(--font-size-xs);letter-spacing:.06em;text-transform:uppercase}
  .rail-label{margin:0 0 var(--space-3)}
  .canvas{padding:var(--space-5) var(--space-6)}
  header{display:flex;justify-content:space-between;gap:var(--space-4);align-items:end;margin-bottom:var(--space-6)}
  h1{margin:0;font-size:var(--font-size-2xl)}
  header p{color:var(--color-text-secondary);font-size:var(--font-size-sm)}
  label{display:grid;gap:var(--space-1);font-size:var(--font-size-xs);color:var(--color-text-tertiary)}
  input{padding:var(--space-2) var(--space-3);border:1px solid var(--color-border-default);background:var(--color-surface-1);color:var(--color-text-primary)}
  section{margin-top:var(--space-6)}
  .section-head{display:flex;justify-content:space-between;align-items:center;margin-bottom:var(--space-3)}
  .section-head h2{margin:0}
  .table{overflow:auto;border:1px solid var(--color-border-default)}
  .tr{display:grid;grid-template-columns:1.2fr .8fr 1.6fr .8fr .8fr;gap:var(--space-3);align-items:center;padding:var(--space-3);border-bottom:1px solid var(--color-border-default);font-size:var(--font-size-sm)}
  .insider .tr{grid-template-columns:1fr 1.4fr .5fr 1fr .8fr .8fr}
  .th{color:var(--color-text-tertiary);font-size:var(--font-size-xs);text-transform:uppercase;background:var(--color-surface-1)}
  .number{font-family:var(--font-mono)}
  .meta{display:grid;gap:var(--space-1)}
  .error{color:var(--color-danger)}
  @media(max-width:900px){header{grid-template-columns:1fr;display:grid}.tr{min-width:760px}}
</style>
