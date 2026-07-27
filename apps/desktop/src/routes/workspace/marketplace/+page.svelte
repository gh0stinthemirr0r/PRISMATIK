<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip, StaleDataMarker, WorkspaceShell } from "@prismatik/ui";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";

  type Listing = {
    pluginId: string;
    version: string;
    publisher: string;
    capabilities: string[];
    licenseClass: string;
    status: string;
    installable?: boolean;
    installGate?: string;
  };

  type Catalog = {
    provider: string;
    retrievedAt: string;
    listings: Listing[];
  };

  const fallback: Catalog = {
    provider: "ui-static-fallback",
    retrievedAt: "2026-07-25T20:00:00Z",
    listings: [
      {
        pluginId: "sma-cross-demo",
        version: "0.3.1",
        publisher: "prismatik-labs",
        capabilities: ["clock"],
        licenseClass: "oss",
        status: "approved",
      },
      {
        pluginId: "vol-regime",
        version: "1.0.0-rc2",
        publisher: "community/vol-regime",
        capabilities: ["clock", "entropy"],
        licenseClass: "oss",
        status: "approved",
      },
      {
        pluginId: "macro-bridge",
        version: "0.9.0",
        publisher: "third-party/macro-bridge",
        capabilities: ["network", "clock", "host_ai"],
        licenseClass: "commercial",
        status: "in_review",
      },
    ],
  };

  let catalog = $state<Catalog>(fallback);
  let provenance = $state("static fallback");
  let pendingInstall = $state<Listing | null>(null);
  let disclosureAccepted = $state(false);
  let installNotice = $state<string | null>(null);

  function statusChip(status: string): "confirmed" | "uncertain" | "contradicted" {
    if (status === "approved") return "confirmed";
    if (status === "revoked") return "contradicted";
    return "uncertain";
  }

  function isSigned(listing: Listing): boolean {
    if (listing.installable != null) return listing.installable;
    return listing.status === "approved";
  }

  function openInstall(plugin: Listing) {
    pendingInstall = plugin;
    disclosureAccepted = false;
  }

  function cancelInstall() {
    pendingInstall = null;
    disclosureAccepted = false;
  }

  async function confirmInstall() {
    if (!pendingInstall || !disclosureAccepted) return;
    try {
      const preview = await invoke<{
        accepted: boolean;
        gate: string;
        provider: string;
      }>("preview_plugin_install", {
        pluginId: pendingInstall.pluginId,
        version: pendingInstall.version,
        publisherId: pendingInstall.publisher,
        capabilities: pendingInstall.capabilities,
      });
      installNotice = preview.accepted
        ? `${pendingInstall.pluginId} install accepted via ${preview.provider}`
        : `${pendingInstall.pluginId} install denied — ${preview.gate}`;
    } catch {
      installNotice = `${pendingInstall.pluginId} install stub recorded — host IPC unavailable.`;
    }
    pendingInstall = null;
    disclosureAccepted = false;
    setTimeout(() => (installNotice = null), 3200);
  }

  onMount(async () => {
    try {
      catalog = await invoke<Catalog>("get_marketplace_listings");
      provenance = "trusted core";
    } catch {
      provenance = "browser fallback";
    }
  });
</script>

<svelte:head><title>Marketplace · PRISMATIK</title></svelte:head>

