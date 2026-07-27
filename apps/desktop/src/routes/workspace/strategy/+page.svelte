<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";

  type Block = { id: string; kind: string; params: string[] };
  type Preview = {
    strategyId: string;
    schemaVersion?: string;
    irDigest?: string;
    dsl: string;
    blocks: Block[];
    ruleKind?: string;
    capabilities?: {
      network: boolean;
      filesystem: boolean;
      ai: boolean;
      maxComputePerBarMs: number;
    };
    provider: string;
    retrievedAt: string;
  };

  const fallback: Preview = {
    strategyId: "sma-cross-v1",
    dsl: `strategy sma-cross-v1 {
  feed equity("AAPL", daily)
  sma_fast = sma(close, 10)
  ema_trend = ema(close, 20)
  rsi_filter = rsi(close, 14)
  signal = cross_over(sma_fast, ema_trend) and rsi_filter < 70
  emit signal
}`,
    blocks: [
      { id: "feed-1", kind: "EquityFeed", params: ["AAPL", "daily"] },
      { id: "sma-10", kind: "SMA", params: ["close", "10"] },
      { id: "ema-20", kind: "EMA", params: ["close", "20"] },
      { id: "rsi-14", kind: "RSI", params: ["close", "14"] },
      { id: "cross-1", kind: "CrossOver", params: ["sma-10", "ema-20"] },
    ],
    provider: "ui-static-fallback",
    retrievedAt: "2026-07-25T20:00:00Z",
  };

  let preview = $state<Preview>(fallback);
  let provenance = $state("static fallback");

  onMount(async () => {
    try {
      preview = await invoke<Preview>("get_strategy_ir_preview", { strategyId: "sma-cross-v1" });
      provenance = "trusted core";
    } catch {
      provenance = "browser fallback";
    }
  });
</script>

<svelte:head><title>Strategy · PRISMATIK</title></svelte:head>
<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}<div class="rail-label">Experiences</div><ExperiencesNav active="strategy" />{/snippet}
  {#snippet status()}<EvidenceChip status="confirmed" label={provenance} />{/snippet}
  <div class="canvas">
    <header>
      <div>
        <h1>Strategy builder</h1>
        <p>Visual block list + DSL preview — emits StrategyIR via codegen floor, not live compile.</p>
      </div>
      <div class="chips">
        <EvidenceChip status="confirmed" label={preview.provider} />
        <StaleDataMarker eventTime={preview.retrievedAt} maxAge={86_400_000} />
      </div>
    </header>
    <section class="grid">
      <div class="panel">
        <div class="label">Blocks</div>
        <ul>
          {#each preview.blocks as block}
            <li>
              <strong>{block.kind}</strong>
              <span>{block.id}</span>
              <div class="params">
                {#each block.params as p}<EvidenceChip status="uncertain" label={p} />{/each}
              </div>
            </li>
          {/each}
        </ul>
      </div>
      <aside class="panel">
        <div class="label">DSL preview · {preview.strategyId}</div>
        <textarea readonly rows="12">{preview.dsl}</textarea>
        <div class="label">IR floor</div>
        <dl class="meta">
          <div><dt>Schema</dt><dd>{preview.schemaVersion ?? "—"}</dd></div>
          <div><dt>Digest</dt><dd><code>{preview.irDigest ?? "—"}</code></dd></div>
          <div><dt>Rule</dt><dd>{preview.ruleKind ?? "—"}</dd></div>
        </dl>
        {#if preview.capabilities}
          <div class="cap-row">
            <EvidenceChip status={preview.capabilities.network ? "contradicted" : "confirmed"} label="network" />
            <EvidenceChip status={preview.capabilities.filesystem ? "contradicted" : "confirmed"} label="fs" />
            <EvidenceChip status={preview.capabilities.ai ? "uncertain" : "confirmed"} label="ai" />
          </div>
        {/if}
        <EvidenceChip status="uncertain" label="P4-EX-01 scaffold" />
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
  ul { margin: 0; padding: 0; list-style: none; display: grid; gap: 10px; }
  li { padding: 10px; border-radius: var(--radius-md); background: var(--color-surface-2); display: grid; gap: 4px; }
  li span { font-size: var(--font-size-sm); color: var(--color-text-secondary); }
  .params { display: flex; flex-wrap: wrap; gap: 4px; }
  textarea { width: 100%; font-family: var(--font-mono, monospace); font-size: var(--font-size-sm); background: var(--color-surface-2); border: none; border-radius: var(--radius-md); padding: var(--space-sm); color: var(--color-text-primary); resize: vertical; }
  .meta { margin: 0; display: grid; gap: 6px; font-size: var(--font-size-sm); }
  .meta div { display: grid; grid-template-columns: 72px 1fr; gap: 8px; }
  dt { color: var(--color-text-secondary); }
  dd { margin: 0; word-break: break-all; }
  code { font-size: var(--font-size-xs); }
  .cap-row { display: flex; flex-wrap: wrap; gap: 4px; }
</style>
