<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title = "PRISMATIK",
    onOpenPalette,
    children,
    sidebar,
    status,
  }: {
    title?: string;
    onOpenPalette?: () => void;
    children?: Snippet;
    sidebar?: Snippet;
    status?: Snippet;
  } = $props();

  let shell: HTMLDivElement | undefined = $state();

  function trackCursor(event: PointerEvent) {
    if (!shell) return;
    shell.style.setProperty("--cursor-x", `${event.clientX}px`);
    shell.style.setProperty("--cursor-y", `${event.clientY}px`);
  }
</script>

<svelte:window onpointermove={trackCursor} />

<div class="shell" bind:this={shell}>
  <div class="ambient" aria-hidden="true">
    <span></span><span></span><span></span><span></span><span></span>
  </div>
  <header class="top">
    <div class="brand">
      <span class="brand__sigil" aria-hidden="true"><i></i></span>
      <span>
        <span class="wordmark">{title}</span>
        <span class="brand__sub">Intelligence fabric</span>
      </span>
    </div>
    <button class="finder" type="button" onclick={() => onOpenPalette?.()}>
      <span class="finder__icon" aria-hidden="true"></span>
      <span class="finder__placeholder">Search intelligence, assets, commands</span>
      <span class="finder__keys"><kbd>⌘</kbd><kbd>K</kbd></span>
    </button>
    <div class="top__end">
      {@render status?.()}
    </div>
  </header>

  <div class="body">
    {#if sidebar}
      <aside class="rail">{@render sidebar()}</aside>
    {/if}
    <main class="stage">{@render children?.()}</main>
  </div>
</div>

<style>
  .shell {
    position: relative;
    display: grid;
    grid-template-rows: 62px 1fr;
    min-height: 100vh;
    overflow: hidden;
    background:
      radial-gradient(700px at var(--cursor-x, 50%) var(--cursor-y, 0), rgba(255, 255, 255, 0.025), transparent 42%),
      transparent;
    color: var(--color-text-primary);
    font-family: var(--font-sans);
  }
  .shell::before {
    position: fixed;
    z-index: 0;
    inset: 0;
    background-image: radial-gradient(rgba(146, 176, 208, 0.18) 0.55px, transparent 0.55px);
    background-size: 28px 28px;
    content: "";
    opacity: 0.15;
    pointer-events: none;
    mask-image: linear-gradient(to bottom, black, transparent 70%);
  }
  .ambient {
    position: fixed;
    z-index: 0;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }
  .ambient span {
    position: absolute;
    width: 2px;
    height: 2px;
    border-radius: 50%;
    background: var(--color-brand-primary);
    box-shadow: 0 0 10px var(--color-brand-primary);
    opacity: 0;
    animation: particle-rise 14s linear infinite;
    will-change: transform, opacity;
  }
  .ambient span:nth-child(1) { left: 18%; bottom: 5%; animation-delay: -2s; }
  .ambient span:nth-child(2) { left: 44%; bottom: 2%; animation-delay: -7s; animation-duration: 18s; }
  .ambient span:nth-child(3) { left: 72%; bottom: 8%; animation-delay: -4s; background: var(--color-brand-accent); }
  .ambient span:nth-child(4) { left: 86%; bottom: 3%; animation-delay: -10s; animation-duration: 16s; }
  .ambient span:nth-child(5) { left: 59%; bottom: 12%; animation-delay: -12s; background: var(--color-brand-accent); }
  .top {
    position: relative;
    z-index: 5;
    display: grid;
    grid-template-columns: 218px minmax(300px, 540px) 1fr;
    align-items: center;
    gap: var(--space-5);
    padding: 0 58px 0 var(--space-5);
    border-bottom: 1px solid var(--color-border-default);
    background: rgba(6, 8, 13, 0.76);
    box-shadow: 0 10px 34px rgba(0, 0, 0, 0.22);
    backdrop-filter: blur(28px) saturate(160%);
  }
  .brand {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 11px;
  }
  .brand__sigil {
    position: relative;
    display: grid;
    width: 30px;
    height: 30px;
    place-items: center;
    border: 1px solid rgba(0, 240, 255, 0.22);
    border-radius: 9px;
    background: linear-gradient(145deg, rgba(0, 240, 255, 0.1), rgba(168, 85, 247, 0.08));
    box-shadow: inset 0 1px 0 rgba(255,255,255,.12), 0 0 18px rgba(0,240,255,.06);
    transform: rotate(45deg);
  }
  .brand__sigil::before,
  .brand__sigil::after,
  .brand__sigil i {
    position: absolute;
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--color-brand-primary);
    box-shadow: 0 0 7px var(--color-brand-primary);
    content: "";
  }
  .brand__sigil::before { top: 7px; left: 7px; }
  .brand__sigil::after { right: 7px; bottom: 7px; background: var(--color-brand-accent); }
  .brand__sigil i { inset: 0; margin: auto; }
  .wordmark {
    display: block;
    font-family: var(--font-display);
    font-size: 0.8125rem;
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.14em;
    line-height: 1.1;
    text-transform: uppercase;
  }
  .brand__sub {
    display: block;
    margin-top: 3px;
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: 0.5rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .finder {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 36px;
    padding: 0 10px 0 12px;
    border: 1px solid var(--color-border-default);
    border-radius: 999px;
    background: rgba(255,255,255,.028);
    color: var(--color-text-tertiary);
    cursor: pointer;
    box-shadow: inset 0 1px 3px rgba(0,0,0,.34);
    transition:
      border-color var(--duration-fast) var(--ease-standard),
      background-color var(--duration-fast) var(--ease-standard),
      box-shadow var(--duration-fast) var(--ease-standard);
  }
  .finder:hover,
  .finder:focus-visible {
    border-color: rgba(0, 240, 255, 0.24);
    background: rgba(0,240,255,.035);
    box-shadow: inset 0 1px 3px rgba(0,0,0,.34), 0 0 22px rgba(0,240,255,.055);
    outline: none;
  }
  .finder__icon {
    position: relative;
    flex: 0 0 auto;
    width: 12px;
    height: 12px;
    border: 1.5px solid var(--color-text-tertiary);
    border-radius: 50%;
  }
  .finder__icon::after {
    position: absolute;
    right: -4px;
    bottom: -2px;
    width: 5px;
    height: 1px;
    background: var(--color-text-tertiary);
    content: "";
    transform: rotate(45deg);
    transform-origin: left;
  }
  .finder__placeholder {
    flex: 1;
    overflow: hidden;
    font-size: var(--font-size-sm);
    letter-spacing: 0.01em;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .finder__keys { display: inline-flex; gap: 4px; }
  kbd {
    min-width: 1.2rem;
    padding: 0 4px;
    border: 1px solid var(--color-border-default);
    border-radius: 5px;
    background: rgba(255,255,255,.04);
    font-family: var(--font-mono);
    font-size: 10px;
    line-height: 1.5;
    text-align: center;
  }
  .top__end {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: var(--space-4);
    min-width: 0;
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--color-text-secondary);
  }
  .body {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: 218px minmax(0, 1fr);
    min-height: 0;
  }
  .rail {
    min-height: 0;
    padding: var(--space-5) var(--space-3);
    overflow-y: auto;
    border-right: 1px solid var(--color-border-default);
    background: linear-gradient(180deg, rgba(10, 14, 23, 0.78), rgba(6, 8, 13, 0.6));
    backdrop-filter: blur(24px);
  }
  .stage {
    min-width: 0;
    overflow: auto;
    background:
      radial-gradient(circle at 78% 8%, rgba(168, 85, 247, 0.055), transparent 28rem),
      radial-gradient(circle at 24% 20%, rgba(0, 240, 255, 0.035), transparent 32rem),
      transparent;
  }
  @keyframes particle-rise {
    0% { opacity: 0; transform: translate3d(0, 20px, 0) scale(.8); }
    15% { opacity: .35; }
    80% { opacity: .08; }
    100% { opacity: 0; transform: translate3d(42px, -92vh, 0) scale(1.4); }
  }
  @media (max-width: 960px) {
    .top { grid-template-columns: auto 1fr; }
    .top__end { display: none; }
    .body { grid-template-columns: 1fr; }
    .rail { display: none; }
  }
</style>
