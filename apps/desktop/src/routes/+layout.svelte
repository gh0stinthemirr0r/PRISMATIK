<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import '../app.css';
  import WindowGrips from "$lib/prismatik/WindowGrips.svelte";
  import WindowChrome from "$lib/WindowChrome.svelte";
  import { platform } from "$lib/prismatik/platform.svelte";

  let { children } = $props();

  // /workspace draws its own chrome inside TerminalFrame's TopBar. Every other
  // route (hero, onboarding, jobs, sessions) would otherwise have no way to
  // move or close the decorationless window, so it gets the minimal bar.
  // In plain browser dev there is no window to drag, so no chrome either.
  const inTauri =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  const needsChrome = $derived(
    inTauri && !page.url.pathname.startsWith("/workspace"),
  );

  let theme = $state<"light" | "dark">("dark");

  function toggleTheme() {
    theme = theme === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = theme;
  }

  onMount(() => {
    // Chrome shape depends on the host OS, so resolve it before anything that
    // draws window controls or resize grips decides whether to render.
    void platform.resolve();
    document.documentElement.dataset.theme = theme;
    document.documentElement.lang = "en-US";
  });
</script>

{#if needsChrome}
  <WindowChrome {theme} onToggleTheme={toggleTheme} />
{/if}

{@render children()}

<WindowGrips />
