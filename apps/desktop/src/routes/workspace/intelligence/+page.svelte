<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type NativeCapability = { id: string; engine: string; observationCount: number; status: string };

  let query = $state("");
  let category = $state("all");
  let native = $state<Record<string, NativeCapability>>({});
  let backendError = $state<string | null>(null);

  onMount(async () => {
    try {
      const rows = await invoke<NativeCapability[]>("intelligence_capabilities");
      native = Object.fromEntries(rows.map((row) => [row.id, row]));
    } catch (error) {
      backendError = error instanceof Error ? error.message : String(error);
    }
  });

  const capabilities = [
    { id: "resolution", category: "prediction", name: "Resolution intelligence", detail: "Rule snapshots, source comparison, precedent evidence, dispute and semantic-risk scoring." },
    { id: "logical-arbitrage", category: "prediction", name: "Logical arbitrage", detail: "Mutual-exclusion, exhaustive-outcome, implication and subset consistency after fees." },
    { id: "calibration", category: "forecast", name: "Calibration laboratory", detail: "Brier reliability, cohort calibration and sample-size-aware participant skill." },
    { id: "l2-reconstruction", category: "microstructure", name: "L2 reconstruction", detail: "Sequence gaps, crossed books, deterministic depth and executable impact." },
    { id: "event-discovery", category: "events", name: "Event discovery", detail: "New listings, rule changes, probability movement, liquidity acceleration and lifecycle changes." },
    { id: "feed-attention", category: "events", name: "Feed attention fabric", detail: "Lawful RSS/Atom novelty, source breadth, diffusion, velocity and silence features with point-in-time citations." },
    { id: "transcripts", category: "events", name: "Transcript intelligence", detail: "Speaker-aware literal mentions, citations and expected-topic silence detection." },
    { id: "derivatives", category: "crypto", name: "Derivatives state", detail: "Basis, funding inputs, open-interest change and liquidation imbalance." },
    { id: "public-address", category: "crypto", name: "Public-address behavior", detail: "Opaque-address turnover and concentration with minimum-history safeguards." },
    { id: "continuous-reconciliation", category: "operations", name: "Continuous reconciliation", detail: "Snapshot differences, benign-resolution policy, quarantine and account health." },
    { id: "strategy-authoring", category: "research", name: "Contract-first authoring", detail: "Get contract, validate capabilities and save an unsigned strategy draft." },
    { id: "typed-ui", category: "research", name: "Typed terminal intents", detail: "Evidence-bound native panes; no model-generated markup or scripts." },
    { id: "pit-query", category: "data", name: "Point-in-time query plane", detail: "Read-only plans, AS OF cutoffs, scan budgets, pinned inputs and export policy." },
    { id: "macro-series", category: "macro", name: "Macro series evidence", detail: "Vintage-aware FRED observations with explicit source policy, availability timestamps, and durable provenance." },
    { id: "filings", category: "filings", name: "Regulatory filing evidence", detail: "Accession-level SEC submissions with official source links, normalized records, and governed retention." },
  ];

  let filtered = $derived(capabilities.filter((item) =>
    (category === "all" || item.category === category) &&
    `${item.name} ${item.detail}`.toLowerCase().includes(query.trim().toLowerCase())
  ));
</script>

<svelte:head><title>Intelligence fabric · PRISMATIK</title></svelte:head>

