<script lang="ts">
  let {
    values,
    labels = [],
    color = "cyan",
    height = 96,
    showAxis = false,
    valuePrefix = "",
    valueSuffix = "",
  }: {
    values: number[];
    labels?: string[];
    color?: "cyan" | "violet" | "emerald" | "amber";
    height?: number;
    showAxis?: boolean;
    valuePrefix?: string;
    valueSuffix?: string;
  } = $props();

  const width = 720;
  const inset = 12;
  let activeIndex = $state<number | null>(null);
  let host = $state<HTMLDivElement | null>(null);

  const minimum = $derived(Math.min(...values));
  const maximum = $derived(Math.max(...values));
  const spread = $derived(Math.max(0.0001, maximum - minimum));
  const points = $derived(
    values.map((value, index) => ({
      x: inset + (index / Math.max(1, values.length - 1)) * (width - inset * 2),
      y: inset + (1 - (value - minimum) / spread) * (height - inset * 2),
    })),
  );
  const line = $derived(points.map((point) => `${point.x},${point.y}`).join(" "));
  const area = $derived(
    points.length
      ? `M ${points[0].x} ${height - inset} L ${points.map((point) => `${point.x} ${point.y}`).join(" L ")} L ${points.at(-1)?.x} ${height - inset} Z`
      : "",
  );
  const active = $derived(activeIndex == null ? null : points[activeIndex]);

  function scrub(event: PointerEvent) {
    if (!host || values.length === 0) return;
    const bounds = host.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width));
    activeIndex = Math.round(ratio * (values.length - 1));
  }
</script>

<div
  class="spark"
  data-color={color}
  style={`--chart-height:${height}px`}
  role="group"
  aria-label="Interactive telemetry trend"
  bind:this={host}
  onpointermove={scrub}
  onpointerleave={() => (activeIndex = null)}
>
  <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" role="img" aria-label="Telemetry trend">
    <defs>
      <linearGradient id={`spark-fill-${color}`} x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="var(--spark-color)" stop-opacity="0.28" />
        <stop offset="100%" stop-color="var(--spark-color)" stop-opacity="0" />
      </linearGradient>
    </defs>
    {#if showAxis}
      <g class="grid">
        <line x1="0" y1={height * 0.25} x2={width} y2={height * 0.25} />
        <line x1="0" y1={height * 0.5} x2={width} y2={height * 0.5} />
        <line x1="0" y1={height * 0.75} x2={width} y2={height * 0.75} />
      </g>
    {/if}
    <path class="area" d={area} fill={`url(#spark-fill-${color})`} />
    <polyline class="line-shadow" points={line} />
    <polyline class="line" points={line} />
    {#if active && activeIndex != null}
      <line class="scrubber" x1={active.x} y1="0" x2={active.x} y2={height} />
      <circle class="active-point active-point--halo" cx={active.x} cy={active.y} r="7" />
      <circle class="active-point" cx={active.x} cy={active.y} r="3.5" />
    {:else if points.length}
      <circle class="live-point live-point--halo" cx={points.at(-1)?.x} cy={points.at(-1)?.y} r="7" />
      <circle class="live-point" cx={points.at(-1)?.x} cy={points.at(-1)?.y} r="3" />
    {/if}
  </svg>

  {#if activeIndex != null}
    <div
      class="tooltip"
      style={`left:${(activeIndex / Math.max(1, values.length - 1)) * 100}%`}
    >
      <span>{labels[activeIndex] ?? `T-${values.length - 1 - activeIndex}`}</span>
      <strong>{valuePrefix}{values[activeIndex].toLocaleString(undefined, { maximumFractionDigits: 2 })}{valueSuffix}</strong>
      {#if activeIndex > 0}
        <em class:negative={values[activeIndex] < values[activeIndex - 1]}>
          {values[activeIndex] >= values[activeIndex - 1] ? "+" : ""}
          {(values[activeIndex] - values[activeIndex - 1]).toFixed(2)}
        </em>
      {/if}
    </div>
  {/if}
</div>

<style>
  .spark {
    --spark-color: var(--color-brand-primary);
    position: relative;
    width: 100%;
    height: var(--chart-height);
    touch-action: pan-y;
  }
  .spark[data-color="violet"] { --spark-color: var(--color-brand-accent); }
  .spark[data-color="emerald"] { --spark-color: var(--color-success); }
  .spark[data-color="amber"] { --spark-color: var(--color-warning); }
  svg { display: block; width: 100%; height: 100%; overflow: visible; }
  .grid line {
    stroke: rgba(255, 255, 255, 0.055);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }
  .line,
  .line-shadow {
    fill: none;
    stroke: var(--spark-color);
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 1.8;
    vector-effect: non-scaling-stroke;
  }
  .line-shadow {
    opacity: 0.55;
    stroke-width: 7;
    filter: blur(7px);
  }
  .scrubber {
    stroke: rgba(255, 255, 255, 0.18);
    stroke-dasharray: 3 4;
    vector-effect: non-scaling-stroke;
  }
  .active-point,
  .live-point { fill: var(--spark-color); }
  .active-point--halo,
  .live-point--halo {
    opacity: 0.24;
    animation: point-pulse 1.7s ease-out infinite;
    transform-box: fill-box;
    transform-origin: center;
  }
  .tooltip {
    position: absolute;
    z-index: 3;
    top: 4px;
    display: grid;
    min-width: 116px;
    gap: 3px;
    padding: 9px 11px;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    background: rgba(5, 8, 14, 0.88);
    box-shadow: var(--shadow-panel);
    backdrop-filter: blur(18px);
    pointer-events: none;
    transform: translateX(-50%);
  }
  .tooltip span {
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: 0.5625rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .tooltip strong {
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }
  .tooltip em {
    color: var(--color-success);
    font-family: var(--font-mono);
    font-size: 0.625rem;
    font-style: normal;
  }
  .tooltip em.negative { color: var(--color-down); }
  @keyframes point-pulse {
    0% { opacity: 0.38; transform: scale(0.6); }
    75%, 100% { opacity: 0; transform: scale(1.8); }
  }
</style>
