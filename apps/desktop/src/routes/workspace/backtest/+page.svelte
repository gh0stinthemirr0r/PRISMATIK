<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, Metric, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";

  type Summary = {
    strategyId: string;
    windowStart: string;
    windowEnd: string;
    fillModel: string;
    totalReturnPct: number;
    maxDrawdownPct: number;
    tradeCount: number;
    annualizedSharpe: number;
    manifestHash: string;
    manifestDigest?: string;
    lineageEvents?: number;
    provider: string;
    retrievedAt: string;
  };

  const fallback: Summary = {
    strategyId: "sma-cross-v1",
    windowStart: "2024-01-02T00:00:00Z",
    windowEnd: "2025-06-30T00:00:00Z",
    fillModel: "next_open",
    totalReturnPct: 12.0,
    maxDrawdownPct: -6.55,
    tradeCount: 0,
    annualizedSharpe: 1.24,
    manifestHash: "backtest.stub.v1",
    provider: "ui-static-fallback",
    retrievedAt: "2026-07-25T20:00:00Z",
  };

  /** Static stub equity curve (normalized account value). */
  const equityCurve = [
    1.0, 1.02, 1.018, 1.035, 1.05, 1.048, 1.062, 1.08, 1.075, 1.09, 1.105, 1.098, 1.12,
  ];

  const drawdownSeries = equityCurve.map((v, i) => {
    const peak = Math.max(...equityCurve.slice(0, i + 1));
    return ((v - peak) / peak) * 100;
  });

  let summary = $state<Summary>(fallback);
  let provenance = $state("static fallback");

  const totalReturn = $derived(summary.totalReturnPct.toFixed(2));
  const maxDrawdown = $derived(summary.maxDrawdownPct);
  const sharpeStub = $derived(summary.annualizedSharpe.toFixed(2));
  const manifestLabel = $derived(
    summary.manifestHash.startsWith("sha256:") || summary.manifestHash.startsWith("manifest:")
      ? summary.manifestHash
      : `manifest: ${summary.manifestHash}`,
  );

  function sparklinePath(values: number[], width: number, height: number): string {
    const min = Math.min(...values);
    const max = Math.max(...values);
    const span = max - min || 1;
    return values
      .map((v, i) => {
        const x = (i / (values.length - 1)) * width;
        const y = height - ((v - min) / span) * height;
        return `${i === 0 ? "M" : "L"} ${x.toFixed(1)} ${y.toFixed(1)}`;
      })
      .join(" ");
  }

  const equityPath = $derived(sparklinePath(equityCurve, 480, 120));
  const ddPath = $derived(sparklinePath(drawdownSeries, 480, 80));

  onMount(async () => {
    try {
      summary = await invoke<Summary>("get_backtest_summary", { strategyId: "sma-cross-v1" });
      provenance = "trusted core";
    } catch {
      provenance = "browser fallback";
    }
  });
</script>

<svelte:head><title>Backtest · PRISMATIK</title></svelte:head>

