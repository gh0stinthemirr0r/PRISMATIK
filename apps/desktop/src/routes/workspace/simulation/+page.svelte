<script lang="ts">
  import UnavailableExperience from '$lib/UnavailableExperience.svelte';
</script>

<svelte:head><title>Simulation laboratory · PRISMATIK</title></svelte:head>

<div class="pk-page">
  <div class="pk-page-head">
    <h1>Simulation laboratory</h1>
    <p>
      Deterministic market simulation and walk-forward replay. The full simulation engine lives in
      the Rust workspace (<code>prismatik-simulation</code>) — the interactive replay UI requires
      a bound strategy and a historical observation corpus to replay against.
    </p>
  </div>

  <div class="pk-cross-links">
    <a href="/workspace/backtest" class="pk-cross-card">
      <div class="pk-cross-eyebrow">Run now</div>
      <div class="pk-cross-title">Backtest laboratory</div>
      <div class="pk-cross-desc">Run deterministic backtests of seed strategies with full Sharpe/Sortino/Calmar metrics over synthetic or real bar series.</div>
    </a>
    <a href="/workspace/strategy" class="pk-cross-card">
      <div class="pk-cross-eyebrow">Author</div>
      <div class="pk-cross-title">Strategy workbench</div>
      <div class="pk-cross-desc">Author and validate StrategyIR drafts. Validated strategies can be bound to a simulation run.</div>
    </a>
    <a href="/workspace/integrations" class="pk-cross-card">
      <div class="pk-cross-eyebrow">Data</div>
      <div class="pk-cross-title">Integrations</div>
      <div class="pk-cross-desc">Connect a market-data provider to supply real governed history for simulation replay.</div>
    </a>
  </div>

  <UnavailableExperience
    active="simulation"
    title="Simulation laboratory"
    description="No generated paths are displayed until a native simulation run has been executed with a bound strategy and observation corpus."
  />
</div>

<style>
  .pk-page { padding: 24px; max-width: 1000px; }
  .pk-page-head h1 { font-size: 1.4rem; font-weight: 600; margin: 0 0 6px; color: var(--p-text); }
  .pk-page-head p { font-size: 0.8125rem; color: var(--p-text-dim); margin: 0 0 20px; max-width: 65ch; line-height: 1.5; }
  .pk-page-head code { font-family: var(--p-mono); background: var(--p-surface2); padding: 1px 5px; border-radius: 3px; font-size: 0.75rem; color: var(--p-accent); }
  .pk-cross-links { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 12px; margin-bottom: 24px; }
  .pk-cross-card { background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 8px; padding: 16px; text-decoration: none; color: inherit; transition: border-color 0.15s, transform 0.15s; }
  .pk-cross-card:hover { border-color: var(--p-accent); transform: translateY(-2px); }
  .pk-cross-eyebrow { font-family: var(--p-mono); font-size: 0.5625rem; letter-spacing: 0.12em; text-transform: uppercase; color: var(--p-accent); margin-bottom: 6px; }
  .pk-cross-title { font-size: 0.9375rem; font-weight: 600; color: var(--p-text); margin-bottom: 6px; }
  .pk-cross-desc { font-size: 0.75rem; line-height: 1.5; color: var(--p-text-dim); }
</style>
