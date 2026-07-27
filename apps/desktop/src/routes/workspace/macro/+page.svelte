<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";

  type Series = {
    seriesId: string;
    title: string;
    unit: string;
    provider: string;
    retrievedAt: string;
    points: Array<{ date: string; value: number }>;
  };
  type Cot = {
    marketCode: string;
    marketName: string;
    reportDate: string;
    long: number;
    short: number;
    net: number;
    provider: string;
    retrievedAt: string;
  };
  type Catalyst = {
    id: string;
    kind: string;
    title: string;
    occursAt: string;
    relatedSymbol: string;
    provider: string;
  };
  type Session = {
    venue: string;
    opensAt: string;
    closesAt: string;
    state: string;
    provider: string;
    retrievedAt: string;
  };

  let rates = $state<Series | null>(null);
  let inflation = $state<Series | null>(null);
  let cot = $state<Cot | null>(null);
  let catalysts = $state<Catalyst[]>([]);
  let session = $state<Session | null>(null);

  onMount(async () => {
    [rates, inflation, cot, catalysts, session] = await Promise.all([
      invoke<Series>("get_macro_series", { seriesId: "DGS10" }),
      invoke<Series>("get_macro_series", { seriesId: "CPIAUCSL" }),
      invoke<Cot>("get_cot_report", { marketCode: "067651" }),
      invoke<Catalyst[]>("get_catalyst_calendar"),
      invoke<Session>("get_next_session", { venue: "XNYS" }),
    ]);
  });
</script>

<svelte:head><title>Macro · PRISMATIK</title></svelte:head>
<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}<div class="rail-label">Experiences</div><ExperiencesNav active="macro" />{/snippet}
  {#snippet status()}<EvidenceChip status="confirmed" label="offline macro" />{/snippet}
  <div class="canvas">
    <header>
      <h1>Macro regime</h1>
      <p>FRED series, COT positioning, equity session, and catalyst calendar.</p>
    </header>

    <div class="cards">
      {#each [rates, inflation] as series}
        <article>
          {#if series}
            <div class="kicker">{series.seriesId}</div>
            <h2>{series.title}</h2>
            <strong>{series.points.at(-1)?.value.toFixed(2)} <small>{series.unit}</small></strong>
            <div class="spark">
              {#each series.points as point}
                <span style={`height:${Math.max(14, (point.value / Math.max(...series.points.map((p) => p.value))) * 100)}%`} title={`${point.date}: ${point.value}`}></span>
              {/each}
            </div>
            <EvidenceChip status="confirmed" label={series.provider} />
            <StaleDataMarker eventTime={series.retrievedAt} maxAge={86_400_000} />
          {:else}
            <p>Loading series…</p>
          {/if}
        </article>
      {/each}
      <article>
        {#if cot}
          <div class="kicker">COT · {cot.marketCode}</div>
          <h2>{cot.marketName}</h2>
          <strong>{cot.net.toLocaleString()} <small>net contracts</small></strong>
          <dl>
            <div><dt>Long</dt><dd>{cot.long.toLocaleString()}</dd></div>
            <div><dt>Short</dt><dd>{cot.short.toLocaleString()}</dd></div>
          </dl>
          <EvidenceChip status="confirmed" label={cot.provider} />
          <StaleDataMarker eventTime={cot.retrievedAt} maxAge={86_400_000} />
        {:else}
          <p>Loading report…</p>
        {/if}
      </article>
    </div>

    <section>
      <h2>Event & catalyst calendar</h2>
      <div class="calendar">
        {#if session}
          <article class="session">
            <div class="kicker">{session.venue} session</div>
            <strong>{session.state}</strong>
            <p>Opens {session.opensAt}</p>
            <p>Closes {session.closesAt}</p>
            <EvidenceChip status="confirmed" label={session.provider} />
          </article>
        {/if}
        {#each catalysts as event}
          <article>
            <div class="kicker">{event.kind} · {event.relatedSymbol}</div>
            <strong>{event.title}</strong>
            <p>{event.occursAt}</p>
            <EvidenceChip status="confirmed" label={event.provider} />
          </article>
        {/each}
      </div>
    </section>
  </div>
</WorkspaceShell>

<style>
  .rail-label,.kicker,h2{margin:0 0 var(--space-3);color:var(--color-text-tertiary);font-size:var(--font-size-xs);letter-spacing:.06em;text-transform:uppercase}
  .canvas{padding:var(--space-5) var(--space-6)}
  header{margin-bottom:var(--space-6)}
  h1{margin:0;font-size:var(--font-size-2xl)}
  header p,article p{color:var(--color-text-secondary);font-size:var(--font-size-sm)}
  .cards,.calendar{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:var(--space-5)}
  .calendar{margin-top:var(--space-6);grid-template-columns:repeat(4,minmax(0,1fr))}
  article{display:grid;gap:var(--space-3);padding:var(--space-5);border:1px solid var(--color-border-default);background:var(--color-surface-1)}
  article h2{min-height:3em;margin:0;font-size:var(--font-size-md);text-transform:none;letter-spacing:0;color:var(--color-text-primary)}
  strong{font:600 var(--font-size-2xl) var(--font-mono)}
  .calendar strong{font-size:var(--font-size-md)}
  small{font-size:var(--font-size-xs);color:var(--color-text-tertiary)}
  .spark{display:flex;align-items:end;gap:6px;height:80px}
  .spark span{flex:1;min-height:4px;background:var(--color-brand-primary)}
  dl{display:grid;gap:var(--space-2);margin:0}
  dl div{display:flex;justify-content:space-between}
  dt{color:var(--color-text-secondary)}
  dd{margin:0;font-family:var(--font-mono)}
  section{margin-top:var(--space-7)}
  @media(max-width:1000px){.cards,.calendar{grid-template-columns:1fr}}
</style>
