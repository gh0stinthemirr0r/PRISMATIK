<script lang="ts">
  // SVG equity chart. Gilt line is the strategy; ghost line is the benchmark.
  import type { EquityPoint } from '../types';
  export let series: EquityPoint[] = [];
  export let ghost: EquityPoint[] = [];
  export let label = 'Equity';

  const W = 860;
  const H = 280;
  const PAD = 34;

  function path(points: EquityPoint[], lo: number, hi: number): string {
    if (points.length < 2) return '';
    const span = hi - lo || 1;
    return points
      .map((p, i) => {
        const x = PAD + (i / (points.length - 1)) * (W - PAD * 2);
        const y = H - PAD - ((p.value - lo) / span) * (H - PAD * 2);
        return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
  }

  $: all = [...series, ...ghost].map((p) => p.value);
  $: lo = all.length ? Math.min(...all) : 0;
  $: hi = all.length ? Math.max(...all) : 1;
  $: main = path(series, lo, hi);
  $: bench = path(ghost, lo, hi);
</script>

<figure class="chart" aria-label={label}>
  <svg viewBox="0 0 {W} {H}" role="img">
    <line x1={PAD} y1={H - PAD} x2={W - PAD} y2={H - PAD} class="axis" />
    <line x1={PAD} y1={PAD} x2={PAD} y2={H - PAD} class="axis" />
    {#if bench}
      <path d={bench} class="ghost" />
    {/if}
    {#if main}
      {#key main}
        <path d={main} class="line" pathLength="1" />
      {/key}
    {/if}
    <text x={PAD} y={PAD - 10} class="tick">{hi.toFixed(2)}</text>
    <text x={PAD} y={H - PAD + 22} class="tick">{lo.toFixed(2)}</text>
  </svg>
  <figcaption class="mono caption">{label}</figcaption>
</figure>

<style>
  .chart { margin: 0; }
  svg { width: 100%; height: auto; display: block; }
  .axis { stroke: var(--gilt-faint); stroke-width: 1; }
  .ghost {
    fill: none;
    stroke: var(--porcelain-2);
    stroke-opacity: 0.35;
    stroke-width: 1.4;
    stroke-dasharray: 5 4;
  }
  .line {
    fill: none;
    stroke: var(--gilt);
    stroke-width: 1.8;
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    animation: draw 0.9s ease forwards;
  }
  @keyframes draw { to { stroke-dashoffset: 0; } }
  .tick { font-family: var(--mono); font-size: 11px; fill: var(--porcelain-2); }
  .caption { color: var(--porcelain-2); font-size: 12px; margin-top: 6px; }
</style>
