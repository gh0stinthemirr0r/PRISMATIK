<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EvidenceChip } from "@prismatik/ui";
  import { market } from "$lib/prismatik/market.svelte";
  import {
    INTEGRATIONS,
    INTEGRATION_STORAGE_KEY,
    type IntegrationDefinition,
    type IntegrationStatus,
    type SavedIntegration,
  } from "$lib/integrations";

  type TestResult = {
    providerId: string;
    status: "connected";
    message: string;
    evidence: string;
  };
  type RuntimeStatus = { providerId: string; active: boolean };
  type ProviderPolicy = { providerId:string; enabled:boolean; termsOwner:string; reviewReference:string; validUntil:string; retainNormalizedPayload:boolean; version:number; recordedAt:string };

  /// Providers the running build can actually connect, per the backend.
  ///
  /// The catalog carries its own readiness flag, but that flag drifted from
  /// reality and left users walking a setup wizard that dead-ended. The
  /// backend is authoritative; the flag is only the fallback for a browser
  /// preview where no backend exists.
  let liveAdapters = $state<Set<string> | null>(null);

  let saved = $state<Record<string, SavedIntegration>>({});
  let selected = $state<IntegrationDefinition | null>(null);
  let setupStep = $state(0);
  let credentials = $state<Record<string, string>>({});
  let mode = $state("paper");
  let testing = $state(false);
  let setupError = $state<string | null>(null);
  let search = $state("");
  let category = $state("All");
  let region = $state("All regions");
  let sortBy = $state("readiness");
  let runtimeActive = $state<Set<string>>(new Set());
  let providerPolicies = $state<Record<string, ProviderPolicy>>({});
  let persistenceEnabled = $state(false);
  let termsOwner = $state("");
  let reviewReference = $state("");
  let validUntil = $state(new Date(Date.now() + 365 * 86400000).toISOString());
  let retainNormalizedPayload = $state(false);

  const categories = ["All", "Market data", "Macro & filings", "Broker", "Information"];
  const regions = ["All regions", "US", "Global", "Europe", "Asia-Pacific", "Emerging markets"];
  const filtered = $derived(
    INTEGRATIONS.filter((provider) => {
      const matchesCategory = category === "All" || provider.category === category;
      const matchesRegion = region === "All regions" || provider.region === region;
      const haystack = `${provider.name} ${provider.description} ${provider.capabilities.join(" ")} ${(provider.assetClasses ?? []).join(" ")} ${provider.region ?? ""}`.toLowerCase();
      return matchesCategory && matchesRegion && haystack.includes(search.trim().toLowerCase());
    }).toSorted((a, b) => {
      if (sortBy === "name") return a.name.localeCompare(b.name);
      if (sortBy === "region") return (a.region ?? "Global").localeCompare(b.region ?? "Global") || a.name.localeCompare(b.name);
      if (sortBy === "category") return a.category.localeCompare(b.category) || a.name.localeCompare(b.name);
      const rank = { "live-adapter": 0, "adapter-next": 1, research: 2, "execution-gated": 3 };
      return rank[a.readiness] - rank[b.readiness] || a.name.localeCompare(b.name);
    }),
  );
  const connectedCount = $derived(
    runtimeActive.size,
  );
  const isConnectable = (provider: IntegrationDefinition) =>
    liveAdapters ? liveAdapters.has(provider.id) : provider.readiness === "live-adapter";
  const liveAdapterCount = $derived(INTEGRATIONS.filter(isConnectable).length);
  const completion = $derived(Math.round((connectedCount / Math.max(1, liveAdapterCount)) * 100));

  const readinessLabel = (provider: IntegrationDefinition) => ({
    "live-adapter": "Live adapter",
    "adapter-next": "Adapter next",
    research: "Research",
    "execution-gated": "Execution gated",
  })[provider.readiness];

  const bestFor = (provider: IntegrationDefinition) => {
    if (provider.venueClass === "crypto") return "Crypto market structure, venue comparison, order books, funding, and derivatives research.";
    if (provider.venueClass === "prediction") return "Event probabilities, cross-venue pricing, liquidity, and resolution-aware research.";
    if (provider.venueClass === "macro") return "Point-in-time macro regimes, rates, sovereign, positioning, and release-aware analysis.";
    if (provider.venueClass === "filings") return "Issuer fundamentals, regulatory filings, ownership, and corporate-event evidence.";
    if (provider.category === "Broker") return "Account reconciliation, paper trading, positions, and explicitly gated order execution.";
    if (provider.venueClass === "information") return "News and event discovery, attention tracking, source topology, and cited research.";
    return "Pricing, reference data, historical research, market breadth, and multi-asset analytics.";
  };

  const accessMethod = (provider: IntegrationDefinition) => {
    if (isConnectable(provider)) return "Native PRISMATIK adapter · governed REST";
    if (provider.category === "Broker") return "Paper/account API first · execution separately gated";
    if (provider.cadence.toLowerCase().includes("stream")) return "REST snapshot + WebSocket stream candidate";
    if (!provider.credentialFields.length) return "Public read-only API candidate";
    return "Credentialed REST API candidate";
  };

  const onboardingSteps = (provider: IntegrationDefinition) => [
    `Review ${provider.name}'s official API documentation and applicable data terms.`,
    provider.credentialFields.length ? `Create least-privilege credentials for ${provider.credentialFields.map((field) => field.label).join(" and ")}.` : "Confirm the public endpoint and fair-use/rate-limit policy for your intended scope.",
    "Register the entitlement and source-retention policy in PRISMATIK; do not enable redistribution by default.",
    isConnectable(provider) ? "Run the governed native connection check below and retain its evidence record." : "This build has no adapter for it yet. Catalog presence alone cannot activate data, and connecting is not possible until one ships.",
  ];

  function isTauriRuntime(): boolean {
    return "__TAURI_INTERNALS__" in window || "__TAURI__" in window;
  }

  function readSaved() {
    try {
      const stored = JSON.parse(localStorage.getItem(INTEGRATION_STORAGE_KEY) ?? "{}") as Record<
        string,
        Omit<SavedIntegration, "status"> & { status: string; connectedAt?: string }
      >;
      saved = Object.fromEntries(
        Object.entries(stored).map(([id, profile]) => [
          id,
          {
            ...profile,
            validatedAt: profile.validatedAt ?? profile.connectedAt,
            status: profile.status === "connected" ? "validated" : profile.status,
          },
        ]),
      ) as Record<string, SavedIntegration>;
    } catch {
      saved = {};
    }
  }

  async function reconcileRuntime() {
    if (!isTauriRuntime()) return;
    try {
      const rows = await invoke<RuntimeStatus[]>("integration_runtime_status");
      runtimeActive = new Set(rows.filter((row) => row.active).map((row) => row.providerId));
    } catch {
      runtimeActive = new Set();
    }
  }

  function persist(next: Record<string, SavedIntegration>) {
    saved = next;
    localStorage.setItem(INTEGRATION_STORAGE_KEY, JSON.stringify(next));
  }

  function openSetup(provider: IntegrationDefinition) {
    selected = provider;
    setupStep = 0;
    credentials = {};
    mode = saved[provider.id]?.mode ?? provider.modes?.[0]?.id ?? "public";
    setupError = null;
    const policy = providerPolicies[provider.id];
    persistenceEnabled = policy?.enabled ?? false;
    termsOwner = policy?.termsOwner ?? "";
    reviewReference = policy?.reviewReference ?? "";
    validUntil = policy?.validUntil ?? new Date(Date.now() + 365 * 86400000).toISOString();
    retainNormalizedPayload = policy?.retainNormalizedPayload ?? false;
  }

  function closeSetup() {
    selected = null;
    credentials = {};
    setupError = null;
  }

  function fieldsComplete(provider: IntegrationDefinition): boolean {
    const credentialsComplete = provider.credentialFields.every(
      (field) => !field.required || Boolean(credentials[field.id]?.trim()),
    );
    return credentialsComplete && (!persistenceEnabled || Boolean(termsOwner.trim() && reviewReference.trim() && validUntil.trim()));
  }

  const supportsPersistence = (provider: IntegrationDefinition) => ["coingecko", "finnhub", "fred", "sec-edgar"].includes(provider.id);

  async function saveProviderPolicy(provider: IntegrationDefinition) {
    if (!supportsPersistence(provider)) return;
    const policy = await invoke<ProviderPolicy>("upsert_provider_policy", { policy: {
      providerId: provider.id, enabled: persistenceEnabled, termsOwner, reviewReference,
      validUntil, retainNormalizedPayload,
    }});
    providerPolicies = { ...providerPolicies, [provider.id]: policy };
  }

  async function testConnection() {
    if (!selected || !fieldsComplete(selected)) return;
    testing = true;
    setupError = null;
    setupStep = 2;
    const started = performance.now();
    try {
      if (!isTauriRuntime()) {
        throw new Error("Connection validation runs inside the PRISMATIK desktop app.");
      }
      const result = await invoke<TestResult>("test_integration", {
        providerId: selected.id,
        credentials: { ...credentials, mode },
      });
      let policyWarning: string | null = null;
      try { await saveProviderPolicy(selected); }
      catch (cause) { policyWarning = cause instanceof Error ? cause.message : String(cause); }
      const latency = Math.max(1, Math.round(performance.now() - started));
      persist({
        ...saved,
        [selected.id]: {
          providerId: selected.id,
          status: "validated",
          mode,
          validatedAt: new Date().toISOString(),
          evidence: `${result.evidence} · ${latency} ms`,
          message: result.message,
        },
      });
      credentials = {};
      runtimeActive = new Set([...runtimeActive, selected.id]);
      if (selected.id === "coingecko" || selected.id === "finnhub") {
        // The store owns the poll and reports its own failure state.
        await market.refresh();
      }
      setupStep = 3;
      setupError = policyWarning ? `Connection active; durable evidence policy was not recorded: ${policyWarning}` : null;
    } catch (cause) {
      setupError = cause instanceof Error ? cause.message : String(cause);
      persist({
        ...saved,
        [selected.id]: {
          providerId: selected.id,
          status: "error",
          mode,
          message: setupError,
        },
      });
      setupStep = 1;
    } finally {
      testing = false;
    }
  }

  async function disconnect(provider: IntegrationDefinition) {
    if (isTauriRuntime()) await invoke("disconnect_integration", { providerId: provider.id });
    runtimeActive = new Set([...runtimeActive].filter((id) => id !== provider.id));
    const next = { ...saved };
    delete next[provider.id];
    persist(next);
  }

  function statusOf(provider: IntegrationDefinition): IntegrationStatus {
    if (runtimeActive.has(provider.id)) return "validated";
    return saved[provider.id]?.status === "error" ? "error" : "not-connected";
  }

  onMount(async () => {
    readSaved();
    void reconcileRuntime();
    if (isTauriRuntime()) {
      try {
        liveAdapters = new Set(await invoke<string[]>("list_live_adapters"));
      } catch {
        // Leave null so the catalog flag is used. Guessing "nothing is
        // connectable" would hide working providers.
        liveAdapters = null;
      }
      try {
        const rows = await invoke<ProviderPolicy[]>("list_provider_policies");
        providerPolicies = Object.fromEntries(rows.map((row) => [row.providerId, row]));
      } catch { providerPolicies = {}; }
    }
  });
