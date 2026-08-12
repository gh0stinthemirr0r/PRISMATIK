<script lang="ts">
  /**
   * Splash. Shown briefly on launch, then the workspace takes over.
   *
   * This used to be a marketing hero with an "Enter workspace" button — a page
   * you had to dismiss every single launch to reach the thing you opened the
   * app for. A desk tool should not ask permission to start.
   *
   * First run still diverts to onboarding, because a terminal with no provider
   * connected has nothing to show and the operator needs to be told why.
   */
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";

  /** Long enough to read the wordmark, short enough not to be in the way. */
  const HOLD_MS = 1000;

  let leaving = $state(false);

  function enter(): void {
    if (leaving) return;
    leaving = true;
    void goto("/workspace");
  }

  onMount(() => {
    if (localStorage.getItem("first_run_complete") !== "true") {
      void goto("/onboarding");
      return;
    }

    // Honour reduced motion by skipping the hold entirely rather than by
    // animating differently: the point of the pause is decorative, and someone
    // who has asked for less motion has asked not to wait for decoration.
    const reduced =
      typeof matchMedia === "function" &&
      matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduced) {
      enter();
      return;
    }

    const timer = setTimeout(enter, HOLD_MS);
    // Any deliberate input skips the remaining hold.
    const skip = () => enter();
    window.addEventListener("keydown", skip, { once: true });
    window.addEventListener("pointerdown", skip, { once: true });
    return () => {
      clearTimeout(timer);
      window.removeEventListener("keydown", skip);
      window.removeEventListener("pointerdown", skip);
    };
  });
</script>

<svelte:head>
  <title>PRISMATIK</title>
  <meta name="description" content="Evidence-chained market intelligence." />
</svelte:head>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="splash" class:leaving onclick={enter} role="presentation">
  <div class="splash__mark" aria-hidden="true">
    <svg viewBox="0 0 64 64" fill="none">
      <path d="M32 6 L58 52 L6 52 Z" stroke="var(--color-brand-primary)" stroke-width="2" />
      <path d="M32 20 L46 45 L18 45 Z" fill="var(--color-brand-accent)" opacity="0.5" />
      <path d="M32 6 L32 52" stroke="var(--color-brand-primary)" stroke-width="1" opacity="0.55" />
    </svg>
  </div>

  <p class="splash__brand">PRISMATIK<span>.</span></p>
  <p class="splash__tagline">Evidence-chained market intelligence</p>

  <div class="splash__bar" aria-hidden="true"><i></i></div>
  <p class="splash__hint">Click or press any key to skip</p>
</div>

<style>
  .splash {
    position: relative;
    display: flex;
    min-height: 100vh;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    background:
      radial-gradient(ellipse 70% 50% at 50% 30%, rgba(0, 240, 255, 0.1), transparent 62%),
      radial-gradient(ellipse 50% 40% at 20% 90%, rgba(168, 85, 247, 0.1), transparent 58%),
      var(--color-surface-0);
    color: var(--color-text-primary);
    cursor: pointer;
    transition: opacity 180ms ease;
  }
  .splash.leaving {
    opacity: 0;
  }
  .splash__mark {
    width: 68px;
    height: 68px;
    animation: rise 520ms ease-out both;
  }
  .splash__mark svg {
    width: 100%;
    height: 100%;
  }
  .splash__brand {
    margin: 4px 0 0;
    font-size: clamp(2rem, 5vw, 3.1rem);
    font-weight: 600;
    letter-spacing: -0.05em;
    animation: rise 520ms 60ms ease-out both;
  }
  .splash__brand span {
    color: var(--color-brand-primary);
  }
  .splash__tagline {
    margin: 0;
    color: var(--color-text-secondary);
    font-size: 0.78rem;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    animation: rise 520ms 120ms ease-out both;
  }
  .splash__bar {
    width: 132px;
    height: 2px;
    margin-top: 14px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--color-border-strong);
  }
  /* Tracks the hold, so the wait is legible rather than an unexplained pause. */
  .splash__bar i {
    display: block;
    width: 100%;
    height: 100%;
    background: linear-gradient(90deg, var(--color-brand-primary), var(--color-brand-accent));
    transform-origin: left center;
    animation: fill 1000ms linear both;
  }
  .splash__hint {
    margin: 6px 0 0;
    color: var(--color-text-secondary);
    font-size: 0.6rem;
    letter-spacing: 0.14em;
    opacity: 0.5;
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @keyframes fill {
    from {
      transform: scaleX(0);
    }
    to {
      transform: scaleX(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .splash,
    .splash__mark,
    .splash__brand,
    .splash__tagline,
    .splash__bar i {
      animation: none;
      transition: none;
    }
  }
</style>
