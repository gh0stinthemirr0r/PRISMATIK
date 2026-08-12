<script lang="ts">
  /**
   * The one way PRISMATIK says "there is nothing here".
   *
   * A terminal that invents a plausible number is worse than one that shows a
   * gap, so every panel routes its empty, unavailable and error states through
   * this component. It always names the thing that is missing and, where there
   * is one, the action that would fix it.
   */
  import type { Snippet } from 'svelte';

  let {
    title,
    detail = null,
    /** What would make data appear — rendered as the call to action. */
    action = null,
    href = null,
    tone = 'neutral',
    compact = false,
    children,
  }: {
    title: string;
    detail?: string | null;
    action?: string | null;
    href?: string | null;
    tone?: 'neutral' | 'warn';
    compact?: boolean;
    children?: Snippet;
  } = $props();
</script>

<div class="pk-nodata" class:compact data-tone={tone}>
  <div class="pk-nodata-mark" aria-hidden="true">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.3">
      <path d="M3 17.5 L9 11 L13 14.5 L21 6.5" stroke-dasharray="3 3" />
      <circle cx="21" cy="6.5" r="1.4" fill="currentColor" stroke="none" />
    </svg>
  </div>
  <p class="pk-nodata-title">{title}</p>
  {#if detail}<p class="pk-nodata-detail">{detail}</p>{/if}
  {#if action && href}
    <a class="pk-nodata-action" {href}>{action}</a>
  {:else if action}
    <p class="pk-nodata-action-static">{action}</p>
  {/if}
  {#if children}<div class="pk-nodata-slot">{@render children()}</div>{/if}
</div>

<style>
  .pk-nodata {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 100%;
    min-height: 90px;
    padding: 18px 16px;
    text-align: center;
    color: var(--p-dim);
  }
  .pk-nodata.compact {
    min-height: 0;
    gap: 3px;
    padding: 10px 12px;
  }
  .pk-nodata-mark {
    width: 26px;
    height: 26px;
    color: var(--p-border);
  }
  .pk-nodata.compact .pk-nodata-mark {
    display: none;
  }
  .pk-nodata[data-tone='warn'] .pk-nodata-mark {
    color: var(--p-down);
    opacity: 0.55;
  }
  .pk-nodata-mark svg {
    width: 100%;
    height: 100%;
  }
  .pk-nodata-title {
    margin: 0;
    color: var(--p-text);
    font-size: var(--fz-sm);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .pk-nodata-detail,
  .pk-nodata-action-static {
    margin: 0;
    max-width: 42ch;
    font-size: var(--fz-sm);
    line-height: 1.5;
  }
  .pk-nodata-action {
    margin-top: 2px;
    padding: 4px 10px;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    color: var(--p-accent);
    font-size: var(--fz-sm);
    text-decoration: none;
    transition:
      border-color 0.15s ease,
      background 0.15s ease;
  }
  .pk-nodata-action:hover {
    border-color: var(--p-accent);
    background: color-mix(in srgb, var(--p-accent) 10%, transparent);
  }
  .pk-nodata-slot {
    margin-top: 4px;
  }
</style>
