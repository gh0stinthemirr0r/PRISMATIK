<script lang="ts">
  import { onDestroy } from "svelte";
  import { ensurePerspective, type PerspectiveRow } from "./bootstrap";

  let {
    rows = [],
    height = 320,
    plugin = "Datagrid",
    title = "Perspective",
  }: {
    rows?: PerspectiveRow[];
    height?: number;
    plugin?: string;
    title?: string;
  } = $props();

  let host = $state<HTMLDivElement | null>(null);
  let status = $state<"loading" | "ready" | "error">("loading");
  let error = $state<string | null>(null);

  let viewer: HTMLPerspectiveViewerElement | null = null;
  // Perspective client/table handles are dynamically typed from WASM bindings.
  let table: { delete: (force?: boolean) => Promise<unknown> } | null = null;
  let client: {
    table: (data: PerspectiveRow[]) => Promise<{ delete: (force?: boolean) => Promise<unknown> }>;
    terminate?: () => void;
  } | null = null;
  let generation = 0;

  async function syncTable(nextRows: PerspectiveRow[]) {
    const gen = ++generation;
    status = "loading";
    error = null;
    try {
      const perspective = await ensurePerspective();
      if (gen !== generation || !host) return;

      if (!viewer) {
        viewer = document.createElement("perspective-viewer");
        viewer.setAttribute("theme", "Pro Dark");
        host.replaceChildren(viewer);
      }

      if (!client) {
        client = await perspective.worker();
      }

      if (table) {
        await table.delete(true);
        table = null;
      }

      const payload = nextRows.length ? nextRows : [{ _empty: 0 }];
      table = await client.table(payload);
      await viewer.load(table);
      await viewer.restore({
        plugin,
        settings: false,
        title,
      });
      if (gen === generation) status = "ready";
    } catch (cause) {
      if (gen === generation) {
        status = "error";
        error = cause instanceof Error ? cause.message : String(cause);
      }
    }
  }

  $effect(() => {
    const snapshot = rows;
    if (!host) return;
    void syncTable(snapshot);
  });

  onDestroy(() => {
    generation += 1;
    void (async () => {
      try {
        if (viewer) await viewer.delete();
      } catch {
        /* viewer may already be detached */
      }
      try {
        if (table) await table.delete(true);
      } catch {
        /* ignore */
      }
      try {
        client?.terminate?.();
      } catch {
        /* ignore */
      }
      viewer = null;
      table = null;
      client = null;
    })();
  });
</script>

<div class="psp" style={`height:${height}px`}>
  <div class="psp__host" bind:this={host} role="region" aria-label={title}></div>
  {#if status === "loading"}
    <div class="psp__overlay">Loading Perspective…</div>
  {:else if status === "error"}
    <div class="psp__overlay error">{error}</div>
  {/if}
</div>

<style>
  .psp {
    position: relative;
    border: 1px solid var(--color-border-default);
    background: var(--color-surface-1);
    min-height: 160px;
  }
  .psp__host,
  .psp__host :global(perspective-viewer) {
    display: block;
    width: 100%;
    height: 100%;
  }
  .psp__overlay {
    position: absolute;
    inset: auto 0 0;
    padding: var(--space-2) var(--space-3);
    font-size: var(--font-size-xs);
    color: var(--color-text-secondary);
    background: color-mix(in srgb, var(--color-surface-1) 88%, transparent);
    pointer-events: none;
  }
  .psp__overlay.error {
    color: var(--color-danger);
  }
</style>
