<script lang="ts">
  import type { Snippet } from "svelte";

  type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
  type ButtonSize = "sm" | "md" | "lg";

  let {
    variant = "primary",
    size = "md",
    loading = false,
    disabled = false,
    type = "button",
    children,
    onclick,
  }: {
    variant?: ButtonVariant;
    size?: ButtonSize;
    loading?: boolean;
    disabled?: boolean;
    type?: "button" | "submit" | "reset";
    children?: Snippet;
    onclick?: (event: MouseEvent) => void;
  } = $props();
</script>

<button
  {type}
  class:loading
  class={`button button--${variant} button--${size}`}
  disabled={disabled || loading}
  aria-busy={loading}
  onclick={onclick}
>
  {#if loading}<span class="button__spinner" aria-hidden="true"></span>{/if}
  {@render children?.()}
</button>

<style>
  .button {
    align-items: center;
    border: 1px solid transparent;
    border-radius: 999px;
    cursor: pointer;
    display: inline-flex;
    font-family: var(--font-sans);
    font-weight: var(--font-weight-semibold);
    gap: var(--space-2);
    justify-content: center;
    letter-spacing: 0.045em;
    line-height: var(--line-height-tight);
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      border-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard),
      transform var(--duration-fast) var(--ease-standard),
      box-shadow var(--duration-fast) var(--ease-standard);
  }
  .button:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: 2px;
  }
  .button:disabled { cursor: not-allowed; opacity: 0.5; }
  .button:not(:disabled):active { transform: scale(0.985); }
  .button--sm { font-size: var(--font-size-sm); min-height: 28px; padding: 0 var(--space-3); }
  .button--md { font-size: var(--font-size-base); min-height: 36px; padding: 0 var(--space-4); }
  .button--lg { font-size: var(--font-size-md); min-height: 44px; padding: 0 var(--space-5); }
  .button--primary {
    background: linear-gradient(115deg, #00dbe8, #00f0ff 54%, #9afaff);
    box-shadow: inset 0 1px 0 rgba(255,255,255,.45), 0 8px 24px -12px rgba(0,240,255,.8);
    color: var(--color-text-inverse);
  }
  .button--primary:hover:not(:disabled) { box-shadow: inset 0 1px 0 rgba(255,255,255,.55), 0 10px 32px -10px rgba(0,240,255,.9); }
  .button--secondary {
    background: rgba(255,255,255,.045);
    border-color: var(--color-border-strong);
    color: var(--color-text-primary);
  }
  .button--secondary:hover:not(:disabled) { background: rgba(255,255,255,.08); }
  .button--ghost { background: transparent; color: var(--color-text-primary); }
  .button--ghost:hover:not(:disabled) { background: rgba(0,240,255,.065); color: var(--color-brand-primary); }
  .button--danger { background: var(--color-danger); color: var(--color-text-inverse); }
  .button__spinner {
    animation: spin var(--duration-base) linear infinite;
    border: 2px solid currentColor;
    border-right-color: transparent;
    border-radius: 50%;
    height: 0.8em;
    width: 0.8em;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