<div class="terminal">
  <main>
    <header>
      <div>
        <p class="eyebrow">NATIVE ANALYTICS FABRIC</p>
        <h1>Intelligence</h1>
        <p class="lede">Every engine is observation-driven. Empty panes remain empty until a governed provider contributes real, timestamped evidence.</p>
      </div>
      <span class="mode">NO MOCK DATA</span>
    </header>

    <section class="controls" aria-label="Capability filters">
      <label><span>Search</span><input bind:value={query} placeholder="Resolution, depth, transcript…" /></label>
      <label><span>Domain</span><select bind:value={category}><option value="all">All domains</option>{#each [...new Set(capabilities.map((item) => item.category))] as item}<option value={item}>{item}</option>{/each}</select></label>
    </section>

    <section class="grid" aria-live="polite">
      {#if backendError}<div class="backend-error">Native capability check failed: {backendError}</div>{/if}
      {#each filtered as capability}
        <article>
          <div class="article-head"><span>{capability.category}</span><i class:observed={native[capability.id]?.status === "observed"} title={native[capability.id]?.status === "observed" ? "Governed observations available" : "Awaiting governed observations"}></i></div>
          <h2>{capability.name}</h2>
          <p>{capability.detail}</p>
          <footer><strong>{native[capability.id]?.engine ?? "NATIVE CHECK"}</strong><span>{native[capability.id] ? `${native[capability.id].observationCount} observations` : "Checking backend…"}</span></footer>
        </article>
      {:else}
        <div class="empty">No capability matches this filter.</div>
      {/each}
    </section>
  </main>
</div>

<style>
  .terminal{height:100%;color:var(--p-text);overflow:auto}
  main{min-width:0;padding:clamp(18px,3vw,42px);overflow:auto}
  header{display:flex;align-items:flex-start;justify-content:space-between;gap:24px;padding-bottom:24px;border-bottom:1px solid var(--p-border)}
  .eyebrow,.mode,.article-head,footer{font-family:var(--font-mono);font-size:.62rem;letter-spacing:.12em;text-transform:uppercase}
  .eyebrow{margin:0 0 8px;color:var(--p-accent)} h1{margin:0;font-family:var(--font-ui);font-size:clamp(1.8rem,4vw,3.4rem)}
  .lede{max-width:760px;margin:12px 0 0;color:var(--p-dim);line-height:1.6}.mode{padding:8px 10px;border:1px solid color-mix(in srgb,var(--p-accent) 30%,transparent);border-radius:999px;color:var(--p-accent);white-space:nowrap}
  .controls{display:grid;grid-template-columns:minmax(220px,1fr) minmax(150px,240px);gap:12px;margin:22px 0}label{display:grid;gap:6px;color:var(--p-dim);font:600 .62rem var(--font-mono);letter-spacing:.1em;text-transform:uppercase}
  input,select{width:100%;min-width:0;padding:11px 12px;border:1px solid var(--p-border);border-radius:8px;background:var(--p-panel-fill);color:var(--p-text);font:inherit;letter-spacing:normal;text-transform:none}
  .grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,280px),1fr));gap:12px}article{display:flex;min-height:190px;flex-direction:column;padding:18px;border:1px solid var(--p-border);border-radius:10px;background:var(--p-panel-fill);box-shadow:inset 0 1px rgba(255,255,255,.025);backdrop-filter:blur(var(--p-glass-blur))}
  .backend-error{grid-column:1/-1;padding:12px;border:1px solid rgba(248,113,113,.35);color:#f87171;font:600 .7rem var(--font-mono)}
  .article-head{display:flex;justify-content:space-between;color:var(--p-dim)}i{width:7px;height:7px;border-radius:50%;background:#f59e0b;box-shadow:0 0 9px rgba(245,158,11,.4)}i.observed{background:var(--p-up);box-shadow:0 0 11px color-mix(in srgb,var(--p-up) 70%,transparent)}h2{margin:22px 0 8px;font-size:1rem}article p{margin:0;color:var(--p-dim);font-size:.82rem;line-height:1.55}footer{display:flex;justify-content:space-between;gap:12px;margin-top:auto;padding-top:18px;color:var(--p-dim)}footer strong{color:var(--p-up)}.empty{grid-column:1/-1;padding:40px;border:1px dashed var(--p-border);color:var(--p-dim);text-align:center}
  @media(max-width:820px){header{flex-direction:column}.controls{grid-template-columns:1fr}}
</style>
