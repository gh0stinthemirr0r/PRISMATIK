<script lang="ts">
  // The panel that keeps the product honest: search accounting, stability,
  // bootstrap spread. This is a feature, not a disclaimer.
  import type { WalkforwardResult } from '../types';
  import { fmtPct, fmtNum } from '../api';
  export let result: WalkforwardResult;
</script>

<section class="honesty">
  <h3>Reading this result</h3>
  <p class="mono row">
    Folds: {result.n_folds} · Configurations searched: {result.configs_searched} ·
    Parameters {result.stable ? 'stable' : 'unstable'} across folds
  </p>
  {#if Object.keys(result.param_stability).length}
    <p class="mono row dim">
      Stability (coefficient of variation, lower is steadier):
      {#each Object.entries(result.param_stability) as [k, v], i}{i > 0 ? ' · ' : ''}{k}: {fmtNum(v, 2)}{/each}
    </p>
  {/if}
  {#if result.bootstrap}
    <p class="mono row">
      Bootstrap ({result.bootstrap.n_sims} resamples, {result.bootstrap.block_bars}-bar blocks):
      5th {fmtPct(result.bootstrap.total_return_p05)} ·
      median {fmtPct(result.bootstrap.total_return_p50)} ·
      95th {fmtPct(result.bootstrap.total_return_p95)} ·
      P(loss) {fmtPct(result.bootstrap.prob_loss, 0)}
    </p>
  {/if}
  <p class="caution">{result.caution}</p>
</section>

<style>
  .honesty {
    border: 1px solid var(--gilt-faint);
    border-radius: var(--radius);
    padding: 16px 18px;
    background: var(--ink-2);
  }
  h3 {
    font-family: var(--display);
    font-weight: 400;
    font-size: 16px;
    margin: 0 0 10px;
    color: var(--gilt);
  }
  .row { margin: 4px 0; font-size: 12.5px; }
  .dim { color: var(--porcelain-2); }
  .caution { color: var(--porcelain-2); font-size: 13px; margin: 10px 0 0; }
</style>
