<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { platform } from "$lib/prismatik/platform.svelte";

  let {
    theme,
    onToggleTheme,
  }: {
    theme: "light" | "dark";
    onToggleTheme: () => void;
  } = $props();

  const tauriRuntime = $derived(
    typeof window !== "undefined" &&
      ("__TAURI_INTERNALS__" in window || "__TAURI__" in window),
  );

  // macOS supplies traffic lights of its own; ours would duplicate them on the
  // wrong side. The bar still renders — it is the drag region and identity —
  // but inset so it does not sit underneath them.
  const showWindowButtons = $derived(
    tauriRuntime && platform.resolved && !platform.usesSystemWindowControls,
  );

  let chromeEl: HTMLElement | undefined = $state();
  let actionsEl: HTMLElement | undefined = $state();

  async function windowAction(action: "minimize" | "maximize" | "close") {
    if (!tauriRuntime) return;
    const appWindow = getCurrentWindow();
    if (action === "minimize") await appWindow.minimize();
    if (action === "maximize") await appWindow.toggleMaximize();
    if (action === "close") await appWindow.close();
  }

  // Drive drag from a document-level pointerdown so every pixel of the chrome
  // bar drags, regardless of which child element the pointer lands on. We
  // exclude the actions region so the min/max/close buttons still get their
  // normal click. This is more reliable than relying on event bubbling from
  // a parent mousedown handler combined with `data-tauri-drag-region`.
  onMount(() => {
    if (!tauriRuntime) return;
    let pendingDrag = false;

    function onPointerDown(event: PointerEvent) {
      if (event.button !== 0) return;
      const target = event.target as Node | null;
      if (!target || !chromeEl) return;
      if (!chromeEl.contains(target)) return;
      if (actionsEl && actionsEl.contains(target)) return;
      pendingDrag = true;
      event.preventDefault();
      void getCurrentWindow()
        .startDragging()
        .catch(() => {
          /* drag start can race with focus loss; non-fatal */
        })
        .finally(() => {
          pendingDrag = false;
        });
    }

    function onPointerCancel() {
      pendingDrag = false;
    }

    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("pointercancel", onPointerCancel);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("pointercancel", onPointerCancel);
      untrack(() => pendingDrag);
    };
  });
</script>

<header
  bind:this={chromeEl}
  class="chrome"
  class:chrome-inset={platform.usesSystemWindowControls}
  role="toolbar"
  aria-label="Application window controls"
  tabindex="-1"
  ondblclick={() => void windowAction("maximize")}
>
  <div class="chrome__identity">
    <span class="mark" aria-hidden="true"><i></i><i></i><i></i></span>
    <strong>PRISMATIK</strong>
  </div>

  <div class="chrome__actions" bind:this={actionsEl}>
    <button
      class="theme"
      type="button"
      aria-label={`Use ${theme === "dark" ? "light" : "dark"} theme`}
      title={`Use ${theme === "dark" ? "light" : "dark"} theme`}
      onclick={onToggleTheme}
    >
      <span aria-hidden="true"></span>
    </button>
    {#if showWindowButtons}
    <span class="divider" aria-hidden="true"></span>
    <button
      type="button"
      aria-label="Minimize PRISMATIK"
      title="Minimize"
      disabled={!tauriRuntime}
      onclick={() => void windowAction("minimize")}
    >
      <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 8.5h8" /></svg>
    </button>
    <button
      type="button"
      aria-label="Maximize or restore PRISMATIK"
      title="Maximize or restore"
      disabled={!tauriRuntime}
      onclick={() => void windowAction("maximize")}
    >
      <svg viewBox="0 0 12 12" aria-hidden="true"><rect x="2.25" y="2.25" width="7.5" height="7.5" rx="1" /></svg>
    </button>
    <button
      class="close"
      type="button"
      aria-label="Close PRISMATIK"
      title="Close"
      disabled={!tauriRuntime}
      onclick={() => void windowAction("close")}
    >
      <svg viewBox="0 0 12 12" aria-hidden="true"><path d="m2.5 2.5 7 7m0-7-7 7" /></svg>
    </button>
    {/if}
  </div>
</header>

<style>
  .chrome {
    position: relative;
    z-index: 1000;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    height: 38px;
    align-items: center;
    border-bottom: 1px solid rgba(151, 171, 199, 0.17);
    background:
      linear-gradient(90deg, rgba(0, 240, 255, 0.025), transparent 28%, rgba(168, 85, 247, 0.025)),
      rgba(5, 8, 14, 0.96);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.24);
    color: var(--color-text-secondary);
    user-select: none;
  }
  .chrome__identity,
  .chrome__actions {
    display: flex;
    align-items: center;
  }
  .chrome__identity {
    min-width: 0;
    gap: 8px;
    padding-left: 12px;
  }
  /* Clear the macOS traffic lights, which are drawn over the webview. */
  .chrome-inset .chrome__identity {
    padding-left: 78px;
  }
  .chrome__identity strong {
    color: var(--color-text-primary);
    font-family: var(--font-display);
    font-size: 0.66rem;
    letter-spacing: 0.15em;
  }
  .mark {
    position: relative;
    display: grid;
    width: 17px;
    height: 17px;
    place-items: center;
    border: 1px solid rgba(0, 240, 255, 0.28);
    border-radius: 5px;
    background: linear-gradient(145deg, rgba(0, 240, 255, 0.12), rgba(168, 85, 247, 0.08));
    transform: rotate(45deg);
  }
  .mark i {
    position: absolute;
    width: 2px;
    height: 2px;
    border-radius: 50%;
    background: var(--color-brand-primary);
    box-shadow: 0 0 6px var(--color-brand-primary);
  }
  .mark i:first-child { transform: translate(-4px, -4px); }
  .mark i:last-child { transform: translate(4px, 4px); background: var(--color-brand-accent); }
  .chrome__actions {
    justify-content: flex-end;
    align-self: stretch;
  }
  .chrome__actions button {
    display: grid;
    width: 42px;
    height: 100%;
    padding: 0;
    place-items: center;
    border: 0;
    background: transparent;
    color: var(--color-text-secondary);
    cursor: pointer;
  }
  .chrome__actions button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.055);
    color: var(--color-text-primary);
  }
  .chrome__actions button.close:hover:not(:disabled) {
    background: rgba(255, 72, 101, 0.82);
    color: white;
  }
  .chrome__actions button:disabled {
    cursor: default;
    opacity: 0.32;
  }
  .chrome__actions svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.15;
  }
  .theme span {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--color-brand-primary), var(--color-brand-accent));
    box-shadow: 0 0 8px rgba(0, 240, 255, 0.7);
  }
  .divider {
    width: 1px;
    height: 16px;
    background: var(--color-border-default);
  }
</style>
