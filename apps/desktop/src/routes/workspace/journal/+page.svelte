<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";

  type Entry = {
    id: string;
    title: string;
    tags: string[];
    createdAt: string;
  };

  type Payload = {
    entries: Entry[];
    provider: string;
    retrievedAt: string;
  };

  const fallback: Payload = {
    entries: [
      {
        id: "je-001",
        title: "AAPL earnings vol crush thesis",
        tags: ["thesis", "equity", "vol"],
        createdAt: "2026-07-20T14:30:00Z",
      },
      {
        id: "je-002",
        title: "Rates +100bp portfolio stress note",
        tags: ["macro", "risk", "review"],
        createdAt: "2026-07-22T09:15:00Z",
      },
      {
        id: "je-003",
        title: "BTC halving scenario — outcome tagged",
        tags: ["crypto", "outcome", "learning"],
        createdAt: "2026-07-24T18:45:00Z",
      },
    ],
    provider: "ui-static-fallback",
    retrievedAt: "2026-07-25T20:00:00Z",
  };

  let data = $state<Payload>(fallback);
  let provenance = $state("static fallback");

  onMount(async () => {
    try {
      data = await invoke<Payload>("get_journal_entries");
      provenance = "trusted core";
    } catch {
      provenance = "browser fallback";
    }
  });
</script>

<svelte:head><title>Journal · PRISMATIK</title></svelte:head>
<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}<div class="rail-label">Experiences</div><ExperiencesNav active="journal" />{/snippet}
  {#snippet status()}<EvidenceChip status="confirmed" label={provenance} />{/snippet}
  <div class="canvas">
    <header>
      <div>
        <h1>Journal</h1>
        <p>Thesis entries with tags and evidence linkage — Wave 4 P6-EX-02 floor scaffold.</p>
      </div>
      <div class="chips">
        <EvidenceChip status="confirmed" label={data.provider} />
        <StaleDataMarker eventTime={data.retrievedAt} maxAge={86_400_000} />
        <EvidenceChip status="uncertain" label="P6-EX-02 scaffold" />
      </div>
    </header>

    <section class="entries" aria-label="Journal entries">
      {#each data.entries as entry}
        <article class="entry">
          <div class="entry-head">
            <strong>{entry.title}</strong>
            <EvidenceChip status="confirmed" label={entry.id} />
          </div>
          <div class="meta">
            <span class="created">{entry.createdAt}</span>
            <div class="tags">
              {#each entry.tags as tag}
                <EvidenceChip status="uncertain" label={tag} />
              {/each}
            </div>
          </div>
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
  .entries { display: grid; gap: var(--space-md); }
  .entry {
    padding: var(--space-md);
    border-radius: var(--radius-lg);
    background: var(--color-surface-1);
    display: grid;
    gap: var(--space-sm);
  }
  .entry-head { display: flex; justify-content: space-between; align-items: center; gap: var(--space-sm); }
  .meta { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-sm); }
  .created { font-size: var(--font-size-xs); font-family: var(--font-mono, monospace); color: var(--color-text-secondary); }
  .tags { display: flex; flex-wrap: wrap; gap: 6px; }
</style>
