<script lang="ts">
  import { goto } from "$app/navigation";
  import { Button } from "@prismatik/ui";

  let step = $state<"welcome" | "suitability" | "mode">("welcome");
  let suitable = $state(false);

  function finish() {
    localStorage.setItem("first_run_complete", "true");
    void goto("/workspace");
  }
</script>

<svelte:head><title>Welcome · PRISMATIK</title></svelte:head>

<main class="onboarding">
  <section class="card">
    <div class="eyebrow">PRISMATIK · FIRST RUN</div>
    {#if step === "welcome"}
      <h1>Evidence before conviction.</h1>
      <p>
        Explore a deterministic market workspace where every quote keeps its provider and
        retrieval time attached.
      </p>
      <Button variant="primary" size="lg" onclick={() => (step = "suitability")}>
        Get started
      </Button>
    {:else if step === "suitability"}
      <h1>Know the limits.</h1>
      <p>PRISMATIK is research software, not investment advice or an execution venue.</p>
      <label>
        <input type="checkbox" bind:checked={suitable} />
        I understand that market data can be delayed, incomplete, or wrong.
      </label>
      <Button
        variant="primary"
        size="lg"
        disabled={!suitable}
        onclick={() => (step = "mode")}
      >
        Continue
      </Button>
    {:else}
      <h1>Choose a data mode.</h1>
      <button class="mode" type="button" aria-pressed="true">
        <strong>Demo cassette</strong>
        <span>Offline, deterministic, and safe for first exploration.</span>
      </button>
      <Button variant="primary" size="lg" onclick={finish}>Enter workspace</Button>
    {/if}
    <div class="steps" aria-label="Onboarding progress">
      <span class:active={step === "welcome"}></span>
      <span class:active={step === "suitability"}></span>
      <span class:active={step === "mode"}></span>
    </div>
  </section>
</main>

<style>
  .onboarding {
    position: relative;
    display: grid;
    min-height: 100vh;
    padding: var(--space-6);
    place-items: center;
    background:
      linear-gradient(rgba(255,255,255,.022) 1px, transparent 1px),
      linear-gradient(90deg, rgba(255,255,255,.022) 1px, transparent 1px),
      radial-gradient(circle at 75% 10%, rgba(0, 240, 255, .11), transparent 42%),
      radial-gradient(circle at 14% 85%, rgba(168, 85, 247, .11), transparent 40%),
      transparent;
    background-size: 34px 34px, 34px 34px, auto, auto, auto;
    color: var(--color-text-primary);
  }
  .card {
    display: grid;
    width: min(100%, 36rem);
    gap: var(--space-5);
    padding: var(--space-7);
    overflow: hidden;
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-xl);
    background:
      radial-gradient(circle at 100% 0, rgba(0,240,255,.055), transparent 44%),
      var(--color-surface-1);
    box-shadow: var(--shadow-panel), 0 0 90px -60px rgba(0,240,255,.9);
    backdrop-filter: blur(28px) saturate(170%);
    animation: reveal var(--duration-slow) var(--ease-standard) both;
  }
  .eyebrow {
    color: var(--color-brand-primary);
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    letter-spacing: .14em;
  }
  h1 { margin: 0; font-size: clamp(2rem, 5vw, 3.2rem); letter-spacing: -.055em; line-height: 1; }
  p { margin: 0; color: var(--color-text-secondary); line-height: var(--line-height-relaxed); }
  label { display: flex; gap: var(--space-3); color: var(--color-text-secondary); }
  .mode {
    display: grid;
    gap: var(--space-2);
    padding: var(--space-4);
    border: 1px solid var(--color-brand-primary);
    border-radius: var(--radius-md);
    background:
      radial-gradient(circle at 100% 0, rgba(0,240,255,.1), transparent 65%),
      rgba(255,255,255,.025);
    box-shadow: inset 0 1px 0 rgba(255,255,255,.08), var(--shadow-cyan);
    color: var(--color-text-primary);
    text-align: left;
  }
  .mode span { color: var(--color-text-secondary); font-size: var(--font-size-sm); }
  .steps { display: flex; gap: var(--space-2); }
  .steps span { width: 34px; height: 2px; border-radius: 999px; background: var(--color-surface-3); }
  .steps span.active { background: var(--color-brand-primary); box-shadow: 0 0 8px var(--color-brand-primary); }
  @keyframes reveal {
    from { opacity: 0; transform: translateY(14px) scale(.985); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }
</style>
