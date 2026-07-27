<script lang="ts">
  type CoverageStatus = "good" | "marginal" | "bad";

  let {
    status,
    label,
  }: {
    status: CoverageStatus;
    label?: string;
  } = $props();

  const text = $derived(label ?? `Coverage ${status}`);
</script>

<span class={`coverage coverage--${status}`} aria-label={`Calibration coverage: ${status}`}>
  <span class="coverage__track" aria-hidden="true">
    <span class="coverage__fill"></span>
  </span>
  {text}
</span>

<style>
  .coverage {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    width: fit-content;
    padding: 5px 8px;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    border-radius: 999px;
    background: color-mix(in oklab, currentColor 5%, transparent);
    color: var(--color-text-secondary);
    font-family: var(--font-mono);
    font-size: .625rem;
    font-weight: var(--font-weight-medium);
  }
  .coverage__track {
    width: 28px;
    height: 4px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--color-surface-3);
  }
  .coverage__fill {
    display: block;
    height: 100%;
    width: 100%;
    background: currentColor;
    box-shadow: 0 0 7px currentColor;
  }
  .coverage--good { color: var(--color-coverage-good); }
  .coverage--good .coverage__fill { width: 92%; }
  .coverage--marginal { color: var(--color-coverage-marginal); }
  .coverage--marginal .coverage__fill { width: 64%; }
  .coverage--bad { color: var(--color-coverage-bad); }
  .coverage--bad .coverage__fill { width: 38%; }
</style>
