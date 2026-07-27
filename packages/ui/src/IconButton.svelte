<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    ariaLabel,
    disabled = false,
    children,
    onclick,
  }: {
    ariaLabel: string;
    disabled?: boolean;
    children?: Snippet;
    onclick?: (event: MouseEvent) => void;
  } = $props();
</script>

<button class="icon-button" aria-label={ariaLabel} {disabled} onclick={onclick}>
  {@render children?.()}
</button>

<style>
  .icon-button {
    align-items: center;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--color-text-secondary);
    cursor: pointer;
    display: inline-flex;
    height: 32px;
    justify-content: center;
    padding: var(--space-1);
    transition: background-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard);
    width: 32px;
  }

  .icon-button:hover:not(:disabled) { background: var(--color-surface-2); color: var(--color-text-primary); }
  .icon-button:focus-visible { outline: 2px solid var(--color-border-focus); outline-offset: 2px; }
  .icon-button:disabled { cursor: not-allowed; opacity: 0.55; }
</style>
