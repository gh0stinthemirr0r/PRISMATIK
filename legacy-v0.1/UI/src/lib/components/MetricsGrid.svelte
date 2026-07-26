<script lang="ts">
  import type { Metrics } from '../types';
  import { fmtPct, fmtNum, signClass } from '../api';
  export let metrics: Metrics;
  export let benchmark: Metrics | null = null;

  $: rows = [
    { k: 'Total return', v: fmtPct(metrics.total_return), b: benchmark ? fmtPct(benchmark.total_return) : null, cls: signClass(metrics.total_return) },
    { k: 'CAGR', v: fmtPct(metrics.cagr), b: benchmark ? fmtPct(benchmark.cagr) : null, cls: signClass(metrics.cagr) },
    { k: 'Sharpe', v: fmtNum(metrics.sharpe, 2), b: benchmark ? fmtNum(benchmark.sharpe, 2) : null, cls: '' },
    { k: 'Sortino', v: fmtNum(metrics.sortino, 2), b: benchmark ? fmtNum(benchmark.sortino, 2) : null, cls: '' },
    { k: 'Volatility (ann.)', v: fmtPct(metrics.ann_volatility), b: benchmark ? fmtPct(benchmark.ann_volatility) : null, cls: '' },
    { k: 'Max drawdown', v: fmtPct(metrics.max_drawdown), b: benchmark ? fmtPct(benchmark.max_drawdown) : null, cls: 'neg' },
    { k: 'Calmar', v: fmtNum(metrics.calmar, 2), b: benchmark ? fmtNum(benchmark.calmar, 2) : null, cls: '' }
  ];
</script>

<table class="grid">
  <thead>
    <tr>
      <th>Measure</th>
      <th>Strategy</th>
      {#if benchmark}<th>Buy &amp; hold</th>{/if}
    </tr>
  </thead>
  <tbody>
    {#each rows as r}
      <tr>
        <td class="k">{r.k}</td>
        <td class="mono {r.cls}">{r.v}</td>
        {#if r.b !== null}<td class="mono dim">{r.b}</td>{/if}
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .grid { width: 100%; border-collapse: collapse; }
  th {
    font-family: var(--mono);
    font-size: 11px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--gilt);
    text-align: left;
    padding: 8px 10px;
    border-bottom: 1px solid var(--gilt-dim);
  }
  td { padding: 9px 10px; border-bottom: 1px solid rgba(201, 169, 106, 0.07); }
  .k { color: var(--porcelain-2); }
  .dim { color: var(--porcelain-2); }
</style>
