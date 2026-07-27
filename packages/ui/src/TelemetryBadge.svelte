<script lang="ts">
  let {
    label,
    tone = "cyan",
    pulse = false,
  }: {
    label: string;
    tone?: "cyan" | "violet" | "emerald" | "amber" | "neutral";
    pulse?: boolean;
  } = $props();
</script>

<span class="badge" data-tone={tone}>
  <span class:pulse class="badge__dot" aria-hidden="true"></span>
  {label}
</span>

<style>
  .badge {
    --badge-color: var(--color-brand-primary);
    display: inline-flex;
    align-items: center;
    gap: 7px;
    width: fit-content;
    padding: 5px 9px;
    border: 1px solid color-mix(in oklab, var(--badge-color) 24%, transparent);
    border-radius: 999px;
    background: color-mix(in oklab, var(--badge-color) 7%, transparent);
    color: color-mix(in oklab, var(--badge-color) 78%, white);
    font-family: var(--font-mono);
    font-size: 0.625rem;
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.08em;
    line-height: 1;
    text-transform: uppercase;
  }
  .badge[data-tone="violet"] { --badge-color: var(--color-brand-accent); }
  .badge[data-tone="emerald"] { --badge-color: var(--color-success); }
  .badge[data-tone="amber"] { --badge-color: var(--color-warning); }
  .badge[data-tone="neutral"] { --badge-color: var(--color-text-tertiary); }
  .badge__dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--badge-color);
    box-shadow: 0 0 8px var(--badge-color);
  }
  .badge__dot.pulse {
    animation: telemetry-pulse 1.8s ease-out infinite;
  }
  @keyframes telemetry-pulse {
    0%, 100% { opacity: 1; box-shadow: 0 0 0 0 color-mix(in oklab, var(--badge-color) 45%, transparent); }
    55% { opacity: 0.72; box-shadow: 0 0 0 5px transparent; }
  }
</style>
