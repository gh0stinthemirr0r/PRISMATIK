<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";

  type Hit = { id: string; score: number; disclosures: string[] };
  type Payload = {
    query: string;
    hits: Hit[];
    provider: string;
    retrievedAt: string;
  };

  const fallback: Payload = {
    query: "AAPL earnings gap + vol crush",
    hits: [
      {
        id: "analog-2019-q1",
        score: 0.91,
        disclosures: [
          "Leave-one-out sensitivity: score drops to 0.78 when 2019-Q1 removed",
          "Mandatory: not a forward-looking guarantee",
        ],
      },
      {
        id: "analog-2022-oct",
        score: 0.84,
        disclosures: [
          "Filter: excludes pre-split symbology",
          "Sensitivity: ±2σ move threshold applied",
        ],
      },
      {
        id: "analog-2016-jan",
        score: 0.76,
        disclosures: [
          "Low sample count in matched window (n=3)",
          "Disclosure: macro regime mismatch flagged",
        ],
      },
    ],
    provider: "ui-static-fallback",
    retrievedAt: "2026-07-25T20:00:00Z",
  };

  let data = $state<Payload>(fallback);
  let provenance = $state("static fallback");

  onMount(async () => {
    try {
      data = await invoke<Payload>("get_analog_hits", {
        query: "AAPL earnings gap + vol crush",
      });
      provenance = "trusted core";
    } catch {
      provenance = "browser fallback";
    }
  });
</script>

<svelte:head><title>Analogs · PRISMATIK</title></svelte:head>
<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}<div class="rail-label">Experiences</div><ExperiencesNav active="analogs" />{/snippet}
  {#snippet status()}<EvidenceChip status="confirmed" label={provenance} />{/snippet}
  <div class="canvas">
    <header>
      <div>
        <h1>Analog search</h1>
        <p>Historical pattern matches with mandatory disclosures and sensitivity notes.</p>
      </div>
      <div class="chips">
        <EvidenceChip status="confirmed" label={data.provider} />
        <StaleDataMarker eventTime={data.retrievedAt} maxAge={86_400_000} />
      </div>
    </header>

    <section class="query-panel">
      <div class="label">Query</div>
      <p class="query">{data.query}</p>
      <EvidenceChip status="uncertain" label="P55-EX-01 scaffold" />
    </section>

    <section class="hits" aria-label="Analog hits">
      {#each data.hits as hit}
        <article class="hit">
          <div class="hit-head">
            <strong>{hit.id}</strong>
            <EvidenceChip status="confirmed" label="score {(hit.score * 100).toFixed(0)}%" />
          </div>
          <ul class="disclosures">
            {#each hit.disclosures as d}
              <li>
                <EvidenceChip status="contradicted" label="disclosure" />
                <span>{d}</span>
              </li>
            {/each}
          </ul>
        </article>
      {/each}
    </section>
  </div>
</WorkspaceShell>

<style>
  .canvas { display: grid; gap: var(--space-lg); }
  header { display: flex; justify-content: space-between; align-items: flex-start; gap: var(--space-md); }
  h1 { margin: 0; font-size: var(--font-size-xl); }
  p { margin: 4px 0 0; color: var(--color-text-secondary); }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; justify-content: flex-end; }
  .label { font-size: var(--font-size-xs); text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-text-secondary); }
  .query-panel {
    padding: var(--space-md);
    border-radius: var(--radius-lg);
    background: var(--color-surface-1);
    display: grid;
    gap: var(--space-sm);
  }
  .query { margin: 0; font-family: var(--font-mono, monospace); font-size: var(--font-size-sm); color: var(--color-text-primary); }
  .hits { display: grid; gap: var(--space-md); }
  .hit { padding: var(--space-md); border-radius: var(--radius-lg); background: var(--color-surface-1); display: grid; gap: var(--space-sm); }
  .hit-head { display: flex; justify-content: space-between; align-items: center; gap: var(--space-sm); }
  .disclosures { margin: 0; padding: 0; list-style: none; display: grid; gap: 8px; }
  .disclosures li { display: grid; gap: 4px; padding: 8px; border-radius: var(--radius-md); background: var(--color-surface-2); }
  .disclosures span { font-size: var(--font-size-sm); color: var(--color-text-primary); }
</style>
