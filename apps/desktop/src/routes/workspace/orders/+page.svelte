<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";

  type Ticket = {
    symbol: string;
    side: string;
    quantity: number;
    orderType: string;
    limitPrice: number | null;
    timeInForce: string;
    idempotencyKey: string;
    signedQuantity?: number;
    riskOk?: boolean;
    riskFailure?: string | null;
    provider: string;
    retrievedAt: string;
  };

  const fallback: Ticket = {
    symbol: "AAPL",
    side: "buy",
    quantity: 10,
    orderType: "limit",
    limitPrice: 214.5,
    timeInForce: "day",
    idempotencyKey: "demo-idempotency-key-001",
    provider: "ui-static-fallback",
    retrievedAt: "2026-07-25T20:00:00Z",
  };

  let ticket = $state<Ticket>(fallback);
  let provenance = $state("static fallback");
  let symbol = $state("AAPL");
  let side = $state("buy");
  let quantity = $state(10);

  async function refresh() {
    try {
      ticket = await invoke<Ticket>("preview_order_ticket", { symbol, side, quantity });
      provenance = "trusted core";
    } catch {
      provenance = "browser fallback";
    }
  }

  onMount(refresh);
</script>

<svelte:head><title>Orders · PRISMATIK</title></svelte:head>
<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}<div class="rail-label">Experiences</div><ExperiencesNav active="orders" />{/snippet}
  {#snippet status()}<EvidenceChip status="confirmed" label={provenance} />{/snippet}
  <div class="canvas">
    <header>
      <div>
        <h1>Order ticket</h1>
        <p>Paper-only preview — idempotency key + risk gates enforced in core, not live broker.</p>
      </div>
      <EvidenceChip status="confirmed" label={ticket.provider} />
      <StaleDataMarker eventTime={ticket.retrievedAt} maxAge={86_400_000} />
    </header>
    <section class="grid">
      <form class="panel" onsubmit={(e) => { e.preventDefault(); refresh(); }}>
        <div class="label">Draft order</div>
        <label>Symbol <input bind:value={symbol} /></label>
        <label>Side
          <select bind:value={side}>
            <option value="buy">Buy</option>
            <option value="sell">Sell</option>
          </select>
        </label>
        <label>Quantity <input type="number" bind:value={quantity} min="1" /></label>
        <button type="submit">Preview ticket</button>
      </form>
      <aside class="panel">
        <div class="label">Preview</div>
        <dl>
          <div><dt>Symbol</dt><dd>{ticket.symbol}</dd></div>
          <div><dt>Side</dt><dd>{ticket.side}</dd></div>
          <div><dt>Qty</dt><dd>{ticket.quantity}</dd></div>
          <div><dt>Type</dt><dd>{ticket.orderType}</dd></div>
          <div><dt>Limit</dt><dd>{ticket.limitPrice != null ? `$${ticket.limitPrice.toFixed(2)}` : "—"}</dd></div>
          <div><dt>TIF</dt><dd>{ticket.timeInForce}</dd></div>
          <div><dt>Idempotency</dt><dd><code>{ticket.idempotencyKey}</code></dd></div>
          <div><dt>Signed qty</dt><dd>{ticket.signedQuantity ?? "—"}</dd></div>
          <div><dt>Risk</dt><dd>{ticket.riskOk === false ? (ticket.riskFailure ?? "denied") : ticket.riskOk === true ? "ok" : "—"}</dd></div>
        </dl>
        <EvidenceChip status={ticket.riskOk === false ? "contradicted" : "uncertain"} label="P7-EX-01 scaffold — paper broker only" />
      </aside>
    </section>
  </div>
</WorkspaceShell>

<style>
  .canvas { display: grid; gap: var(--space-lg); }
  header { display: flex; flex-wrap: wrap; gap: var(--space-sm); }
  h1 { margin: 0; width: 100%; }
  header p { margin: 4px 0 0; color: var(--color-text-secondary); width: 100%; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-md); }
  .panel { padding: var(--space-md); border-radius: var(--radius-lg); background: var(--color-surface-1); display: grid; gap: var(--space-sm); }
  .label { font-size: var(--font-size-xs); text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-text-secondary); }
  label { display: grid; gap: 4px; font-size: var(--font-size-sm); }
  input, select { padding: 8px; border-radius: var(--radius-md); border: 1px solid var(--color-border, #333); background: var(--color-surface-2); color: var(--color-text-primary); }
  button { padding: 10px 16px; border-radius: var(--radius-md); border: none; background: var(--color-accent, #4f8); color: #000; font-weight: 600; cursor: pointer; }
  dl { margin: 0; display: grid; gap: 8px; }
  dl div { display: grid; grid-template-columns: 120px 1fr; gap: 8px; }
  dt { color: var(--color-text-secondary); font-size: var(--font-size-sm); }
  dd { margin: 0; }
  code { font-size: var(--font-size-xs); word-break: break-all; }
</style>
