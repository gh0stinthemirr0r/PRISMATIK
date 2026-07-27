<script lang="ts">
  let {
    label,
    value,
    delta,
    unit = "",
    fresh = true,
    series = [],
    tone = "cyan",
  }: {
    label: string;
    value: string;
    delta?: string;
    unit?: string;
    fresh?: boolean;
    series?: number[];
    tone?: "cyan" | "violet" | "emerald" | "amber";
  } = $props();

  const deltaTone = $derived(
    !delta ? "flat" : delta.trim().startsWith("-") ? "down" : delta.trim().startsWith("+") ? "up" : "flat",
  );
</script>

<div class="metric" class:stale={!fresh} data-tone={tone}>
  <div class="metric__label">{label}</div>
  <div class="metric__value">
    <span class="metric__num">{value}</span>
    {#if unit}<span class="metric__unit">{unit}</span>{/if}
  </div>
  {#if delta}
    <div class="metric__delta" data-tone={deltaTone}>{delta}</div>
  {/if}
  {#if series.length > 1}
    <div class="metric__spark" aria-hidden="true">
      {#each series as point, index}
        <i style={`height:${10 + ((point - Math.min(...series)) / Math.max(0.001, Math.max(...series) - Math.min(...series))) * 18}px;opacity:${0.28 + index / series.length * 0.72}`}></i>
      {/each}
    </div>
  {/if}
</div>

<style>
  .metric {
    --metric-color: var(--color-brand-primary);
    position: relative;
    display: grid;
    gap: var(--space-1);
    min-width: 0;
    padding: var(--space-4);
    overflow: hidden;
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-md);
    background:
      radial-gradient(circle at 100% 0, color-mix(in oklab, var(--metric-color) 8%, transparent), transparent 58%),
      rgba(255,255,255,.018);
    box-shadow: inset 0 1px 0 rgba(255,255,255,.055);
    transition:
      transform var(--duration-base) var(--ease-standard),
      border-color var(--duration-base) var(--ease-standard);
  }
  .metric:hover {
    border-color: color-mix(in oklab, var(--metric-color) 24%, transparent);
    transform: translateY(-2px);
  }
  .metric[data-tone="violet"] { --metric-color: var(--color-brand-accent); }
  .metric[data-tone="emerald"] { --metric-color: var(--color-success); }
  .metric[data-tone="amber"] { --metric-color: var(--color-warning); }
  .metric__label {
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.11em;
    text-transform: uppercase;
  }
  .metric__value {
    display: flex;
    align-items: baseline;
    gap: var(--space-1);
    font-variant-numeric: tabular-nums;
  }
  .metric__num {
    font-family: var(--font-mono);
    font-size: 1.55rem;
    font-weight: var(--font-weight-semibold);
    letter-spacing: -0.02em;
    line-height: var(--line-height-tight);
  }
  .metric__unit {
    color: var(--color-text-tertiary);
    font-size: var(--font-size-sm);
  }
  .metric__delta {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    font-variant-numeric: tabular-nums;
  }
  .metric__delta[data-tone="up"] { color: var(--color-up); }
  .metric__delta[data-tone="down"] { color: var(--color-down); }
  .metric__delta[data-tone="flat"] { color: var(--color-neutral); }
  .metric.stale .metric__num { color: var(--color-text-secondary); }
  .metric__spark {
    position: absolute;
    right: 12px;
    bottom: 10px;
    display: flex;
    height: 30px;
    align-items: flex-end;
    gap: 2px;
    opacity: .72;
  }
  .metric__spark i {
    width: 2px;
    border-radius: 2px;
    background: var(--metric-color);
    box-shadow: 0 0 5px color-mix(in oklab, var(--metric-color) 45%, transparent);
  }
</style>
