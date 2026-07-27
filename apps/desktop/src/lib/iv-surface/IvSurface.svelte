<script lang="ts">
  import { renderIvSurface, type IvSurfaceBackend } from "./render";
  import type { IvQuote } from "./mesh";

  let {
    quotes = [],
    preferWebGpu = true,
  }: {
    quotes?: IvQuote[];
    preferWebGpu?: boolean;
  } = $props();

  let surfaceEl = $state<HTMLCanvasElement | null>(null);
  let overlayEl = $state<HTMLCanvasElement | null>(null);
  let backend = $state<IvSurfaceBackend>("unavailable");

  $effect(() => {
    const q = quotes;
    const canvas = surfaceEl;
    const overlay = overlayEl;
    if (!canvas) return;
    let cancelled = false;
    void renderIvSurface(canvas, q, preferWebGpu, overlay).then((result) => {
      if (!cancelled) backend = result.backend;
    });
    return () => {
      cancelled = true;
    };
  });
</script>

<div class="iv">
  <canvas bind:this={surfaceEl} class="iv__surface" aria-label="Implied volatility surface"></canvas>
  <canvas bind:this={overlayEl} class="iv__overlay" aria-hidden="true"></canvas>
</div>
<p class="iv__meta">
  Backend: {backend}
  · WebGPU preferred when available
  · native wgpu (`prismatik-renderer`) deferred
</p>

<style>
  .iv {
    position: relative;
    width: 100%;
    height: 280px;
    border: 1px solid var(--color-border-default);
    background: var(--color-surface-1);
  }
  .iv__surface,
  .iv__overlay {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .iv__overlay {
    pointer-events: none;
  }
  .iv__meta {
    margin: var(--space-2) 0 0;
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
  }
</style>
