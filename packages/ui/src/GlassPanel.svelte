<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    eyebrow,
    title,
    tone = "cyan",
    interactive = true,
    class: className = "",
    children,
    actions,
  }: {
    eyebrow?: string;
    title?: string;
    tone?: "cyan" | "violet" | "emerald" | "amber";
    interactive?: boolean;
    class?: string;
    children?: Snippet;
    actions?: Snippet;
  } = $props();

  let panel: HTMLElement | undefined = $state();

  function track(event: PointerEvent) {
    if (!panel) return;
    const bounds = panel.getBoundingClientRect();
    const x = event.clientX - bounds.left;
    const y = event.clientY - bounds.top;
    panel.style.setProperty("--mouse-x", `${x}px`);
    panel.style.setProperty("--mouse-y", `${y}px`);
    if (!interactive) return;
    const ry = ((x / bounds.width) - 0.5) * 1.1;
    const rx = ((y / bounds.height) - 0.5) * -0.8;
    panel.style.setProperty("--tilt-x", `${rx}deg`);
    panel.style.setProperty("--tilt-y", `${ry}deg`);
  }

  function reset() {
    panel?.style.setProperty("--tilt-x", "0deg");
    panel?.style.setProperty("--tilt-y", "0deg");
  }
</script>

<section
  bind:this={panel}
  class={`glass-panel ${className}`}
  class:interactive
  data-tone={tone}
  role="group"
  aria-label={title ?? eyebrow ?? "Data panel"}
  onpointermove={track}
  onpointerleave={reset}
>
  {#if eyebrow || title || actions}
    <header class="glass-panel__head">
      <div>
        {#if eyebrow}<div class="glass-panel__eyebrow">{eyebrow}</div>{/if}
        {#if title}<h2>{title}</h2>{/if}
      </div>
      {#if actions}<div class="glass-panel__actions">{@render actions()}</div>{/if}
    </header>
  {/if}
  <div class="glass-panel__body">{@render children?.()}</div>
</section>

<style>
  .glass-panel {
    --panel-glow: rgba(0, 240, 255, 0.09);
    position: relative;
    min-width: 0;
    overflow: hidden;
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-lg);
    background:
      radial-gradient(620px at var(--mouse-x, 50%) var(--mouse-y, 0%), rgba(255, 255, 255, 0.055), transparent 42%),
      linear-gradient(145deg, rgba(19, 25, 40, 0.76), rgba(9, 13, 22, 0.63));
    box-shadow: var(--shadow-panel);
    backdrop-filter: blur(var(--blur-panel)) saturate(170%);
    transform: perspective(1000px) rotateX(var(--tilt-x, 0deg)) rotateY(var(--tilt-y, 0deg));
    transform-style: preserve-3d;
    transition:
      transform var(--duration-base) var(--ease-standard),
      border-color var(--duration-base) var(--ease-standard),
      box-shadow var(--duration-base) var(--ease-standard);
    will-change: transform;
  }

  .glass-panel::before {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background:
      linear-gradient(90deg, transparent, var(--panel-glow), transparent) top / 74% 1px no-repeat,
      radial-gradient(circle at 0 0, var(--panel-glow), transparent 32%);
    content: "";
    pointer-events: none;
  }

  .glass-panel[data-tone="violet"] { --panel-glow: rgba(168, 85, 247, 0.16); }
  .glass-panel[data-tone="emerald"] { --panel-glow: rgba(16, 185, 129, 0.14); }
  .glass-panel[data-tone="amber"] { --panel-glow: rgba(245, 158, 11, 0.14); }
  .glass-panel.interactive:hover {
    border-color: rgba(0, 240, 255, 0.16);
    box-shadow: var(--shadow-panel), 0 0 42px -28px var(--color-brand-primary);
  }

  .glass-panel__head {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-5) var(--space-5) 0;
  }
  .glass-panel__eyebrow {
    margin-bottom: var(--space-2);
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: 0.625rem;
    font-weight: var(--font-weight-semibold);
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }
  h2 {
    margin: 0;
    color: var(--color-text-primary);
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-semibold);
    letter-spacing: -0.025em;
  }
  .glass-panel__actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .glass-panel__body {
    position: relative;
    z-index: 1;
    padding: var(--space-5);
  }
</style>