<WorkspaceShell title="PRISMATIK">
  {#snippet sidebar()}
    <div class="rail-label">Experiences</div>
    <ExperiencesNav active="marketplace" />
  {/snippet}
  {#snippet status()}
    <EvidenceChip status="confirmed" label={provenance} />
  {/snippet}

  <div class="canvas">
    <header class="page-head">
      <div>
        <h1>Plugin marketplace</h1>
        <p>
          Signed manifests with capability disclosure before install — static listing until Wave 7
          registry connects; IPC overlay when the stub floor is available.
        </p>
      </div>
      <div class="chips">
        <EvidenceChip status="confirmed" label={catalog.provider} />
        <StaleDataMarker eventTime={catalog.retrievedAt} maxAge={86_400_000} />
      </div>
    </header>

    <ol class="catalog" aria-label="Plugin catalog">
      {#each catalog.listings as plugin}
        <li class="plugin">
          <div class="plugin__head">
            <div>
              <div class="plugin__name">{plugin.pluginId}</div>
              <div class="plugin__meta">
                {plugin.publisher} · v{plugin.version} · {plugin.licenseClass}
              </div>
            </div>
            <div class="plugin__status">
              <EvidenceChip status={statusChip(plugin.status)} label={plugin.status} />
              {#if isSigned(plugin)}
                <EvidenceChip status="confirmed" label="signed manifest" />
              {:else}
                <EvidenceChip status="contradicted" label="unsigned" />
              {/if}
            </div>
          </div>
          <div class="plugin__caps">
            <span class="caps-label">Requested capabilities</span>
            <div class="cap-chips">
              {#each plugin.capabilities as cap}
                <EvidenceChip
                  status={cap.includes("network") || cap.includes("host_ai") ? "uncertain" : "confirmed"}
                  label={cap}
                />
              {/each}
            </div>
          </div>
          <button type="button" class="text-action" onclick={() => openInstall(plugin)}>
            Review & install
          </button>
        </li>
      {/each}
    </ol>
  </div>
</WorkspaceShell>

{#if pendingInstall}
  <div class="overlay" role="presentation" onclick={cancelInstall}></div>
  <dialog class="disclosure" open aria-labelledby="disclosure-title">
    <header>
      <h2 id="disclosure-title">Capability disclosure</h2>
      <p>Review grants before installing <strong>{pendingInstall.pluginId}</strong>.</p>
    </header>

    <section aria-label="Requested capabilities">
      <div class="label">Host capabilities requested</div>
      <ul class="cap-list">
        {#each pendingInstall.capabilities as cap}
          <li>
            <code>{cap}</code>
            {#if cap.includes("network")}
              <span>— outbound network access; deny-by-default in hardened host.</span>
            {:else if cap.includes("host_ai")}
              <span>— host AI bridge; inference calls are audited.</span>
            {:else if cap.includes("filesystem")}
              <span>— scoped filesystem paths only.</span>
            {:else if cap.includes("entropy")}
              <span>— kernel-injected entropy, not OS rand.</span>
            {:else}
              <span>— capability grant from plugin manifest.</span>
            {/if}
          </li>
        {/each}
      </ul>
    </section>

    <section aria-label="Manifest">
      <div class="label">Manifest</div>
      <p class="manifest-id">
        <code>{pendingInstall.pluginId}@{pendingInstall.version}</code> · publisher
        {pendingInstall.publisher}
      </p>
      {#if !isSigned(pendingInstall)}
        <p class="warn">Unsigned / non-approved manifest — install blocked in production policy.</p>
      {/if}
    </section>

    <label class="accept">
      <input type="checkbox" bind:checked={disclosureAccepted} />
      I understand the capability grants above and accept install stub (no live host).
    </label>

    <footer class="disclosure__actions">
      <button type="button" class="btn btn--ghost" onclick={cancelInstall}>Cancel</button>
      <button
        type="button"
        class="btn btn--primary"
        disabled={!disclosureAccepted || !isSigned(pendingInstall)}
        onclick={confirmInstall}
      >
        Install (stub)
      </button>
    </footer>
  </dialog>
{/if}

{#if installNotice}
  <div class="notice" role="status">{installNotice}</div>
{/if}

<style>
  .rail-label {
    margin: 0 0 var(--space-3);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .canvas {
    padding: var(--space-5) var(--space-6);
  }
  .page-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: var(--space-5);
    margin-bottom: var(--space-5);
  }
  .page-head h1 {
    margin: 0;
    font-size: var(--font-size-2xl);
    font-weight: var(--font-weight-semibold);
    letter-spacing: -0.03em;
  }
  .page-head p {
    max-width: 40rem;
    margin: var(--space-2) 0 0;
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    justify-content: flex-end;
  }
  .catalog {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .plugin {
    padding: var(--space-5) 0;
    border-top: 1px solid var(--color-border-default);
  }
  .plugin__head {
    display: flex;
    justify-content: space-between;
    gap: var(--space-4);
    align-items: flex-start;
    margin-bottom: var(--space-3);
  }
  .plugin__name {
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-semibold);
  }
  .plugin__meta {
    margin-top: 2px;
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
  }
  .plugin__status {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    justify-content: flex-end;
  }
  .plugin__caps {
    display: grid;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }
  .caps-label {
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }
  .cap-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .text-action {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-brand-primary);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
  }
  .overlay {
    position: fixed;
    inset: 0;
    background: color-mix(in oklab, var(--color-surface-0) 40%, transparent);
    z-index: 40;
  }
  .disclosure {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 50;
    width: min(480px, calc(100vw - var(--space-6)));
    margin: 0;
    padding: var(--space-5);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-lg);
    background: var(--color-surface-1);
    color: var(--color-text-primary);
  }
  .disclosure header h2 {
    margin: 0;
    font-size: var(--font-size-xl);
  }
  .disclosure header p {
    margin: var(--space-2) 0 0;
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
  }
  .label {
    margin: var(--space-4) 0 var(--space-2);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .cap-list {
    margin: 0;
    padding-left: 1.1rem;
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
    line-height: var(--line-height-relaxed);
  }
  .cap-list code {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--color-text-primary);
  }
  .manifest-id {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--color-text-secondary);
  }
  .manifest-id code {
    font-family: var(--font-mono);
  }
  .warn {
    margin: var(--space-2) 0 0;
    color: var(--color-warning);
    font-size: var(--font-size-sm);
  }
  .accept {
    display: flex;
    gap: var(--space-3);
    align-items: flex-start;
    margin-top: var(--space-4);
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
    cursor: pointer;
  }
  .disclosure__actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-3);
    margin-top: var(--space-5);
  }
  .btn {
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
  }
  .btn--ghost {
    border: 1px solid var(--color-border-default);
    background: transparent;
    color: var(--color-text-secondary);
  }
  .btn--primary {
    border: 0;
    background: var(--color-brand-primary);
    color: var(--color-surface-0);
  }
  .btn--primary:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .notice {
    position: fixed;
    right: var(--space-5);
    bottom: var(--space-5);
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface-1);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    z-index: 60;
  }
</style>
