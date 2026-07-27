<script lang="ts">
  type EvidenceStatus = "confirmed" | "uncertain" | "contradicted";

  let {
    status,
    label,
  }: {
    status: EvidenceStatus;
    label?: string;
  } = $props();

  const text = $derived(label ?? status);
</script>

<span class={`evidence evidence--${status}`}>
  <span class="evidence__mark" aria-hidden="true"></span>
  {text}
</span>

<style>
  .evidence {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    width: fit-content;
    padding: 5px 8px;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    border-radius: 999px;
    background: color-mix(in oklab, currentColor 5%, transparent);
    color: var(--color-text-secondary);
    font-family: var(--font-mono);
    font-size: .625rem;
    font-weight: var(--font-weight-medium);
    letter-spacing: .05em;
    line-height: 1;
    text-transform: uppercase;
  }
  .evidence__mark {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: currentColor;
    box-shadow: 0 0 7px currentColor;
  }
  .evidence--confirmed { color: var(--color-evidence-confirmed); }
  .evidence--uncertain { color: var(--color-evidence-uncertain); }
  .evidence--contradicted { color: var(--color-evidence-contradicted); }
</style>
