<script lang="ts">
  import { Button } from "@prismatik/ui";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";

  onMount(() => {
    if (localStorage.getItem("first_run_complete") !== "true") {
      void goto("/onboarding");
    }
  });
</script>

<svelte:head>
  <title>PRISMATIK</title>
  <meta
    name="description"
    content="Evidence-chained crypto intelligence. Local-first. Deterministic."
  />
</svelte:head>

<div class="hero">
  <div class="hero__atmosphere" aria-hidden="true">
    <svg class="hero__grid" viewBox="0 0 1440 900" preserveAspectRatio="xMidYMid slice">
      <defs>
        <linearGradient id="fade" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="currentColor" stop-opacity="0.18" />
          <stop offset="55%" stop-color="currentColor" stop-opacity="0.06" />
          <stop offset="100%" stop-color="currentColor" stop-opacity="0" />
        </linearGradient>
        <linearGradient id="path" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stop-color="var(--color-brand-primary)" stop-opacity="0" />
          <stop offset="35%" stop-color="var(--color-brand-primary)" stop-opacity="0.9" />
          <stop offset="100%" stop-color="var(--color-brand-accent)" stop-opacity="0.35" />
        </linearGradient>
      </defs>
      <g stroke="url(#fade)" stroke-width="1" fill="none">
        {#each Array.from({ length: 24 }, (_, i) => i) as i}
          <line x1={i * 60} y1="0" x2={i * 60} y2="900" />
        {/each}
        {#each Array.from({ length: 16 }, (_, i) => i) as i}
          <line x1="0" y1={i * 56} x2="1440" y2={i * 56} />
        {/each}
      </g>
      <path
        class="hero__trace"
        d="M0 620 C160 600 220 540 340 520 C480 490 520 610 680 580 C820 555 860 430 1020 410 C1160 392 1240 470 1440 390"
        fill="none"
        stroke="url(#path)"
        stroke-width="2.5"
      />
      <path
        class="hero__trace hero__trace--delay"
        d="M0 700 C180 690 260 650 400 640 C560 628 600 720 760 700 C920 678 980 560 1140 548 C1260 540 1340 580 1440 560"
        fill="none"
        stroke="currentColor"
        stroke-opacity="0.22"
        stroke-width="1.5"
      />
    </svg>
  </div>

  <div class="hero__status" aria-label="System status">
    <span class="status-wordmark">PRISMATIK / NODE 01</span>
    <span><i></i> Evidence fabric online</span>
    <span>UTC 20:31:08</span>
  </div>

  <div class="hero__content">
    <p class="overline">Deterministic market intelligence</p>
    <p class="brand">PRISMATIK<span class="brand__point">.</span></p>
    <h1>See what the market knows—and where certainty fractures.</h1>
    <p class="lede">
      A local-first intelligence instrument for market structure, correlation,
      reproducible research, and evidence that survives scrutiny.
    </p>
    <div class="cta">
      <Button variant="primary" size="lg" onclick={() => goto("/workspace")}>
        Enter workspace
      </Button>
      <Button variant="ghost" size="lg" onclick={() => goto("/workspace?panel=providers")}>
        Provider plane
      </Button>
    </div>
    <div class="hero__telemetry" aria-label="Platform telemetry">
      <div><span>EVIDENCE INTEGRITY</span><strong>99.97%</strong></div>
      <div><span>ACTIVE SIGNALS</span><strong>184</strong></div>
      <div><span>MEDIAN LATENCY</span><strong>18 ms</strong></div>
    </div>
  </div>
</div>

<style>
  .hero {
    position: relative;
    display: grid;
    min-height: 100vh;
    overflow: hidden;
    background:
      radial-gradient(ellipse 80% 55% at 70% 15%, rgba(0, 240, 255, .11), transparent 60%),
      radial-gradient(ellipse 50% 40% at 10% 90%, rgba(168, 85, 247, .12), transparent 55%),
      linear-gradient(165deg, rgba(13,17,28,.76), var(--color-surface-0) 55%, rgba(13,17,28,.66));
    color: var(--color-text-primary);
  }
  .hero__atmosphere {
    position: absolute;
    inset: 0;
    color: var(--color-border-strong);
    pointer-events: none;
  }
  .hero__grid {
    width: 100%;
    height: 100%;
    opacity: .65;
    mask-image: linear-gradient(to bottom, black, transparent 92%);
  }
  .hero__trace {
    stroke-dasharray: 1600;
    stroke-dashoffset: 1600;
    animation: draw var(--duration-slow) var(--ease-decelerate) 0.15s forwards;
  }
  .hero__trace--delay {
    animation-delay: 0.35s;
  }
  @keyframes draw {
    to { stroke-dashoffset: 0; }
  }
  .hero__content {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    width: min(100%, 72rem);
    min-height: 100vh;
    padding: clamp(5rem, 8vw, 8rem) clamp(2rem, 7vw, 7rem) 4rem;
    animation: rise var(--duration-slow) var(--ease-decelerate) both;
  }
  .hero__status {
    position: absolute;
    z-index: 2;
    top: 0;
    left: 0;
    display: flex;
    width: 100%;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: 22px clamp(2rem, 4vw, 4rem);
    border-bottom: 1px solid var(--color-border-default);
    background: rgba(6,8,13,.44);
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: .5625rem;
    letter-spacing: .1em;
    text-transform: uppercase;
    backdrop-filter: blur(18px);
  }
  .hero__status span { display: inline-flex; align-items: center; gap: 8px; }
  .hero__status i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--color-success);
    box-shadow: 0 0 8px var(--color-success);
    animation: breathe 1.8s ease-in-out infinite;
  }
  .status-wordmark { color: var(--color-text-primary); font-weight: var(--font-weight-semibold); }
  .overline {
    margin: 0 0 var(--space-4);
    color: var(--color-brand-primary);
    font-family: var(--font-mono);
    font-size: .625rem;
    font-weight: var(--font-weight-semibold);
    letter-spacing: .16em;
    text-transform: uppercase;
  }
  @keyframes rise {
    from { opacity: 0; transform: translateY(12px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .brand {
    margin: 0 0 var(--space-5);
    color: var(--color-brand-primary);
    font-family: var(--font-display);
    font-size: clamp(3.5rem, 10vw, 8rem);
    font-weight: var(--font-weight-bold);
    letter-spacing: -.075em;
    line-height: .78;
    text-shadow: 0 0 52px rgba(0,240,255,.12);
  }
  .brand__point { color: var(--color-brand-accent); text-shadow: 0 0 30px rgba(168,85,247,.45); }
  h1 {
    max-width: 24ch;
    margin: 0;
    font-family: var(--font-display);
    font-size: clamp(1.35rem, 2.4vw, 2.2rem);
    font-weight: var(--font-weight-medium);
    letter-spacing: -0.02em;
    line-height: 1.16;
    color: var(--color-text-primary);
  }
  .lede {
    max-width: 42rem;
    margin: var(--space-4) 0 0;
    color: var(--color-text-secondary);
    font-size: var(--font-size-md);
    line-height: var(--line-height-relaxed);
  }
  .cta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    margin-top: var(--space-6);
  }
  .hero__telemetry {
    display: grid;
    grid-template-columns: repeat(3, minmax(120px, 1fr));
    gap: var(--space-3);
    width: min(100%, 42rem);
    margin-top: clamp(2.5rem, 6vh, 5rem);
  }
  .hero__telemetry div {
    display: grid;
    gap: 5px;
    padding-top: var(--space-3);
    border-top: 1px solid var(--color-border-default);
  }
  .hero__telemetry span {
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: .5625rem;
    letter-spacing: .08em;
  }
  .hero__telemetry strong {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
  }
  @keyframes breathe {
    50% { opacity: .65; box-shadow: 0 0 0 5px rgba(16,185,129,0); }
  }
  @media (max-width: 640px) {
    .hero__status span:nth-child(2) { display: none; }
    .hero__telemetry { grid-template-columns: 1fr; }
  }
  @media (prefers-reduced-motion: reduce) {
    .hero__trace,
    .hero__trace--delay,
    .hero__content {
      animation: none;
      stroke-dashoffset: 0;
      opacity: 1;
      transform: none;
    }
  }
</style>