<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}
    <div class="rail-label">Experiences</div>
    <ExperiencesNav active="backtest" />
  {/snippet}
  {#snippet status()}
    <EvidenceChip status="confirmed" label={provenance} />
  {/snippet}

  <div class="canvas">
    <header class="page-head">
      <div>
        <h1>Backtest results</h1>
        <p>
          Equity curve, drawdown, and trade ledger for a signed manifest bundle — SVG demo with IPC
          metric overlay when the stub floor is available.
        </p>
      </div>
      <div class="chips">
        <EvidenceChip status="confirmed" label={summary.provider} />
        <EvidenceChip status="confirmed" label={manifestLabel} />
        {#if summary.manifestDigest}
          <EvidenceChip status="confirmed" label={summary.manifestDigest.slice(0, 24)} />
        {/if}
        {#if summary.lineageEvents != null && summary.lineageEvents > 0}
          <EvidenceChip status="confirmed" label="lineage {summary.lineageEvents} events" />
        {/if}
        <StaleDataMarker eventTime={summary.retrievedAt} maxAge={86_400_000} />
      </div>
    </header>

    <section class="metric-strip" aria-label="Backtest metrics">
      <Metric
        label="Total return"
        value={`${summary.totalReturnPct >= 0 ? "+" : ""}${totalReturn}`}
        unit="%"
      />
      <Metric label="Max drawdown" value={maxDrawdown.toFixed(2)} unit="%" delta="peak-to-trough" />
      <div class="metric-with-badge">
        <Metric label="Sharpe" value={sharpeStub} delta="annualized" />
        <span class="badge badge--warn">non-production</span>
      </div>
      <Metric
        label="Trades"
        value={String(summary.tradeCount)}
        delta={summary.tradeCount === 0 ? "empty ledger" : summary.fillModel}
      />
    </section>

    <section class="grid">
      <div class="panel">
        <div class="label">Equity curve</div>
        <div class="chart-placeholder" aria-label="Equity curve placeholder">
          <svg viewBox="0 0 480 120" role="img" aria-hidden="true">
            <path d={equityPath} fill="none" stroke="var(--color-brand-primary)" stroke-width="2" />
          </svg>
          <p class="placeholder-note">Normalized account value · demo cassette</p>
        </div>
      </div>

      <div class="panel">
        <div class="label">Drawdown</div>
        <div class="chart-placeholder chart-placeholder--dd" aria-label="Drawdown panel">
          <svg viewBox="0 0 480 80" role="img" aria-hidden="true">
            <path d={ddPath} fill="none" stroke="var(--color-down)" stroke-width="2" />
          </svg>
          <p class="placeholder-note">Underwater % from running peak · max {maxDrawdown.toFixed(2)}%</p>
        </div>
      </div>
    </section>

    <section class="panel panel--trades">
      <div class="label">Trade list</div>
      <div class="empty-state" role="status">
        <div class="empty-state__rule"></div>
        <strong>No fills recorded</strong>
        <p>
          Run a backtest from Strategy IR to populate the trade ledger. Fills carry slippage,
          commission micros, and manifest lineage when the engine is connected.
        </p>
      </div>
      <table class="trades-table" aria-label="Trade list (empty)">
        <thead>
          <tr>
            <th>Time</th>
            <th>Symbol</th>
            <th class="num">Qty</th>
            <th class="num">Price</th>
            <th class="num">PnL</th>
            <th>Evidence</th>
          </tr>
        </thead>
        <tbody>
          <tr class="empty-row">
            <td colspan="6">— awaiting backtest run —</td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
</WorkspaceShell>

<style>
  .rail-label {
    margin: 0 0 var(--space-3);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .canvas {
    padding: var(--space-5) var(--space-6);
  }
  .page-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: var(--space-5);
    margin-bottom: var(--space-5);
  }
  .page-head h1 {
    margin: 0;
    font-size: var(--font-size-2xl);
    font-weight: var(--font-weight-semibold);
    letter-spacing: -0.03em;
  }
  .page-head p {
    max-width: 40rem;
    margin: var(--space-2) 0 0;
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    justify-content: flex-end;
  }
  .metric-strip {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-4);
    padding: var(--space-4) 0;
    margin-bottom: var(--space-5);
    border-top: 1px solid var(--color-border-default);
    border-bottom: 1px solid var(--color-border-default);
  }
  .metric-with-badge {
    display: grid;
    gap: var(--space-2);
    align-content: start;
  }
  .badge {
    width: fit-content;
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .badge--warn {
    border: 1px solid var(--color-warning);
    color: var(--color-warning);
    background: color-mix(in oklab, var(--color-surface-1) 90%, var(--color-warning));
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-5);
    margin-bottom: var(--space-5);
  }
  .label {
    margin-bottom: var(--space-3);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .panel {
    padding-top: var(--space-4);
    border-top: 1px solid var(--color-border-default);
  }
  .panel--trades {
    margin-top: var(--space-2);
  }
  .chart-placeholder {
    padding: var(--space-4);
    border: 1px dashed var(--color-border-default);
    border-radius: var(--radius-md);
    background: var(--color-surface-1);
  }
  .chart-placeholder svg {
    display: block;
    width: 100%;
    height: auto;
  }
  .chart-placeholder--dd {
    border-color: color-mix(in oklab, var(--color-border-default) 70%, var(--color-down));
  }
  .placeholder-note {
    margin: var(--space-3) 0 0;
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    font-family: var(--font-mono);
  }
  .empty-state {
    display: grid;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
    padding: var(--space-4);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-md);
    background: var(--color-surface-1);
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
  }
  .empty-state__rule {
    width: 48px;
    height: 2px;
    background: var(--color-brand-primary);
  }
  .empty-state strong {
    color: var(--color-text-primary);
  }
  .empty-state p {
    margin: 0;
    max-width: 36rem;
    line-height: var(--line-height-relaxed);
  }
  .trades-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-sm);
  }
  th {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border-default);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.05em;
    text-align: left;
    text-transform: uppercase;
  }
  .num {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  th.num {
    text-align: right;
  }
  .empty-row td {
    padding: var(--space-5) var(--space-3);
    color: var(--color-text-tertiary);
    text-align: center;
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  @media (max-width: 900px) {
    .metric-strip,
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
