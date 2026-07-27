<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";

  type Position = {
    symbol: string;
    side: string;
    quantity: number;
    avgCost: number;
    mark: number;
    unrealizedPnl: number;
    avgCostSource: string;
    multiplier: number;
  };

  type Snapshot = {
    currency: string;
    totalEquity: number;
    cash: number;
    unrealizedPnl: number;
    realizedPnl: number;
    provider: string;
    retrievedAt: string;
    positions: Position[];
  };

  type CheckRow = {
    id: string;
    label: string;
    severity: "hard_deny" | "soft_warn";
    status: "pass" | "warn" | "deny" | "pending";
  };

  const fallback: Snapshot = {
    currency: "USD",
    totalEquity: 500_000,
    cash: 125_000,
    unrealizedPnl: 0,
    realizedPnl: 0,
    provider: "ui-static-fallback",
    retrievedAt: "2026-07-25T20:00:00Z",
    positions: [
      {
        symbol: "AAPL",
        side: "long",
        quantity: 100,
        avgCost: 180,
        mark: 214.95,
        unrealizedPnl: 3_495,
        avgCostSource: "stub",
        multiplier: 1,
      },
      {
        symbol: "MSFT",
        side: "long",
        quantity: 80,
        avgCost: 380,
        mark: 420,
        unrealizedPnl: 3_200,
        avgCostSource: "stub",
        multiplier: 1,
      },
      {
        symbol: "SPY",
        side: "long",
        quantity: 40,
        avgCost: 500,
        mark: 548.22,
        unrealizedPnl: 1_928.8,
        avgCostSource: "stub",
        multiplier: 1,
      },
      {
        symbol: "TLT",
        side: "short",
        quantity: -200,
        avgCost: 95,
        mark: 92.5,
        unrealizedPnl: 500,
        avgCostSource: "stub",
        multiplier: 1,
      },
      {
        symbol: "BTC-USD",
        side: "long",
        quantity: 1,
        avgCost: 58_000,
        mark: 67_842,
        unrealizedPnl: 9_842,
        avgCostSource: "stub",
        multiplier: 1,
      },
    ],
  };

  const riskChecks: CheckRow[] = [
    { id: "max_position", label: "Max position", severity: "hard_deny", status: "pass" },
    { id: "max_premium", label: "Max premium", severity: "hard_deny", status: "pass" },
    { id: "max_account_loss", label: "Max account loss", severity: "hard_deny", status: "pass" },
    { id: "sector_concentration", label: "Sector concentration", severity: "hard_deny", status: "warn" },
    { id: "account_permissions", label: "Account permissions", severity: "hard_deny", status: "pass" },
    { id: "market_status", label: "Market status", severity: "hard_deny", status: "pass" },
    { id: "stale_data", label: "Stale data", severity: "hard_deny", status: "pass" },
    { id: "duplicate_order", label: "Duplicate order", severity: "hard_deny", status: "pass" },
    { id: "adjusted_contract", label: "Adjusted contract", severity: "hard_deny", status: "pass" },
    { id: "calendar_mismatch", label: "Calendar mismatch", severity: "hard_deny", status: "pass" },
    { id: "model_suppressed", label: "Model suppressed", severity: "hard_deny", status: "pass" },
    { id: "correlation_cluster", label: "Correlation cluster", severity: "soft_warn", status: "warn" },
    { id: "event_proximity", label: "Event proximity", severity: "soft_warn", status: "pending" },
    { id: "spread_width", label: "Spread width", severity: "soft_warn", status: "pass" },
    { id: "liquidity_volume", label: "Liquidity / volume", severity: "soft_warn", status: "pass" },
    { id: "open_interest", label: "Open interest", severity: "soft_warn", status: "pass" },
  ];

  const scenarios = [
    { id: "rates_up_100bp", label: "Rates +100 bp parallel shift" },
    { id: "spy_down_10", label: "SPY −10% shock" },
    { id: "btc_halving", label: "BTC halving stress" },
  ];

  let snapshot = $state<Snapshot>(fallback);
  let provenance = $state("static fallback");
  let selectedScenario = $state(scenarios[0].id);
  let scenarioNotice = $state<string | null>(null);

  const equityLabel = $derived(
    `${snapshot.currency} $${snapshot.totalEquity.toLocaleString(undefined, { maximumFractionDigits: 0 })}`,
  );

  function runScenario() {
    const label = scenarios.find((s) => s.id === selectedScenario)?.label ?? selectedScenario;
    scenarioNotice = `Scenario "${label}" queued — runner stub only (no live broker).`;
    setTimeout(() => (scenarioNotice = null), 2800);
  }

  function formatUsd(n: number): string {
    const sign = n < 0 ? "-" : "";
    return `${sign}$${Math.abs(n).toLocaleString(undefined, { maximumFractionDigits: 2 })}`;
  }

  function statusChip(status: CheckRow["status"]): "confirmed" | "uncertain" | "contradicted" {
    if (status === "pass") return "confirmed";
    if (status === "deny") return "contradicted";
    return "uncertain";
  }

  onMount(async () => {
    try {
      snapshot = await invoke<Snapshot>("get_portfolio_snapshot");
      provenance = "trusted core";
    } catch {
      provenance = "browser fallback";
    }
  });
</script>

<svelte:head><title>Portfolio · PRISMATIK</title></svelte:head>

