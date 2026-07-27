<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";

  type ModelCard = {
    modelId: string;
    family: string;
    familyRole?: string;
    onboardingStatus?: string;
    licenseSpdx?: string;
    driftStatus: string;
    driftAction?: string;
    ladderRung?: string;
    promotionOk?: boolean;
    blindSpots: string[];
    provider: string;
    retrievedAt: string;
  };

  const fallback: ModelCard = {
    modelId: "probabilistic-baseline-demo",
    family: "probabilistic_forecast",
    familyRole: "ProbabilisticBaseline",
    onboardingStatus: "onboarded",
    licenseSpdx: "Apache-2.0",
    driftStatus: "watch",
    driftAction: "annotate",
    ladderRung: "r4",
    promotionOk: true,
    blindSpots: [
      "Regime shifts after macro shocks (2020-style)",
      "Thin liquidity in far-dated options",
      "Corporate action gaps in symbology resolver",
    ],
    provider: "ui-static-fallback",
    retrievedAt: "2026-07-25T20:00:00Z",
  };

  let card = $state<ModelCard>(fallback);
  let provenance = $state("static fallback");

  const driftChip = $derived(
    card.driftStatus === "stable"
      ? "confirmed"
      : card.driftStatus === "watch"
        ? "uncertain"
        : "contradicted",
  );

  onMount(async () => {
    try {
      card = await invoke<ModelCard>("get_model_card", {
        modelId: "probabilistic-baseline-demo",
      });
      provenance = "trusted core";
    } catch {
      provenance = "browser fallback";
    }
  });
</script>

<svelte:head><title>Models · PRISMATIK</title></svelte:head>
<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}<div class="rail-label">Experiences</div><ExperiencesNav active="models" />{/snippet}
  {#snippet status()}<EvidenceChip status="confirmed" label={provenance} />{/snippet}
  <div class="canvas">
    <header>
      <div>
        <h1>Model card</h1>
        <p>Drift status and blind-spot disclosure surface — registry floor, not live monitoring.</p>
      </div>
      <div class="chips">
        <EvidenceChip status="confirmed" label={card.provider} />
        <StaleDataMarker eventTime={card.retrievedAt} maxAge={86_400_000} />
      </div>
    </header>

    <section class="grid">
      <div class="panel">
        <div class="label">Identity</div>
        <dl>
          <div><dt>Model ID</dt><dd>{card.modelId}</dd></div>
          <div><dt>Family</dt><dd>{card.familyRole ?? card.family}</dd></div>
          <div><dt>License</dt><dd>{card.licenseSpdx ?? "—"}</dd></div>
          <div><dt>Ladder</dt><dd>{card.ladderRung ?? "—"} · promote {card.promotionOk ? "ok" : "denied"}</dd></div>
          <div>
            <dt>Drift status</dt>
            <dd><EvidenceChip status={driftChip} label={card.driftStatus} /></dd>
          </div>
          <div><dt>Drift action</dt><dd>{card.driftAction ?? "—"}</dd></div>
        </dl>
      </div>
      <aside class="panel">
        <div class="label">Blind spots</div>
        <ul>
          {#each card.blindSpots as spot}
            <li>
              <EvidenceChip status="contradicted" label="disclosed" />
              <span>{spot}</span>
            </li>
          {/each}
        </ul>
        <EvidenceChip status="uncertain" label="P5-EX-03 scaffold" />
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
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-md); }
  .panel { padding: var(--space-md); border-radius: var(--radius-lg); background: var(--color-surface-1); display: grid; gap: var(--space-sm); }
  .label { font-size: var(--font-size-xs); text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-text-secondary); }
  dl { margin: 0; display: grid; gap: 12px; }
  dt { font-size: var(--font-size-xs); color: var(--color-text-tertiary); text-transform: uppercase; letter-spacing: 0.05em; }
  dd { margin: 4px 0 0; font-size: var(--font-size-sm); }
  ul { margin: 0; padding: 0; list-style: none; display: grid; gap: 10px; }
  li { padding: 10px; border-radius: var(--radius-md); background: var(--color-surface-2); display: grid; gap: 6px; }
  li span { font-size: var(--font-size-sm); color: var(--color-text-primary); }
  @media (max-width: 900px) { .grid { grid-template-columns: 1fr; } }
</style>
