<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import UnavailableExperience from '$lib/UnavailableExperience.svelte';

  interface IntelligenceCapability {
    id: string;
    engine: string;
    observationCount: number;
    status: string;
  }

  let capabilities = $state<IntelligenceCapability[]>([]);
  let loading = $state(true);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      capabilities = await invoke<IntelligenceCapability[]>('intelligence_capabilities');
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    load();
    const interval = setInterval(load, 30_000);
    return () => clearInterval(interval);
  });

  // The analog-search capability specifically.
  const analogCapability = $derived(
    capabilities.find((c) => c.id.includes('analog') || c.id.includes('pit-query') || c.id.includes('resolution')),
  );
  const totalObservations = $derived(capabilities.reduce((sum, c) => sum + c.observationCount, 0));
  const hasCorpus = $derived(totalObservations > 0);
</script>

<svelte:head><title>Analog search · PRISMATIK</title></svelte:head>

<div class="pk-page">
  <div class="pk-page-head">
    <h1>Analog search</h1>
    <p>
      Historical analog discovery and point-in-time replay require a populated, survivorship-safe
      observation corpus. The intelligence engine below tracks which capabilities have real
      observations vs. which are awaiting ingestion.
    </p>
  </div>

  {#if error}
    <div class="pk-error">{error}</div>
  {/if}

  {#if loading}
    <div class="pk-empty">Loading intelligence capabilities…</div>
  {:else}
    <div class="pk-corpus-status" class:populated={hasCorpus} class:empty={!hasCorpus}>
      <span class="pk-corpus-dot"></span>
      <span>Observation corpus: <strong>{totalObservations > 0 ? `${totalObservations} records` : 'empty'}</strong></span>
      <span class="pk-dim" style="margin-left:auto">
        {hasCorpus ? 'Point-in-time analogs available' : 'Connect providers and ingest history to enable analog search'}
      </span>
    </div>

    {#if hasCorpus && analogCapability}
      <section class="pk-panel">
        <div class="pk-panel-head"><span>Analog search engine</span></div>
        <div class="pk-engine-body">
          <p>The observation corpus has {totalObservations} governed records. Analog search uses point-in-time-safe
          feature vectors to find the nearest historical matches to the current market state, with leave-N-out
          sensitivity disclosure.</p>
          <p class="pk-dim">Full analog query UI requires the embedding index and historical feature store to be
          populated. The capability is active — run a query from the Strategy or Backtest workspace once a
          strategy is bound to a universe.</p>
        </div>
      </section>
    {/if}

    <section class="pk-panel">
      <div class="pk-panel-head"><span>Intelligence capability manifest ({capabilities.length})</span></div>
      <table class="pk-table">
        <thead>
          <tr><th>Capability</th><th>Engine</th><th>Observations</th><th>Status</th></tr>
        </thead>
        <tbody>
          {#each capabilities as cap (cap.id)}
            <tr>
              <td class="pk-mono">{cap.id}</td>
              <td class="pk-mono pk-dim">{cap.engine}</td>
              <td class="pk-mono">{cap.observationCount}</td>
              <td class="pk-mono" style="color: {cap.status === 'observed' ? '#34d399' : '#94a3b8'}">
                {cap.status === 'observed' ? '● observed' : '○ awaiting'}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>

    {#if !hasCorpus}
      <UnavailableExperience
        active="analogs"
        title="Analog search"
        description="Historical analogs require a populated, point-in-time-safe observation corpus. The capability manifest above shows which engines are active but awaiting observations."
      />
    {/if}
  {/if}
</div>

<style>
  .pk-page { padding: 24px; max-width: 1000px; }
  .pk-page-head h1 { font-size: 1.4rem; font-weight: 600; margin: 0 0 6px; color: var(--p-text); }
  .pk-page-head p { font-size: 0.8125rem; color: var(--p-text-dim); margin: 0 0 20px; max-width: 65ch; line-height: 1.5; }
  .pk-error { background: rgba(255,80,80,0.12); border: 1px solid rgba(255,80,80,0.3); color: #ff9090; padding: 10px 14px; border-radius: 4px; font-size: 0.8125rem; margin-bottom: 16px; }
  .pk-empty { padding: 40px; text-align: center; color: var(--p-text-dim); }
  .pk-corpus-status { display: flex; align-items: center; gap: 10px; padding: 10px 16px; border-radius: 6px; margin-bottom: 16px; font-size: 0.75rem; font-family: var(--p-mono); }
  .pk-corpus-status.populated { background: rgba(52,211,153,0.08); border: 1px solid rgba(52,211,153,0.2); color: #34d399; }
  .pk-corpus-status.empty { background: rgba(148,163,184,0.06); border: 1px solid var(--p-border); color: var(--p-text-dim); }
  .pk-corpus-dot { width: 7px; height: 7px; border-radius: 50%; background: currentColor; }
  .pk-panel { background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 8px; margin-bottom: 16px; }
  .pk-panel-head { padding: 12px 16px; border-bottom: 1px solid var(--p-border); font-family: var(--p-mono); font-size: 0.625rem; letter-spacing: 0.14em; text-transform: uppercase; color: var(--p-text-dim); }
  .pk-engine-body { padding: 16px; font-size: 0.8125rem; line-height: 1.6; color: var(--p-text-dim); }
  .pk-engine-body p { margin: 0 0 10px; }
  .pk-table { width: 100%; border-collapse: collapse; }
  .pk-table th { text-align: left; padding: 10px 16px; font-size: 0.625rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--p-text-dim); border-bottom: 1px solid var(--p-border); }
  .pk-table td { padding: 8px 16px; font-size: 0.8125rem; border-bottom: 1px solid var(--p-border); }
  .pk-mono { font-family: var(--p-mono); }
  .pk-dim { color: var(--p-text-dim); }
</style>
