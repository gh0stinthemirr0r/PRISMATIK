<script lang="ts">
  import UnavailableExperience from '$lib/UnavailableExperience.svelte';
</script>

<svelte:head><title>Options intelligence · PRISMATIK</title></svelte:head>

<div class="pk-page">
  <div class="pk-page-head">
    <h1>Options intelligence</h1>
    <p>
      Option chains, flow analytics, implied-volatility surfaces, and dealer gamma exposure.
      The Rust contract layer (<code>prismatik-options</code>) defines Greeks, chains, flow classification,
      and OCC symbology. An IV-surface renderer exists locally. All of this awaits a licensed options data feed.
    </p>
  </div>

  <div class="pk-capability-grid">
    <div class="pk-cap-card">
      <div class="pk-cap-label">Contract layer</div>
      <div class="pk-cap-status ready">Ready</div>
      <div class="pk-cap-desc">Greeks (delta/gamma/vega/theta), OptionsChain, FlowPrint, OccSymbol, Moneyness — all defined in Rust, unit-tested.</div>
    </div>
    <div class="pk-cap-card">
      <div class="pk-cap-label">IV-surface renderer</div>
      <div class="pk-cap-status ready">Ready</div>
      <div class="pk-cap-desc">Canvas/WebGPU 3D volatility-surface renderer in lib/iv-surface — draws the surface from quote arrays. Needs real IV data.</div>
    </div>
    <div class="pk-cap-card">
      <div class="pk-cap-label">Options data feed</div>
      <div class="pk-cap-status blocked">Blocked</div>
      <div class="pk-cap-desc">No licensed options provider connected. OPRA/exchange feeds, Unusual Whales flow, or Databento IV surfaces require commercial entitlements.</div>
    </div>
    <div class="pk-cap-card">
      <div class="pk-cap-label">GEX / dealer positioning</div>
      <div class="pk-cap-status blocked">Blocked</div>
      <div class="pk-cap-desc">Gamma exposure and dealer positioning require open-interest publication timing, contract adjustments, and point-in-time-safe symbology.</div>
    </div>
  </div>

  <UnavailableExperience
    active="options"
    title="Options intelligence"
    description="Option chains, flow, volatility surfaces, and dealer exposure require an entitled live options source. Connect Unusual Whales, Databento, or an OPRA-licensed feed in Integrations."
  />
</div>

<style>
  .pk-page { padding: 24px; max-width: 1000px; }
  .pk-page-head h1 { font-size: 1.4rem; font-weight: 600; margin: 0 0 6px; color: var(--p-text); }
  .pk-page-head p { font-size: 0.8125rem; color: var(--p-text-dim); margin: 0 0 20px; max-width: 65ch; line-height: 1.5; }
  .pk-page-head code { font-family: var(--p-mono); background: var(--p-surface2); padding: 1px 5px; border-radius: 3px; font-size: 0.75rem; color: var(--p-accent); }
  .pk-capability-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 12px; margin-bottom: 24px; }
  .pk-cap-card { background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 8px; padding: 16px; }
  .pk-cap-label { font-size: 0.8125rem; font-weight: 600; color: var(--p-text); margin-bottom: 4px; }
  .pk-cap-status { display: inline-block; font-family: var(--p-mono); font-size: 0.5625rem; letter-spacing: 0.1em; text-transform: uppercase; padding: 2px 8px; border-radius: 3px; margin-bottom: 8px; }
  .pk-cap-status.ready { background: rgba(52,211,153,0.12); color: #34d399; }
  .pk-cap-status.blocked { background: rgba(251,191,36,0.1); color: #fbbf24; }
  .pk-cap-desc { font-size: 0.75rem; line-height: 1.5; color: var(--p-text-dim); }
</style>