<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}
    <div class="rail-label">Experiences</div>
    <ExperiencesNav active="portfolio" />
  {/snippet}
  {#snippet status()}
    <EvidenceChip status="confirmed" label={provenance} />
  {/snippet}

  <div class="canvas">
    <header class="page-head">
      <div>
        <h1>Portfolio</h1>
        <p>
          Positions from audit ledger projections — static stub until Wave 4 write path connects;
          IPC overlay when the stub floor is available.
        </p>
      </div>
      <div class="chips">
        <EvidenceChip status="confirmed" label={snapshot.provider} />
        <EvidenceChip status="confirmed" label={equityLabel} />
        <StaleDataMarker eventTime={snapshot.retrievedAt} maxAge={86_400_000} />
      </div>
    </header>

    <section class="panel">
      <div class="label">Positions</div>
      <table aria-label="Portfolio positions">
        <thead>
          <tr>
            <th>Symbol</th>
            <th>Side</th>
            <th class="num">Qty</th>
            <th class="num">Avg cost</th>
            <th class="num">Mark</th>
            <th class="num">uPnL</th>
            <th>Source</th>
          </tr>
        </thead>
        <tbody>
          {#each snapshot.positions as row}
            <tr>
              <td><strong class="sym">{row.symbol}</strong></td>
              <td>
                <EvidenceChip
                  status={row.side === "long" ? "confirmed" : "uncertain"}
                  label={row.side}
                />
              </td>
              <td class="num">{row.quantity}</td>
              <td class="num">{formatUsd(row.avgCost)}</td>
              <td class="num">{formatUsd(row.mark)}</td>
              <td class="num" data-tone={row.unrealizedPnl < 0 ? "down" : undefined}>
                {formatUsd(row.unrealizedPnl)}
              </td>
              <td><EvidenceChip status="uncertain" label={row.avgCostSource} /></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>

    <div class="split">
      <section class="panel" aria-labelledby="risk-panel-title">
        <div class="label" id="risk-panel-title">Risk · 16-check catalogue</div>
        <p class="panel-note">
          Fixed evaluation order from <code>prismatik-risk</code> — HardDeny before SoftWarn.
        </p>
        <ol class="checks">
          {#each riskChecks as check}
            <li id={check.id}>
              <a class="check-link" href="#{check.id}">
                <span class="check-name">{check.label}</span>
                <span class="check-meta">{check.id} · {check.severity.replace("_", " ")}</span>
              </a>
              <EvidenceChip status={statusChip(check.status)} label={check.status} />
            </li>
          {/each}
        </ol>
      </section>

      <section class="panel" aria-labelledby="scenario-title">
        <div class="label" id="scenario-title">Scenario runner</div>
        <p class="panel-note">
          Stress paths over the exposure stub — no live broker or order submission.
        </p>
        <div class="scenario-form">
          <label>
            Scenario
            <select bind:value={selectedScenario}>
              {#each scenarios as scenario}
                <option value={scenario.id}>{scenario.label}</option>
              {/each}
            </select>
          </label>
          <button type="button" class="text-action" onclick={runScenario}>Run scenario (stub)</button>
        </div>
        <div class="scenario-placeholder" role="status">
          <strong>Output pending</strong>
          <p>
            PnL ladder, factor shocks, and signed scenario manifest will render here when the
            simulation kernel is wired.
          </p>
        </div>
      </section>
    </div>
  </div>
</WorkspaceShell>

{#if scenarioNotice}
  <div class="notice" role="status">{scenarioNotice}</div>
{/if}

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
    margin-bottom: var(--space-5);
    border-top: 1px solid var(--color-border-default);
  }
  .panel-note {
    margin: 0 0 var(--space-4);
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
  }
  table {
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
  td {
    padding: 12px var(--space-3);
    border-bottom: 1px solid var(--color-border-default);
    vertical-align: middle;
  }
  .sym {
    font-family: var(--font-mono);
  }
  .num {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  th.num {
    text-align: right;
  }
  [data-tone="down"] {
    color: var(--color-down);
  }
  .split {
    display: grid;
    grid-template-columns: 1.2fr 1fr;
    gap: var(--space-5);
  }
  .checks {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .checks li {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--space-3);
    align-items: center;
    padding: var(--space-3) 0;
    border-bottom: 1px solid var(--color-border-default);
  }
  .check-link {
    color: inherit;
    text-decoration: none;
  }
  .check-link:hover .check-name {
    color: var(--color-brand-primary);
  }
  .check-name {
    display: block;
    font-weight: var(--font-weight-semibold);
    font-size: var(--font-size-sm);
  }
  .check-meta {
    display: block;
    margin-top: 2px;
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
  }
  code {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
  }
  .scenario-form {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4);
    align-items: end;
    margin-bottom: var(--space-4);
  }
  .scenario-form label {
    display: grid;
    gap: var(--space-1);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  select {
    min-width: 220px;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-md);
    background: var(--color-surface-1);
    color: var(--color-text-primary);
  }
  .text-action {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-md);
    background: var(--color-surface-1);
    color: var(--color-brand-primary);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
  }
  .scenario-placeholder {
    padding: var(--space-4);
    border: 1px dashed var(--color-border-default);
    border-radius: var(--radius-md);
    background: var(--color-surface-1);
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
  }
  .scenario-placeholder strong {
    display: block;
    margin-bottom: var(--space-2);
    color: var(--color-text-primary);
  }
  .scenario-placeholder p {
    margin: 0;
    line-height: var(--line-height-relaxed);
  }
  .notice {
    position: fixed;
    right: var(--space-5);
    bottom: var(--space-5);
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface-1);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }
  @media (max-width: 900px) {
    .split {
      grid-template-columns: 1fr;
    }
  }
</style>