</script>

<svelte:head><title>Integrations · PRISMATIK</title></svelte:head>

<div class="canvas">
    <header class="page-head">
      <div class="fabric-title">
        <div class="fabric-sigil" aria-hidden="true"><i></i><i></i><i></i></div>
        <div>
          <div class="eyebrow">PROVIDER FABRIC / GLOBAL INPUT CONTROL</div>
          <h1>Integration fabric</h1>
          <p>Route external market evidence through native contracts, source policy, budget control, normalization, and durable provenance.</p>
        </div>
      </div>
      <div class="readiness">
        <div class="readiness__top"><span>Evidence plane readiness</span><strong>{completion}%</strong></div>
        <div class="track"><i style={`width:${completion}%`}></i></div>
        <small>{connectedCount ? `${connectedCount} adapter probe validated` : `${liveAdapterCount} governed live adapter available now`}</small>
      </div>
    </header>

    <section class="fabric-telemetry" aria-label="Integration telemetry">
      <article><span>CATALOG</span><strong>{INTEGRATIONS.length}</strong><small>Global providers indexed</small></article>
      <article><span>NATIVE PATHS</span><strong>{liveAdapterCount}</strong><small>Governed validation adapters</small></article>
      <article><span>ACTIVE</span><strong>{connectedCount}</strong><small>Current native session</small></article>
      <article><span>INPUT MODE</span><strong>{connectedCount ? "HYBRID" : "SIM"}</strong><small>{connectedCount ? "Live coverage + explicit simulation fallback" : "No live market API session"}</small></article>
    </section>

    <section class="control-row" aria-label="Integration filters">
      <label class="search">
        <span aria-hidden="true"></span>
        <input bind:value={search} aria-label="Search integrations" placeholder="Search providers and capabilities" />
      </label>
      <div class="categories">
        {#each categories as item}
          <button type="button" class:active={category === item} onclick={() => (category = item)}>{item}</button>
        {/each}
      </div>
      <select class="region-filter" bind:value={region} aria-label="Filter integration region">
        {#each regions as item}<option>{item}</option>{/each}
      </select>
      <select class="region-filter" bind:value={sortBy} aria-label="Sort integrations">
        <option value="readiness">Sort: readiness</option>
        <option value="name">Sort: name</option>
        <option value="region">Sort: region</option>
        <option value="category">Sort: category</option>
      </select>
    </section>

    <section class="provider-grid" aria-label="Available integrations">
      {#each filtered as provider, index}
        {@const profile = saved[provider.id]}
        {@const status = statusOf(provider)}
        <article class:connected={status === "validated"} style={`--provider:${provider.accent}`}>
          <span class="provider-index">{String(index + 1).padStart(2, "0")}</span>
          <div class="provider-head">
            <div class="provider-mark" aria-hidden="true">{provider.name.slice(0, 2).toUpperCase()}</div>
            <div>
              <span class="category">{provider.category}</span>
              <h2>{provider.name}</h2>
            </div>
            <span class="status" class:ok={status === "validated"} class:error={status === "error"}>
              <i></i>{status === "validated" ? "Validated" : status === "error" ? "Attention" : readinessLabel(provider)}
            </span>
          </div>
          <p>{provider.description}</p>
          <div class="use-case"><span>BEST FOR</span><p>{bestFor(provider)}</p><code>{accessMethod(provider)}</code></div>
          <div class="capabilities">
            {#each provider.capabilities.slice(0, 3) as capability}<span>{capability}</span>{/each}
          </div>
          {#if provider.region || provider.assetClasses?.length}
            <div class="coverage"><span>{provider.region ?? "Global"}</span><code>{provider.assetClasses?.join(" · ") ?? provider.venueClass}</code></div>
          {/if}
          {#if profile?.evidence}
            <div class="evidence"><span>Last validation</span><code>{profile.evidence}</code></div>
          {:else}
            <div class="evidence"><span>Cadence</span><code>{provider.cadence}</code></div>
          {/if}
          {#if providerPolicies[provider.id]?.enabled}
            <div class="policy-chip">DURABLE EVIDENCE · POLICY V{providerPolicies[provider.id].version}</div>
          {/if}
          <div class="provider-actions">
            <button class="primary" type="button" onclick={() => openSetup(provider)}>
              {status === "validated" ? "Revalidate" : isConnectable(provider) ? "Validate" : "View setup"}
            </button>
            {#if status === "validated"}
              <button class="text" type="button" onclick={() => disconnect(provider)}>Clear validation</button>
            {/if}
          </div>
        </article>
      {/each}
    </section>

    <aside class="security-note">
      <span class="security-icon" aria-hidden="true">◇</span>
      <div>
        <strong>Desktop-owned validation</strong>
        <p>Live probes use a Layer-2 provider adapter, an application-owned transport, and a BudgetGovernor permit. Credentials stay only in native process memory, are never written to localStorage, and must be reconnected after restart.</p>
      </div>
      <EvidenceChip status="confirmed" label="fail closed" />
    </aside>
</div>

{#if selected}
  <button class="scrim" type="button" aria-label="Close integration setup" onclick={closeSetup}></button>
  <dialog class="setup" open aria-labelledby="setup-title">
    <header>
      <div class="provider-mark" style={`--provider:${selected.accent}`} aria-hidden="true">{selected.name.slice(0, 2).toUpperCase()}</div>
      <div>
        <div class="eyebrow">PROVIDER ONBOARDING</div>
        <h2 id="setup-title">{selected.name}</h2>
      </div>
      <button class="close-dialog" type="button" aria-label="Close" onclick={closeSetup}>×</button>
    </header>

    <div class="stepper" aria-label="Setup progress">
      {#each ["Understand", "Configure", "Verify", "Ready"] as label, index}
        <div class:active={index === setupStep} class:done={index < setupStep}>
          <i>{index < setupStep ? "✓" : index + 1}</i><span>{label}</span>
        </div>
      {/each}
    </div>

    {#if setupStep === 0}
      <div class="setup-body">
        <h3>What this connection enables</h3>
        <p>{selected.description}</p>
        <ul>{#each selected.capabilities as capability}<li>{capability}</li>{/each}</ul>
        <div class="disclosure"><strong>Best used for</strong><p>{bestFor(selected)}</p><code>{accessMethod(selected)}</code></div>
        <div class="onboarding"><strong>Onboarding checklist</strong><ol>{#each onboardingSteps(selected) as step}<li>{step}</li>{/each}</ol></div>
        <div class="disclosure"><strong>Data handling</strong><p>{selected.privacy}</p></div>
        <a href={selected.docsUrl} target="_blank" rel="noreferrer">Open official provider documentation ↗</a>
      </div>
      <footer>
        <button class="ghost" type="button" onclick={closeSetup}>Cancel</button>
        {#if isConnectable(selected)}
          <button class="primary" type="button" onclick={() => (setupStep = 1)}>Configure connection</button>
        {:else}
          <button class="primary" type="button" disabled>Native adapter required</button>
        {/if}
      </footer>
    {:else if setupStep === 1}
      <div class="setup-body">
        {#if selected.modes}
          <fieldset>
            <legend>Connection environment</legend>
            <div class="mode-grid">
              {#each selected.modes as option}
                <button type="button" class:active={mode === option.id} onclick={() => (mode = option.id)}>
                  <strong>{option.label}</strong><span>{option.help}</span>
                </button>
              {/each}
            </div>
          </fieldset>
        {/if}
        {#if selected.credentialFields.length}
          <div class="fields">
            {#each selected.credentialFields as field}
              <label>
                <span>{field.label}</span>
                <input
                  type={field.type}
                  value={credentials[field.id] ?? ""}
                  autocomplete="off"
                  oninput={(event) => (credentials[field.id] = event.currentTarget.value)}
                />
                <small>{field.help}</small>
              </label>
            {/each}
          </div>
        {:else}
          <div class="zero-key">
            <i aria-hidden="true">✓</i>
            <div><strong>No credential required</strong><p>This validation uses a public, read-only provider endpoint.</p></div>
          </div>
        {/if}
        {#if supportsPersistence(selected)}
          <fieldset class="retention">
            <legend>Durable evidence policy</legend>
            <label class="toggle"><input type="checkbox" bind:checked={persistenceEnabled}/><span><strong>Persist normalized observations</strong><small>Disabled by default. Requires a named terms review; redistribution remains prohibited.</small></span></label>
            {#if persistenceEnabled}
              <div class="fields policy-fields">
                <label><span>Terms owner</span><input bind:value={termsOwner} placeholder="Named reviewer or team"/><small>Accountable owner for provider terms and entitlement.</small></label>
                <label><span>Review reference</span><input bind:value={reviewReference} placeholder="Ticket, memo, or approval reference"/><small>Auditable record supporting lawful retention.</small></label>
                <label><span>Policy valid until (RFC3339)</span><input bind:value={validUntil}/><small>Persistence fails closed after this timestamp.</small></label>
                <label class="toggle"><input type="checkbox" bind:checked={retainNormalizedPayload}/><span><strong>Retain normalized record bodies</strong><small>Otherwise PRISMATIK keeps provenance metadata and hashes only.</small></span></label>
              </div>
            {/if}
          </fieldset>
        {/if}
        {#if setupError}<div class="setup-error" role="alert">{setupError}</div>{/if}
      </div>
      <footer>
        <button class="ghost" type="button" onclick={() => (setupStep = 0)}>Back</button>
        <button class="primary" type="button" disabled={!fieldsComplete(selected) || testing} onclick={testConnection}>
          {testing ? "Checking…" : "Test connection"}
        </button>
      </footer>
    {:else if setupStep === 2}
      <div class="setup-body verifying">
        <div class="radar" aria-hidden="true"><i></i><i></i><i></i></div>
        <h3>Verifying {selected.name}</h3>
        <p>Checking endpoint reachability, authentication, and the expected response contract.</p>
      </div>
    {:else}
      <div class="setup-body ready">
        <div class="ready-mark" aria-hidden="true">✓</div>
        <h3>{selected.name} was validated</h3>
        <p>{saved[selected.id]?.message}</p>
        <div class="disclosure"><strong>Evidence</strong><code>{saved[selected.id]?.evidence}</code></div>
        {#if providerPolicies[selected.id]?.enabled}<div class="disclosure"><strong>Durable evidence</strong><p>Policy v{providerPolicies[selected.id].version} active until {providerPolicies[selected.id].validUntil}. Normalized payload retention: {providerPolicies[selected.id].retainNormalizedPayload ? "enabled" : "metadata and hashes only"}.</p></div>{/if}
        {#if setupError}<div class="setup-error" role="alert">{setupError}</div>{/if}
        <p class="secret-note">Secret fields were cleared from the form. The credential now exists only in native process memory and powers governed terminal refreshes until disconnect or application exit.</p>
      </div>
      <footer>
        <button class="primary" type="button" onclick={closeSetup}>Done</button>
      </footer>
    {/if}
  </dialog>
{/if}

<style>
  .eyebrow{margin:0 0 var(--space-3);color:var(--color-brand-primary);font:500 .58rem var(--font-mono);letter-spacing:.12em;text-transform:uppercase}
  .canvas{--color-brand-primary:var(--p-accent);--color-brand-accent:var(--p-accent2);--color-text-primary:var(--p-text);--color-text-secondary:var(--p-dim);--color-text-tertiary:var(--p-dim);--color-border-default:var(--p-border);--color-border-strong:var(--p-dim);--color-surface-1:var(--p-panel-fill);--color-surface-2:var(--p-surface2);--color-surface-3:var(--p-border);box-sizing:border-box;width:min(100%,1560px);height:100%;margin:0 auto;padding:34px clamp(24px,3vw,48px) 72px;overflow:auto;color:var(--p-text)}
  .page-head{display:grid;grid-template-columns:minmax(0,1fr) 300px;gap:var(--space-7);align-items:end;margin-bottom:var(--space-6)}
  h1{margin:0;font-size:clamp(2.5rem,4vw,4.4rem);letter-spacing:-.065em;line-height:.92}
  .page-head p{max-width:760px;margin:var(--space-3) 0 0;color:var(--color-text-secondary);line-height:1.6}
  .readiness{display:grid;gap:8px;padding:16px;border:1px solid var(--color-border-default);border-radius:var(--radius-lg);background:var(--color-surface-1)}
  .readiness__top{display:flex;justify-content:space-between;font-size:var(--font-size-xs);text-transform:uppercase;color:var(--color-text-tertiary)}
  .readiness__top strong{color:var(--color-brand-primary);font-family:var(--font-mono)}
  .track{height:4px;overflow:hidden;border-radius:99px;background:var(--color-surface-3)}.track i{display:block;height:100%;background:linear-gradient(90deg,var(--color-brand-primary),var(--color-brand-accent));box-shadow:0 0 12px var(--color-brand-primary)}
  .readiness small{color:var(--color-text-secondary)}
  .control-row{display:flex;gap:var(--space-4);align-items:center;margin-bottom:var(--space-6)}
  .search{display:flex;min-width:280px;flex:1;align-items:center;gap:10px;padding:0 14px;border:1px solid var(--color-border-default);border-radius:999px;background:rgba(255,255,255,.025)}
  .search span{width:10px;height:10px;border:1px solid var(--color-text-tertiary);border-radius:50%}
  .search input{width:100%;padding:11px 0;border:0;outline:0;background:transparent;color:var(--color-text-primary)}
  .categories{display:flex;gap:5px}.categories button{padding:8px 11px;border:1px solid transparent;border-radius:999px;background:transparent;color:var(--color-text-tertiary);cursor:pointer;font-size:var(--font-size-xs)}.categories button.active{border-color:rgba(0,240,255,.25);background:rgba(0,240,255,.07);color:var(--color-text-primary)}
  .region-filter{padding:8px 12px;border:1px solid var(--color-border-default);border-radius:999px;background:var(--color-surface-1);color:var(--color-text-secondary);font-size:var(--font-size-xs)}
  .provider-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:var(--space-4)}
  .provider-grid>article{position:relative;display:grid;gap:var(--space-4);min-height:300px;padding:22px;overflow:hidden;border:1px solid var(--color-border-default);border-radius:var(--radius-lg);background:radial-gradient(circle at 0 0,color-mix(in srgb,var(--provider) 11%,transparent),transparent 50%),var(--color-surface-1);transition:transform .18s ease,border-color .18s ease}
  .provider-grid>article:hover{transform:translateY(-2px);border-color:color-mix(in srgb,var(--provider) 38%,var(--color-border-default))}
  .provider-grid>article.connected{box-shadow:inset 0 2px 0 color-mix(in srgb,var(--provider) 58%,transparent)}
  .provider-head{display:grid;grid-template-columns:auto 1fr auto;gap:12px;align-items:center}
  .provider-mark{display:grid;width:42px;height:42px;place-items:center;border:1px solid color-mix(in srgb,var(--provider) 48%,transparent);border-radius:12px;background:color-mix(in srgb,var(--provider) 12%,var(--color-surface-2));color:var(--provider);font:600 .7rem var(--font-mono);box-shadow:0 0 22px color-mix(in srgb,var(--provider) 12%,transparent)}
  .category{color:var(--color-text-tertiary);font-size:.55rem;letter-spacing:.08em;text-transform:uppercase}
  h2{margin:3px 0 0;font-size:var(--font-size-lg)}
  .status{display:flex;align-items:center;gap:6px;color:var(--color-text-tertiary);font:500 .55rem var(--font-mono);text-transform:uppercase}.status i{width:5px;height:5px;border-radius:50%;background:var(--color-text-tertiary)}.status.ok{color:var(--color-success)}.status.ok i{background:var(--color-success);box-shadow:0 0 8px var(--color-success)}.status.error{color:var(--color-warning)}
  .provider-grid>article>p{margin:0;color:var(--color-text-secondary);font-size:var(--font-size-sm);line-height:1.55}
  .use-case{display:grid;gap:5px;padding:10px;border-left:2px solid color-mix(in srgb,var(--provider) 45%,transparent);background:rgba(255,255,255,.018)}.use-case span{color:var(--color-text-tertiary);font-size:.5rem;letter-spacing:.08em}.use-case p{margin:0;color:var(--color-text-secondary);font-size:.66rem;line-height:1.45}.use-case code{color:var(--provider);font-size:.55rem}
  .capabilities{display:flex;flex-wrap:wrap;gap:6px}.capabilities span{padding:4px 7px;border:1px solid var(--color-border-default);border-radius:999px;color:var(--color-text-tertiary);font-size:.62rem}
  .coverage{display:flex;align-items:center;justify-content:space-between;gap:10px;color:var(--color-text-tertiary);font-size:.55rem;text-transform:uppercase}.coverage code{overflow:hidden;color:var(--color-text-secondary);font-size:.55rem;text-overflow:ellipsis;white-space:nowrap;text-transform:none}
  .evidence{display:grid;gap:5px;margin-top:auto}.evidence span{color:var(--color-text-tertiary);font-size:.55rem;text-transform:uppercase}.evidence code{overflow:hidden;color:var(--color-text-secondary);font-size:.61rem;text-overflow:ellipsis;white-space:nowrap}
  .policy-chip{width:fit-content;padding:4px 7px;border:1px solid color-mix(in srgb,var(--p-up) 35%,var(--p-border));border-radius:5px;color:var(--p-up);font:700 .5rem var(--font-mono);letter-spacing:.08em}
  .provider-actions{display:flex;align-items:center;gap:12px}.primary,.ghost,.text{border:0;cursor:pointer}.primary{padding:9px 15px;border-radius:var(--radius-md);background:linear-gradient(135deg,var(--color-brand-primary),#72e6ff);color:#031014;font-weight:650}.text{padding:0;background:transparent;color:var(--color-text-tertiary);font-size:var(--font-size-xs)}
  .security-note{display:grid;grid-template-columns:auto 1fr auto;gap:14px;align-items:center;margin-top:var(--space-6);padding:18px;border:1px solid rgba(0,240,255,.14);border-radius:var(--radius-lg);background:rgba(0,240,255,.025)}.security-icon{display:grid;width:34px;height:34px;place-items:center;border:1px solid rgba(0,240,255,.2);border-radius:10px;color:var(--color-brand-primary)}.security-note p{margin:4px 0 0;color:var(--color-text-secondary);font-size:var(--font-size-sm)}
  .scrim{position:fixed;z-index:1500;inset:38px 0 0;border:0;background:rgba(2,5,10,.72);backdrop-filter:blur(10px)}
  .setup{position:fixed;z-index:1510;top:calc(50% + 19px);left:50%;display:grid;grid-template-rows:auto auto minmax(0,1fr) auto;width:min(680px,calc(100vw - 48px));max-height:calc(100vh - 90px);overflow:auto;border:1px solid var(--color-border-strong);border-radius:18px;background:radial-gradient(circle at 0 0,rgba(0,240,255,.06),transparent 40%),#0a0f19;box-shadow:0 34px 100px rgba(0,0,0,.65);transform:translate(-50%,-50%)}
  .setup>header{display:grid;grid-template-columns:auto 1fr auto;gap:14px;align-items:center;padding:22px 24px;border-bottom:1px solid var(--color-border-default)}.setup h2{font-size:var(--font-size-xl)}.close-dialog{width:32px;height:32px;border:1px solid var(--color-border-default);border-radius:50%;background:transparent;color:var(--color-text-secondary);cursor:pointer;font-size:1.2rem}
  .stepper{display:grid;grid-template-columns:repeat(4,1fr);padding:16px 24px;border-bottom:1px solid var(--color-border-default)}.stepper div{display:flex;align-items:center;gap:7px;color:var(--color-text-tertiary);font-size:.62rem}.stepper i{display:grid;width:20px;height:20px;place-items:center;border:1px solid var(--color-border-default);border-radius:50%;font-style:normal;font-family:var(--font-mono)}.stepper .active{color:var(--color-text-primary)}.stepper .active i,.stepper .done i{border-color:var(--color-brand-primary);background:rgba(0,240,255,.1);color:var(--color-brand-primary)}
  .setup-body{display:grid;gap:18px;padding:28px 30px}.setup-body h3{margin:0;font-size:var(--font-size-xl)}.setup-body>p{margin:0;color:var(--color-text-secondary);line-height:1.6}.setup-body ul{display:grid;grid-template-columns:1fr 1fr;gap:8px;margin:0;padding:0;list-style:none}.setup-body li::before{margin-right:8px;color:var(--color-brand-primary);content:"◇"}.setup-body a{width:fit-content;color:var(--color-brand-primary);font-size:var(--font-size-sm)}
  .disclosure{display:grid;gap:6px;padding:14px;border:1px solid var(--color-border-default);border-radius:var(--radius-md);background:rgba(255,255,255,.025)}.disclosure p,.disclosure code{margin:0;color:var(--color-text-secondary);font-size:var(--font-size-sm);line-height:1.5}
  .onboarding{display:grid;gap:10px;padding:14px;border:1px solid var(--color-border-default);border-radius:var(--radius-md)}.onboarding ol{display:grid!important;grid-template-columns:1fr!important;gap:8px!important;margin:0!important;padding-left:20px!important;list-style:decimal!important}.onboarding li{color:var(--color-text-secondary);font-size:.72rem;line-height:1.45}.onboarding li::before{content:none!important}
  fieldset{padding:0;border:0}legend{margin-bottom:10px;color:var(--color-text-tertiary);font-size:var(--font-size-xs);text-transform:uppercase}.mode-grid{display:grid;grid-template-columns:1fr 1fr;gap:10px}.mode-grid button{display:grid;gap:4px;padding:13px;border:1px solid var(--color-border-default);border-radius:var(--radius-md);background:var(--color-surface-1);color:var(--color-text-primary);text-align:left;cursor:pointer}.mode-grid button.active{border-color:var(--color-brand-primary);box-shadow:inset 0 0 0 1px rgba(0,240,255,.18)}.mode-grid span{color:var(--color-text-tertiary);font-size:.68rem}
  .fields{display:grid;gap:16px}.fields label{display:grid;gap:7px}.fields label>span{font-size:var(--font-size-sm);font-weight:600}.fields input{padding:11px 12px;border:1px solid var(--color-border-default);border-radius:var(--radius-md);outline:none;background:#070b12;color:var(--color-text-primary);font-family:var(--font-mono)}.fields input:focus{border-color:var(--color-brand-primary);box-shadow:0 0 0 3px rgba(0,240,255,.07)}.fields small{color:var(--color-text-tertiary)}
  .retention{display:grid;gap:12px;padding:15px;border:1px solid color-mix(in srgb,var(--p-accent) 24%,var(--p-border));border-radius:9px;background:color-mix(in srgb,var(--p-accent) 3%,transparent)}.toggle{display:flex!important;align-items:flex-start;gap:10px}.toggle input{width:16px;height:16px;margin-top:2px;accent-color:var(--p-accent)}.toggle span{display:grid;gap:4px}.toggle small{font-weight:400}.policy-fields{padding-top:4px}
  .zero-key{display:flex;gap:12px;align-items:center;padding:18px;border:1px solid rgba(72,220,153,.25);border-radius:var(--radius-lg);background:rgba(72,220,153,.04)}.zero-key>i,.ready-mark{display:grid;width:36px;height:36px;place-items:center;border-radius:50%;background:rgba(72,220,153,.13);color:var(--color-success);font-style:normal}.zero-key p{margin:3px 0 0;color:var(--color-text-secondary);font-size:var(--font-size-sm)}
  .setup-error{padding:12px;border:1px solid rgba(255,184,77,.3);border-radius:var(--radius-md);background:rgba(255,184,77,.05);color:var(--color-warning);font-size:var(--font-size-sm)}
  .setup>footer{display:flex;justify-content:flex-end;gap:10px;padding:18px 24px;border-top:1px solid var(--color-border-default)}.ghost{padding:9px 15px;border:1px solid var(--color-border-default);border-radius:var(--radius-md);background:transparent;color:var(--color-text-secondary)}.primary:disabled{cursor:not-allowed;opacity:.4}
  .verifying,.ready{place-items:center;min-height:300px;text-align:center}.radar{position:relative;width:90px;height:90px;border:1px solid rgba(0,240,255,.3);border-radius:50%;background:repeating-radial-gradient(circle,transparent 0 14px,rgba(0,240,255,.08) 15px 16px)}.radar::after{position:absolute;inset:0;border-radius:50%;background:conic-gradient(from 0deg,rgba(0,240,255,.32),transparent 28%);content:"";animation:scan 1.2s linear infinite}.radar i{position:absolute;width:5px;height:5px;border-radius:50%;background:var(--color-brand-primary);box-shadow:0 0 9px var(--color-brand-primary)}.radar i:nth-child(1){top:20px;left:54px}.radar i:nth-child(2){top:59px;left:25px}.radar i:nth-child(3){top:44px;left:67px}
  .ready-mark{width:64px;height:64px;font-size:1.6rem;box-shadow:0 0 40px rgba(72,220,153,.14)}.secret-note{font-size:var(--font-size-xs)!important}
  @keyframes scan{to{transform:rotate(360deg)}}
  @media(max-width:1320px){.page-head{grid-template-columns:1fr}.readiness{width:min(100%,440px)}.provider-grid{grid-template-columns:repeat(2,minmax(0,1fr))}.control-row{align-items:stretch;flex-direction:column}.categories{overflow:auto}}
  @media(max-width:720px){.page-head,.provider-grid{grid-template-columns:1fr}.setup-body ul{grid-template-columns:1fr}.stepper span{display:none}.categories{display:none}}

  /* Terminal-fabric redesign */
  .canvas{position:relative;isolation:isolate;background:radial-gradient(circle at 78% 2%,color-mix(in srgb,var(--p-accent2) 9%,transparent),transparent 28rem),radial-gradient(circle at 18% 8%,color-mix(in srgb,var(--p-accent) 7%,transparent),transparent 32rem)}
  .canvas::before{position:fixed;z-index:-1;inset:82px 0 0 58px;background-image:linear-gradient(var(--p-grid) 1px,transparent 1px),linear-gradient(90deg,var(--p-grid) 1px,transparent 1px);background-size:32px 32px;content:"";opacity:.18;mask-image:linear-gradient(to bottom,black,transparent 70%);pointer-events:none}
  .page-head{grid-template-columns:minmax(0,1fr) minmax(260px,340px);align-items:center;margin-bottom:14px;padding:2px 0 22px;border-bottom:1px solid var(--p-border)}
  .fabric-title{display:flex;align-items:center;gap:18px}.fabric-sigil{position:relative;flex:0 0 62px;width:62px;height:62px;border:1px solid color-mix(in srgb,var(--p-accent) 28%,var(--p-border));border-radius:18px;background:conic-gradient(from 210deg,color-mix(in srgb,var(--p-accent) 18%,transparent),transparent 32%,color-mix(in srgb,var(--p-accent2) 18%,transparent),transparent 70%);box-shadow:inset 0 0 26px color-mix(in srgb,var(--p-accent) 7%,transparent),0 0 32px color-mix(in srgb,var(--p-accent) 8%,transparent)}.fabric-sigil::before,.fabric-sigil::after{position:absolute;inset:12px;border:1px solid color-mix(in srgb,var(--p-accent) 35%,transparent);border-radius:50%;content:""}.fabric-sigil::after{inset:23px;background:var(--p-accent);box-shadow:0 0 16px var(--p-accent)}.fabric-sigil i{position:absolute;width:5px;height:5px;border-radius:50%;background:var(--p-accent2);box-shadow:0 0 8px var(--p-accent2)}.fabric-sigil i:nth-child(1){top:8px;left:27px}.fabric-sigil i:nth-child(2){right:8px;bottom:14px}.fabric-sigil i:nth-child(3){bottom:10px;left:12px}
  .page-head h1{font:700 clamp(2rem,3.2vw,3.7rem) var(--font-ui);letter-spacing:-.065em}.page-head p{color:var(--p-dim);font-size:.76rem}.readiness{border-color:color-mix(in srgb,var(--p-accent) 20%,var(--p-border));background:var(--p-panel-fill);backdrop-filter:blur(var(--p-glass-blur)) saturate(150%);box-shadow:inset 0 1px rgba(255,255,255,.05)}
  .fabric-telemetry{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:7px;margin-bottom:12px}.fabric-telemetry article{display:grid;gap:3px;padding:11px 13px;border:1px solid var(--p-border);border-radius:8px;background:var(--p-panel-fill);backdrop-filter:blur(var(--p-glass-blur))}.fabric-telemetry span{color:var(--p-dim);font:700 .48rem var(--font-mono);letter-spacing:.16em}.fabric-telemetry strong{font:700 1rem var(--font-mono)}.fabric-telemetry small{color:var(--p-dim);font-size:.55rem}
  .control-row{position:sticky;z-index:5;top:-34px;gap:7px;margin-bottom:12px;padding:9px;border:1px solid var(--p-border);border-radius:10px;background:color-mix(in srgb,var(--p-bg2) 78%,transparent);backdrop-filter:blur(22px) saturate(160%)}.search,.region-filter{border-radius:6px;background:var(--p-panel-fill)}.categories button{border-radius:5px}.categories button.active{border-color:color-mix(in srgb,var(--p-accent) 35%,transparent);background:color-mix(in srgb,var(--p-accent) 10%,transparent);color:var(--p-accent)}
  .provider-grid{grid-template-columns:repeat(auto-fill,minmax(310px,1fr));gap:9px}.provider-grid>article{min-height:330px;gap:12px;padding:17px;border-radius:10px;background:linear-gradient(145deg,color-mix(in srgb,var(--provider) 7%,transparent),transparent 43%),var(--p-panel-fill);backdrop-filter:blur(var(--p-glass-blur)) saturate(145%);box-shadow:inset 0 1px rgba(255,255,255,.04)}.provider-grid>article::after{position:absolute;right:-34px;bottom:-34px;width:110px;height:110px;border:1px solid color-mix(in srgb,var(--provider) 18%,transparent);border-radius:50%;content:"";box-shadow:0 0 34px color-mix(in srgb,var(--provider) 7%,transparent)}.provider-index{position:absolute;right:12px;bottom:8px;color:color-mix(in srgb,var(--provider) 35%,transparent);font:700 1.8rem var(--font-mono);letter-spacing:-.08em}.provider-head{grid-template-columns:auto 1fr auto}.provider-mark{border-radius:50%;background:color-mix(in srgb,var(--provider) 10%,var(--p-surface));box-shadow:inset 0 0 14px color-mix(in srgb,var(--provider) 8%,transparent),0 0 22px color-mix(in srgb,var(--provider) 10%,transparent)}.use-case{border:1px solid color-mix(in srgb,var(--provider) 16%,var(--p-border));border-left:2px solid var(--provider);border-radius:6px;background:color-mix(in srgb,var(--provider) 4%,transparent)}.capabilities span{background:color-mix(in srgb,var(--p-surface2) 70%,transparent)}.primary{border:1px solid color-mix(in srgb,var(--provider) 40%,transparent);border-radius:6px;background:color-mix(in srgb,var(--provider) 17%,var(--p-surface));color:var(--provider);font:700 .62rem var(--font-mono);letter-spacing:.06em;text-transform:uppercase}.security-note{border-color:color-mix(in srgb,var(--p-accent) 18%,var(--p-border));background:var(--p-panel-fill);backdrop-filter:blur(var(--p-glass-blur))}
  .setup{--color-brand-primary:var(--p-accent);--color-text-primary:var(--p-text);--color-text-secondary:var(--p-dim);--color-text-tertiary:var(--p-dim);--color-border-default:var(--p-border);--color-border-strong:var(--p-dim);--color-surface-1:var(--p-surface);--color-surface-2:var(--p-surface2);background:radial-gradient(circle at 0 0,color-mix(in srgb,var(--p-accent) 8%,transparent),transparent 40%),color-mix(in srgb,var(--p-bg2) 88%,transparent);color:var(--p-text);backdrop-filter:blur(30px) saturate(170%)}
  @media(max-width:1100px){.fabric-telemetry{grid-template-columns:1fr 1fr}.fabric-sigil{display:none}.page-head{grid-template-columns:1fr}.readiness{width:100%}.provider-grid{grid-template-columns:repeat(2,minmax(0,1fr))}}
  @media(max-width:760px){.fabric-telemetry,.provider-grid{grid-template-columns:1fr}.control-row{position:static}.canvas{padding:20px 12px 80px}.provider-grid>article{min-height:0}}
</style>
