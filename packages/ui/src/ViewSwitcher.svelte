<script lang="ts">
  let {
    value = $bindable(),
    options,
    ariaLabel = "Select view",
  }: {
    value?: string;
    options: { id: string; label: string }[];
    ariaLabel?: string;
  } = $props();
</script>

<div class="switcher" role="tablist" aria-label={ariaLabel}>
  {#each options as option}
    <button
      type="button"
      role="tab"
      aria-selected={value === option.id}
      class:active={value === option.id}
      onclick={() => (value = option.id)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .switcher {
    display: inline-flex;
    gap: 3px;
    max-width: 100%;
    padding: 3px;
    overflow-x: auto;
    border: 1px solid var(--color-border-default);
    border-radius: 999px;
    background: rgba(4, 7, 13, 0.52);
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.4);
  }
  button {
    position: relative;
    flex: 0 0 auto;
    padding: 7px 12px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--color-text-tertiary);
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 0.625rem;
    letter-spacing: 0.055em;
    text-transform: uppercase;
    transition:
      color var(--duration-base) var(--ease-standard),
      background-color var(--duration-base) var(--ease-standard),
      box-shadow var(--duration-base) var(--ease-standard),
      transform var(--duration-instant) var(--ease-standard);
  }
  button:hover { color: var(--color-text-primary); }
  button.active {
    background: linear-gradient(135deg, rgba(0, 240, 255, 0.16), rgba(168, 85, 247, 0.12));
    color: #dffcff;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.1),
      0 0 18px rgba(0, 240, 255, 0.08);
  }
</style>
