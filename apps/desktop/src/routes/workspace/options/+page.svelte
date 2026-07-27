<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";
  import PerspectiveViewer from "$lib/perspective/PerspectiveViewer.svelte";
  import IvSurface from "$lib/iv-surface/IvSurface.svelte";
  import type { PerspectiveRow } from "$lib/perspective/bootstrap";

  type Contract = {
    occSymbol: string;
    expiration: string;
    strike: number;
    optionType: string;
    bid: number;
    ask: number;
    impliedVolatility: number;
    openInterest: number;
    delta: number;
    gamma: number;
    theta: number;
    vega: number;
    liquidityScore: number;
  };
  type Chain = {
    underlying: string;
    spot: number;
    provider: string;
    retrievedAt: string;
    contracts: Contract[];
  };
  type Print = {
    id: string;
    occSymbol: string;
    side: string;
    contracts: number;
    premium: number;
    classification: string;
    confidence: number;
    tradeQualityVersion: string;
    evidence: string;
    occurredAt: string;
  };
  type Flow = {
    underlying: string;
    provider: string;
    retrievedAt: string;
    prints: Print[];
  };
  type Dealer = {
    underlying: string;
    netGex: number;
    netDex: number;
    byStrike: Array<{ strike: number; gex: number; dex: number }>;
    provider: string;
    retrievedAt: string;
  };

  type Leg = { right: "call" | "put"; strike: number; premium: number; qty: number };

  const PREVIEW_AS_OF = "2026-07-27T09:30:00Z";
  const PREVIEW_EXPIRIES = ["2026-08-21", "2026-09-18", "2026-12-18"];
  const PREVIEW_STRIKES = [195, 205, 215, 225, 235, 245, 255];
  const PREVIEW_CONTRACTS: Contract[] = PREVIEW_EXPIRIES.flatMap((expiration, expiryIndex) =>
    PREVIEW_STRIKES.flatMap((strike) =>
      (["call", "put"] as const).map((optionType) => {
        const distance = (strike - 225) / 10;
        const isCall = optionType === "call";
        const mid = Math.max(0.45, 8.2 - Math.abs(distance) * 1.55 + expiryIndex * 2.4);
        const delta = isCall
          ? Math.max(0.08, Math.min(0.92, 0.52 - distance * 0.115))
          : -Math.max(0.08, Math.min(0.92, 0.48 + distance * 0.115));
        return {
          occSymbol: `AAPL${expiration.replaceAll("-", "").slice(2)}${isCall ? "C" : "P"}${String(strike * 1000).padStart(8, "0")}`,
          expiration,
          strike,
          optionType,
          bid: Number((mid - 0.08).toFixed(2)),
          ask: Number((mid + 0.08).toFixed(2)),
          impliedVolatility: Number((0.238 + Math.abs(distance) * 0.017 + expiryIndex * 0.011).toFixed(3)),
          openInterest: 1840 + (6 - Math.abs(distance)) * 620 + expiryIndex * 370,
          delta: Number(delta.toFixed(3)),
          gamma: Number((0.036 - Math.min(0.025, Math.abs(distance) * 0.004)).toFixed(3)),
          theta: Number((-0.042 - expiryIndex * 0.009 - Math.abs(distance) * 0.002).toFixed(3)),
          vega: Number((0.118 + expiryIndex * 0.032 - Math.abs(distance) * 0.006).toFixed(3)),
          liquidityScore: Number(Math.max(0.62, 0.96 - Math.abs(distance) * 0.055 - expiryIndex * 0.025).toFixed(2)),
        };
      }),
    ),
  );
  const PREVIEW_CHAIN: Chain = {
    underlying: "AAPL",
    spot: 224.86,
    provider: "OCC · OPRA cassette",
    retrievedAt: PREVIEW_AS_OF,
    contracts: PREVIEW_CONTRACTS,
  };
  const PREVIEW_FLOW: Flow = {
    underlying: "AAPL",
    provider: "OPRA flow cassette",
    retrievedAt: PREVIEW_AS_OF,
    prints: [
      {
        id: "flow-01",
        occSymbol: "AAPL260821C00225000",
        side: "ask",
        contracts: 1840,
        premium: 1545600,
        classification: "sweep · opening",
        confidence: 0.94,
        tradeQualityVersion: "tq-v3.4",
        evidence: "multi-exchange sequence · 42 ms",
        occurredAt: "2026-07-27T09:27:14Z",
      },
      {
        id: "flow-02",
        occSymbol: "AAPL260918P00215000",
        side: "mid",
        contracts: 720,
        premium: 486000,
        classification: "block · protective",
        confidence: 0.86,
        tradeQualityVersion: "tq-v3.4",
        evidence: "single-print size anomaly · 3.8σ",
        occurredAt: "2026-07-27T09:24:48Z",
      },
      {
        id: "flow-03",
        occSymbol: "AAPL261218C00245000",
        side: "ask",
        contracts: 1150,
        premium: 1017750,
        classification: "split · directional",
        confidence: 0.89,
        tradeQualityVersion: "tq-v3.4",
        evidence: "volatility-lift confirmation · +1.7 vol",
        occurredAt: "2026-07-27T09:21:06Z",
      },
    ],
  };
  const PREVIEW_DEALER: Dealer = {
    underlying: "AAPL",
    netGex: 184_720_000,
    netDex: -42_860_000,
    provider: "derived exposure cassette",
    retrievedAt: PREVIEW_AS_OF,
    byStrike: PREVIEW_STRIKES.map((strike, index) => ({
      strike,
      gex: (index - 2.4) * 18_400_000,
      dex: (3.2 - index) * 7_150_000,
    })),
  };

  function isTauriRuntime(): boolean {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  let chain = $state<Chain | null>(null);
  let flow = $state<Flow | null>(null);
  let dealer = $state<Dealer | null>(null);
  let legs = $state<Leg[]>([{ right: "call", strike: 220, premium: 5.55, qty: 1 }]);

  const chainRows = $derived<PerspectiveRow[]>(
    (chain?.contracts ?? []).map((row) => ({
      occ: row.occSymbol,
      type: row.optionType,
      expiration: row.expiration,
      strike: row.strike,
      bid: row.bid,
      ask: row.ask,
      iv: row.impliedVolatility,
      delta: row.delta,
      gamma: row.gamma,
      theta: row.theta,
      vega: row.vega,
      oi: row.openInterest,
      liq: row.liquidityScore,
    })),
  );

  const flowRows = $derived<PerspectiveRow[]>(
    (flow?.prints ?? []).map((row) => ({
      id: row.id,
      occ: row.occSymbol,
      side: row.side,
      contracts: row.contracts,
      premium: row.premium,
      classification: row.classification,
      confidence: row.confidence,
      tq: row.tradeQualityVersion,
      evidence: row.evidence,
      occurred_at: row.occurredAt,
    })),
  );

  const ivQuotes = $derived(
    (chain?.contracts ?? []).map((row) => ({
      expiration: row.expiration,
      strike: row.strike,
      optionType: row.optionType,
      impliedVolatility: row.impliedVolatility,
    })),
  );

  function payoffAt(spot: number): number {
    return legs.reduce((sum, leg) => {
      const intrinsic =
        leg.right === "call"
          ? Math.max(0, spot - leg.strike)
          : Math.max(0, leg.strike - spot);
      return sum + leg.qty * (intrinsic - leg.premium);
    }, 0);
  }

  function breakEvens(): number[] {
    const spots = Array.from({ length: 401 }, (_, i) => 100 + i * 0.5);
    const zeros: number[] = [];
    for (let i = 1; i < spots.length; i++) {
      const a = payoffAt(spots[i - 1]);
      const b = payoffAt(spots[i]);
      if (a === 0) zeros.push(spots[i - 1]);
      else if (a * b < 0) zeros.push(spots[i - 1] - (a * (spots[i] - spots[i - 1])) / (b - a));
    }
    return zeros.slice(0, 4);
  }

  function payoffPoints(): string {
    return Array.from({ length: 101 }, (_, i) => {
      const spot = 160 + i;
      const y = 95 - payoffAt(spot) * 4;
      return `${20 + i * 4.8},${Math.max(10, Math.min(180, y))}`;
    }).join(" ");
  }

  onMount(async () => {
    if (!isTauriRuntime()) {
      chain = PREVIEW_CHAIN;
      flow = PREVIEW_FLOW;
      dealer = PREVIEW_DEALER;
      return;
    }
    [chain, flow, dealer] = await Promise.all([
      invoke<Chain>("get_options_chain", { underlying: "AAPL" }),
      invoke<Flow>("get_options_flow", { underlying: "AAPL" }),
      invoke<Dealer>("get_dealer_exposure", { underlying: "AAPL" }),
    ]);
  });
</script>

<svelte:head><title>Options · PRISMATIK</title></svelte:head>
<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}<div class="rail-label">Experiences</div><ExperiencesNav active="options" />{/snippet}
  {#snippet status()}<EvidenceChip status="confirmed" label="OCC / OPRA fixtures" />{/snippet}
  <div class="canvas">
    <header>
      <div>
        <h1>Options intelligence</h1>
        <p>Perspective chain/flow grids, WebGPU-oriented IV surface, multi-leg payoff, and dealer exposure.</p>
      </div>
      {#if chain}
        <div class="spot">
          ${chain.spot.toFixed(2)}
          <EvidenceChip status="confirmed" label={chain.provider} />
          <StaleDataMarker eventTime={chain.retrievedAt} maxAge={86_400_000} />
        </div>
      {/if}
    </header>

    <section>
      <div class="section-head">
        <h2>Chain explorer</h2>
        <EvidenceChip status="confirmed" label="Perspective datagrid" />
      </div>
      <PerspectiveViewer rows={chainRows} height={340} title="Options chain" />
    </section>

    <div class="split">
      <section>
        <div class="section-head">
          <h2>Flow prints</h2>
          <EvidenceChip status="confirmed" label="Perspective datagrid" />
        </div>
        <PerspectiveViewer rows={flowRows} height={300} title="Options flow" />
        {#each flow?.prints ?? [] as row}
          <article class="flow">
            <div>
              <code>{row.occSymbol}</code>
              <p>{row.classification} · {row.side}</p>
              <small>{row.evidence}</small>
            </div>
            <strong>{row.contracts.toLocaleString()} · ${row.premium.toLocaleString()}</strong>
            <div>
              <EvidenceChip status={row.confidence >= 0.8 ? "confirmed" : "uncertain"} label={`${(row.confidence * 100).toFixed(0)}% confidence`} />
              <small>{row.tradeQualityVersion}</small>
              <StaleDataMarker eventTime={row.occurredAt} maxAge={86_400_000} />
            </div>
          </article>
        {/each}
      </section>
      <section>
        <h2>Volatility lab</h2>
        <IvSurface quotes={ivQuotes} />
        <div class="term">
          {#each [...new Set((chain?.contracts ?? []).map((c) => c.expiration))].sort() as expiry}
            {@const ivs = (chain?.contracts ?? []).filter((c) => c.expiration === expiry).map((c) => c.impliedVolatility)}
            <div>
              <span>{expiry}</span>
              <strong>{((ivs.reduce((a, b) => a + b, 0) / Math.max(1, ivs.length)) * 100).toFixed(1)}% avg IV</strong>
            </div>
          {/each}
        </div>
        <EvidenceChip status="confirmed" label="WebGPU preferred · Canvas mesh fallback" />
      </section>
    </div>

    <div class="split">
      <section>
        <h2>Strategy constructor</h2>
        <div class="constructor">
          <div class="legs">
            {#each legs as leg, index}
              <label>
                Leg {index + 1}
                <select bind:value={leg.right}>
                  <option value="call">Call</option>
                  <option value="put">Put</option>
                </select>
                <input type="number" bind:value={leg.strike} step="1" />
                <input type="number" bind:value={leg.premium} step="0.05" />
                <input type="number" bind:value={leg.qty} step="1" />
              </label>
            {/each}
            <button type="button" onclick={() => (legs = [...legs, { right: "put", strike: 210, premium: 4.9, qty: 1 }])}>Add leg</button>
            <p>Break-evens: {breakEvens().map((x) => `$${x.toFixed(2)}`).join(" · ") || "none in band"}</p>
            <EvidenceChip status="confirmed" label="deterministic payoff" />
          </div>
          <svg class="payoff" viewBox="0 0 520 190" role="img" aria-label="multi-leg payoff">
            <line x1="20" y1="95" x2="500" y2="95" />
            <line x1="260" y1="15" x2="260" y2="175" />
            <polyline points={payoffPoints()} />
          </svg>
        </div>
      </section>
      <section>
        <h2>Dealer exposure</h2>
        {#if dealer}
          <div class="dealer-head">
            <div><span>Net GEX</span><strong>{dealer.netGex.toExponential(2)}</strong></div>
            <div><span>Net DEX</span><strong>{dealer.netDex.toExponential(2)}</strong></div>
            <EvidenceChip status="confirmed" label={dealer.provider} />
          </div>
          <div class="table dealer">
            <div class="tr th"><span>Strike</span><span>GEX</span><span>DEX</span></div>
            {#each dealer.byStrike as row}
              <div class="tr">
                <span>${row.strike.toFixed(0)}</span>
                <span>{row.gex.toExponential(2)}</span>
                <span>{row.dex.toExponential(2)}</span>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    </div>
  </div>
</WorkspaceShell>

<style>
  .rail-label,h2{margin:0 0 var(--space-3);color:var(--color-text-tertiary);font-size:var(--font-size-xs);letter-spacing:.06em;text-transform:uppercase}
  .canvas{padding:var(--space-5) var(--space-6)}
  header{display:flex;justify-content:space-between;align-items:end;margin-bottom:var(--space-6)}
  h1{margin:0;font-size:var(--font-size-2xl)}
  header p,.flow p,.constructor p,small{color:var(--color-text-secondary);font-size:var(--font-size-sm)}
  .spot{display:grid;gap:var(--space-2);font:600 var(--font-size-xl) var(--font-mono)}
  section{margin-top:var(--space-6)}
  .section-head{display:flex;justify-content:space-between;align-items:center;margin-bottom:var(--space-3)}
  .section-head h2{margin:0}
  .table{overflow:auto}
  .tr{display:grid;grid-template-columns:minmax(180px,1.4fr) .5fr .6fr 1fr .5fr repeat(4,.55fr) .6fr .5fr;gap:var(--space-2);align-items:center;padding:var(--space-2) var(--space-3);border-bottom:1px solid var(--color-border-default);font:var(--font-size-sm) var(--font-mono)}
  .dealer .tr{grid-template-columns:1fr 1fr 1fr}
  .th{color:var(--color-text-tertiary);font-size:var(--font-size-xs);text-transform:uppercase}
  .split{display:grid;grid-template-columns:1.2fr 1fr;gap:var(--space-6)}
  .flow{display:grid;grid-template-columns:1fr auto auto;gap:var(--space-4);align-items:center;padding:var(--space-3) 0;border-bottom:1px solid var(--color-border-default)}
  .flow p,.flow small{margin:var(--space-1) 0}
  .flow>strong{font-family:var(--font-mono)}
  .flow>div:last-child{display:grid;gap:var(--space-1)}
  .term{display:grid;gap:var(--space-2);margin-top:var(--space-3)}
  .term div{display:flex;justify-content:space-between;font-family:var(--font-mono);font-size:var(--font-size-sm)}
  .constructor{display:grid;grid-template-columns:280px 1fr;gap:var(--space-6);align-items:center}
  .legs{display:grid;gap:var(--space-3)}
  .legs label{display:grid;gap:var(--space-1);font-size:var(--font-size-xs);color:var(--color-text-tertiary)}
  .legs select,.legs input,.legs button{padding:var(--space-2);border:1px solid var(--color-border-default);background:var(--color-surface-1);color:var(--color-text-primary)}
  .payoff{width:100%;max-height:210px}
  .payoff line{stroke:var(--color-border-strong);stroke-width:1}
  .payoff polyline{fill:none;stroke:var(--color-brand-primary);stroke-width:3}
  .dealer-head{display:grid;grid-template-columns:1fr 1fr auto;gap:var(--space-3);margin-bottom:var(--space-3)}
  .dealer-head span{display:block;color:var(--color-text-tertiary);font-size:var(--font-size-xs);text-transform:uppercase}
  .dealer-head strong{font:600 var(--font-size-lg) var(--font-mono)}
  @media(max-width:1100px){.split,.constructor,.dealer-head{grid-template-columns:1fr}.tr{min-width:980px}.flow{grid-template-columns:1fr}}
</style>
