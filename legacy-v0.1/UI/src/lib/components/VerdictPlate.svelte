<script lang="ts">
  // The signature element: one engraved plate that states the finding plainly.
  import { fly } from 'svelte/transition';
  export let verdict: string;
  export let positive: boolean;
</script>

<div class="plate" class:positive in:fly={{ y: 10, duration: 420 }}>
  <div class="plate-label">Verdict</div>
  <div class="plate-text">{verdict}</div>
</div>

<style>
  .plate {
    border: 1px solid var(--gilt-dim);
    border-left: 3px solid var(--oxblood);
    background: linear-gradient(180deg, var(--ink-2), var(--ink-3));
    border-radius: var(--radius);
    padding: 18px 22px;
  }
  .plate.positive { border-left-color: var(--patina); }
  .plate-label {
    font-family: var(--mono);
    font-size: 11px;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: var(--gilt);
    margin-bottom: 6px;
  }
  .plate-text {
    font-family: var(--display);
    font-size: 21px;
    font-weight: 400;
    line-height: 1.35;
  }
</style>
