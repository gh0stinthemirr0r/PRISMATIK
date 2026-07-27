<script lang="ts">
  import { onMount } from "svelte";
  import '../app.css';

  let { children } = $props();
  let theme = $state<"light" | "dark">("dark");

  function applyTheme(next: "light" | "dark") {
    theme = next;
    document.documentElement.dataset.theme = next;
    localStorage.setItem("prismatik-theme", next);
  }

  onMount(() => {
    const saved = localStorage.getItem("prismatik-theme");
    applyTheme(
      saved === "dark" || saved === "light"
        ? saved
        : "dark",
    );
  });
</script>

{@render children()}

<button
  class="theme-toggle"
  type="button"
  aria-label={`Use ${theme === "dark" ? "light" : "dark"} theme`}
  title={`Use ${theme === "dark" ? "light" : "dark"} theme`}
  onclick={() => applyTheme(theme === "dark" ? "light" : "dark")}
>
  <span class="theme-toggle__orb" aria-hidden="true"></span>
</button>

<style>
  .theme-toggle {
    position: fixed;
    z-index: 100;
    top: 14px;
    right: 18px;
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
    border: 1px solid var(--color-border-default);
    border-radius: 50%;
    background: rgba(9, 13, 22, 0.72);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.09);
    color: var(--color-text-secondary);
    cursor: pointer;
  }
  .theme-toggle:hover {
    border-color: rgba(0, 240, 255, 0.4);
    color: var(--color-text-primary);
    box-shadow: var(--shadow-cyan);
  }
  .theme-toggle__orb {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--color-brand-primary), var(--color-brand-accent));
    box-shadow:
      0 0 8px rgba(0, 240, 255, 0.7),
      0 0 14px rgba(168, 85, 247, 0.45);
  }
</style>
