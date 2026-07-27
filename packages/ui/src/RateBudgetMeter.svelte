<script lang="ts">
  let {
    label,
    remainingRatio = 1,
    nextPermitLabel = "ready",
  }: {
    label: string;
    remainingRatio?: number;
    nextPermitLabel?: string;
  } = $props();

  const pct = $derived(Math.round(Math.max(0, Math.min(1, remainingRatio)) * 100));
</script>

<div class="budget" title={`${label}: ${pct}% remaining · next ${nextPermitLabel}`}>
  <span class="budget__label">{label}</span>
  <span class="budget__bar" aria-hidden="true">
    <span class="budget__fill" style={`width:${pct}%`}></span>
  </span>
  <span class="budget__meta">{pct}% · {nextPermitLabel}</span>
</div>

<style>
  .budget {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-variant-numeric: tabular-nums;
    font-family: var(--font-mono);
    font-size: .625rem;
  }
  .budget__label {
    color: var(--color-text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .budget__bar {
    width: 64px;
    height: 4px;
    overflow: hidden;
    border-radius: 999px;
    background: rgba(255,255,255,.08);
  }
  .budget__fill {
    display: block;
    height: 100%;
    background: var(--color-brand-primary);
    box-shadow: 0 0 8px var(--color-brand-primary);
    transition: width var(--duration-base) var(--ease-standard);
  }
  .budget__meta { color: var(--color-text-secondary); }
</style>
