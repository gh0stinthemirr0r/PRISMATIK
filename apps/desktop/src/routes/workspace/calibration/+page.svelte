<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, Metric, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";

  type Point = { x: number; y: number };
  type Ribbon = {
    nominal: number;
    realizedCoverage: number;
    points: Point[];
    provider: string;
    retrievedAt: string;
  };

  const fallback: Ribbon = {
    nominal: 0.9,
    realizedCoverage: 0.872,
    points: [
      { x: 0.5, y: 0.512 },
      { x: 0.6, y: 0.598 },
      { x: 0.7, y: 0.681 },
      { x: 0.8, y: 0.774 },
      { x: 0.9, y: 0.872 },
      { x: 0.95, y: 0.918 },
    ],
    provider: "ui-static-fallback",
    retrievedAt: "2026-07-25T20:00:00Z",
  };

  let ribbon = $state<Ribbon>(fallback);
  let provenance = $state("static fallback");

  const gap = $derived((ribbon.realizedCoverage - ribbon.nominal) * 100);
  const gapLabel = $derived(`${gap >= 0 ? "+" : ""}${gap.toFixed(1)} pp`);

  function ribbonPath(points: Point[], width: number, height: number): string {
    if (points.length === 0) return "";
    return points
      .map((p, i) => {
        const x = p.x * width;
        const y = height - p.y * height;
        return `${i === 0 ? "M" : "L"} ${x.toFixed(1)} ${y.toFixed(1)}`;
      })
      .join(" ");
  }

  const curvePath = $derived(ribbonPath(ribbon.points, 480, 160));
  const diagonalPath = "M 0 160 L 480 0";

  onMount(async () => {
    try {
      ribbon = await invoke<Ribbon>("get_calibration_ribbon");
      provenance = "trusted core";
    } catch {
      provenance = "browser fallback";
    }
  });
</script>

<svelte:head><title>Calibration · PRISMATIK</title></svelte:head>
<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}<div class="rail-label">Experiences</div><ExperiencesNav active="calibration" />{/snippet}
  {#snippet status()}<EvidenceChip status="confirmed" label={provenance} />{/snippet}
  <div class="canvas">
    <header>
      <div>
        <h1>Calibration ribbon</h1>
        <p>Nominal vs realized coverage ladder — I2 calibration floor, not live model scoring.</p>
      </div>
      <div class="chips">
        <EvidenceChip status="confirmed" label={ribbon.provider} />
        <StaleDataMarker eventTime={ribbon.retrievedAt} maxAge={86_400_000} />
      </div>
    </header>

    <section class="metric-strip" aria-label="Calibration summary">
      <Metric label="Nominal coverage" value={(ribbon.nominal * 100).toFixed(0)} unit="%" />
      <Metric label="Realized coverage" value={(ribbon.realizedCoverage * 100).toFixed(1)} unit="%" />
      <Metric label="Gap" value={gapLabel} delta="nominal − realized" />
      <Metric label="Ladder points" value={String(ribbon.points.length)} delta="confidence bins" />
    </section>

    <section class="panel">
      <div class="label">Coverage ladder</div>
      <div class="chart" aria-label="Calibration ribbon chart">
        <svg viewBox="0 0 480 160" role="img">
          <path d={diagonalPath} fill="none" stroke="var(--color-border-default)" stroke-dasharray="4 4" />
          <path d={curvePath} fill="none" stroke="var(--color-brand-primary)" stroke-width="2" />
          {#each ribbon.points as pt}
            <circle
              cx={pt.x * 480}
              cy={160 - pt.y * 160}
              r="4"
              fill="var(--color-brand-primary)"
            />
          {/each}
        </svg>
        <p class="note">Diagonal = perfect calibration · points = realized coverage at nominal confidence</p>
      </div>
      <EvidenceChip status="uncertain" label="P5-EX-02 scaffold" />
    </section>
  </div>
</WorkspaceShell>

<style>
  .canvas { display: grid; gap: var(--space-lg); }
  header { display: flex; justify-content: space-between; align-items: flex-start; gap: var(--space-md); }
  h1 { margin: 0; font-size: var(--font-size-xl); }
  p { margin: 4px 0 0; color: var(--color-text-secondary); }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; justify-content: flex-end; }
  .metric-strip {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-md);
    padding: var(--space-md) 0;
    border-top: 1px solid var(--color-border-default);
    border-bottom: 1px solid var(--color-border-default);
  }
  .panel { display: grid; gap: var(--space-sm); }
  .label { font-size: var(--font-size-xs); text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-text-secondary); }
  .chart {
    padding: var(--space-md);
    border-radius: var(--radius-lg);
    background: var(--color-surface-1);
    border: 1px dashed var(--color-border-default);
  }
  .chart svg { display: block; width: 100%; height: auto; }
  .note { margin: var(--space-sm) 0 0; font-size: var(--font-size-xs); color: var(--color-text-tertiary); font-family: var(--font-mono); }
  @media (max-width: 900px) { .metric-strip { grid-template-columns: 1fr 1fr; } }
</style>
