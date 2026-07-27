<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";

  type Payload = {
    seed: number;
    source?: string;
    terminalMean?: number;
    maxDrawdownProxy?: number;
    ruinProbability?: number;
    provider: string;
    retrievedAt: string;
    paths: number[][];
  };

  const fallback: Payload = {
    seed: 42001,
    provider: "ui-static-fallback",
    retrievedAt: "2026-07-25T20:00:00Z",
    paths: [
      [0, 0.012, -0.004, 0.018, 0.006, 0.021],
      [0, -0.008, -0.015, 0.003, 0.011, 0.009],
      [0, 0.005, 0.014, 0.022, 0.017, 0.025],
      [0, -0.002, 0.007, -0.011, 0.004, 0.013],
      [0, 0.019, 0.011, 0.028, 0.016, 0.031],
    ],
  };

  let data = $state<Payload>(fallback);
  let provenance = $state("static fallback");

  const terminalReturns = $derived(data.paths.map((p) => p.at(-1) ?? 0));
  const meanReturn = $derived(
    data.terminalMean != null
      ? data.terminalMean
      : terminalReturns.length === 0
        ? 0
        : terminalReturns.reduce((a, b) => a + b, 0) / terminalReturns.length,
  );

  /** Fixed-width bins for a tiny SVG histogram of terminal path returns. */
  const histogram = $derived.by(() => {
    const values = terminalReturns;
    if (values.length === 0) return [] as { label: string; count: number; heightPct: number }[];
    const min = Math.min(...values);
    const max = Math.max(...values);
    const bins = 5;
    const span = Math.max(max - min, 1e-9);
    const counts = Array.from({ length: bins }, () => 0);
    for (const v of values) {
      const idx = Math.min(bins - 1, Math.floor(((v - min) / span) * bins));
      counts[idx] += 1;
    }
    const peak = Math.max(...counts, 1);
    return counts.map((count, i) => {
      const lo = min + (span * i) / bins;
      const hi = min + (span * (i + 1)) / bins;
      return {
        label: `${(lo * 100).toFixed(1)}–${(hi * 100).toFixed(1)}%`,
        count,
        heightPct: (count / peak) * 100,
      };
    });
  });

  onMount(async () => {
    try {
      data = await invoke<Payload>("get_monte_carlo_paths");
      provenance = "trusted core";
    } catch {
      provenance = "browser fallback";
    }
  });
</script>

<svelte:head><title>Simulation · PRISMATIK</title></svelte:head>
<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}<div class="rail-label">Experiences</div><ExperiencesNav active="simulation" />{/snippet}
  {#snippet status()}<EvidenceChip status="confirmed" label={provenance} />{/snippet}
  <div class="canvas">
    <header>
      <div>
        <h1>Monte Carlo simulation</h1>
        <p>Path return histogram + table — wgpu path clouds deferred; seeded stub floor.</p>
      </div>
      <div class="chips">
        <EvidenceChip status="confirmed" label={data.provider} />
        <EvidenceChip status="uncertain" label="seed {data.seed}" />
        <StaleDataMarker eventTime={data.retrievedAt} maxAge={86_400_000} />
      </div>
    </header>
    <section class="metrics">
      <article>
        <span>Paths</span>
        <strong>{data.paths.length}</strong>
        <EvidenceChip status="confirmed" label="stub paths" />
      </article>
      <article>
        <span>Horizon steps</span>
        <strong>{data.paths[0]?.length ?? 0}</strong>
        <EvidenceChip status="uncertain" label="P5-EX-01 scaffold" />
      </article>
      <article>
        <span>Mean terminal return</span>
        <strong>{(meanReturn * 100).toFixed(2)}%</strong>
        <EvidenceChip status="confirmed" label={data.source ?? `seed ${data.seed}`} />
      </article>
      {#if data.ruinProbability != null}
        <article>
          <span>Ruin probability</span>
          <strong>{(data.ruinProbability * 100).toFixed(1)}%</strong>
          <EvidenceChip status="uncertain" label="summarize_terminals" />
        </article>
      {/if}
    </section>
    <section class="grid">
      <div class="panel">
        <div class="label">Terminal return histogram</div>
        <div class="hist" role="img" aria-label="Histogram of terminal path returns">
          {#each histogram as bin}
            <div class="bar-col">
              <div class="bar" style="height: {bin.heightPct}%" title="{bin.label}: {bin.count}"></div>
              <span>{bin.count}</span>
              <small>{bin.label}</small>
            </div>
          {/each}
        </div>
      </div>
      <aside class="panel">
        <div class="label">Path returns</div>
        <table>
          <thead>
            <tr>
              <th>Path</th>
              {#each data.paths[0] ?? [] as _, i}<th>t{i}</th>{/each}
              <th>Terminal</th>
            </tr>
          </thead>
          <tbody>
            {#each data.paths as path, idx}
              <tr>
                <td>{idx + 1}</td>
                {#each path as step}<td>{(step * 100).toFixed(2)}%</td>{/each}
                <td><strong>{((path.at(-1) ?? 0) * 100).toFixed(2)}%</strong></td>
              </tr>
            {/each}
          </tbody>
        </table>
        <EvidenceChip status="uncertain" label="distribution panels / wgpu deferred" />
      </aside>
    </section>
  </div>
</WorkspaceShell>

<style>
  .canvas { display: grid; gap: var(--space-lg); }
  header { display: flex; justify-content: space-between; align-items: flex-start; gap: var(--space-md); }
  h1 { margin: 0; font-size: var(--font-size-xl); }
  p { margin: 4px 0 0; color: var(--color-text-secondary); }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; justify-content: flex-end; }
  .metrics { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: var(--space-md); }
  article { padding: var(--space-md); border-radius: var(--radius-lg); background: var(--color-surface-1); display: grid; gap: 6px; }
  article span { font-size: var(--font-size-sm); color: var(--color-text-secondary); }
  article strong { font-size: var(--font-size-2xl); }
  .grid { display: grid; grid-template-columns: 1fr 1.4fr; gap: var(--space-md); }
  .panel { padding: var(--space-md); border-radius: var(--radius-lg); background: var(--color-surface-1); display: grid; gap: var(--space-sm); overflow-x: auto; }
  .label { font-size: var(--font-size-xs); text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-text-secondary); }
  .hist { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 8px; align-items: end; min-height: 160px; padding-top: 8px; }
  .bar-col { display: grid; grid-template-rows: 1fr auto auto; justify-items: center; gap: 4px; height: 140px; }
  .bar { width: 100%; max-width: 36px; align-self: end; border-radius: var(--radius-md) var(--radius-md) 0 0; background: var(--color-surface-2); min-height: 4px; }
  .bar-col span { font-size: var(--font-size-sm); font-weight: var(--font-weight-medium); }
  .bar-col small { font-size: 10px; color: var(--color-text-secondary); text-align: center; line-height: 1.2; }
  table { width: 100%; border-collapse: collapse; font-size: var(--font-size-sm); }
  th, td { padding: 6px 8px; text-align: right; border-bottom: 1px solid var(--color-border, #333); }
  th:first-child, td:first-child { text-align: left; }
</style>
