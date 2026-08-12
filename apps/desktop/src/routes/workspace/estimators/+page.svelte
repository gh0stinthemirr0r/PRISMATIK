<script lang="ts">
  /**
   * Estimators — three independent forecasters on one question.
   *
   * Regime reasons from history, Tape from sequence, Crowd from population.
   * They live on one surface rather than three because the comparison is the
   * point: three estimators that agree for methodologically different reasons
   * say something none of them says alone, and two that split are reporting a
   * fact about the instrument rather than a fault in themselves.
   *
   * Nothing here blends them into a single number. A combined probability
   * would read as a fourth, better forecast while having no cohort, no
   * resolution and no measured skill — so each estimator is shown with its
   * own standing, and the reader does the weighting knowing which have
   * earned it.
   */
  import { onMount } from 'svelte';
  import { Loader2, Play, FileUp, Plug, Users, Activity, Scale } from 'lucide-svelte';
  import NoData from '$lib/prismatik/NoData.svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface TrackedInstrument {
    symbol: string;
    kind: string;
    providerId: string;
  }

  interface EstimatorView {
    name: string;
    providerId: string;
    cohortModel: string;
    direction: string;
    probabilityPpm: number;
    climatologyPpm: number;
    edgePpm: number;
    skillPpm: number | null;
    resolvedCount: number;
    standing: string;
    summary: string;
    generatedAt: string;
  }

  interface ConsensusView {
    target: string;
    horizonDays: number;
    estimators: EstimatorView[];
    reporting: number;
    distinctDirections: number;
    unanimous: boolean;
    verdict: string;
    provenReporting: number;
  }

  interface CrowdConfig {
    baseUrl: string;
    connected: boolean;
    version: string | null;
  }

  // Matches FORECAST_HORIZONS_DAYS in analytics.rs — a horizon the desk does
  // not otherwise forecast would have no Regime claim to compare against.
  const HORIZONS = [1, 5, 21];

  let instruments = $state<TrackedInstrument[]>([]);
  let symbol = $state('');
  let horizon = $state(5);

  /** Null until the tracked list has been read at all. */
  let instrumentsLoaded = $state(false);
  let instrumentsError = $state<string | null>(null);

  let consensus = $state<ConsensusView | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  let tapeBusy = $state(false);
  let tapeResult = $state<string | null>(null);
  let tapeError = $state<string | null>(null);

  let crowd = $state<CrowdConfig | null>(null);
  let crowdBusy = $state(false);
  let crowdResult = $state<string | null>(null);
  let crowdError = $state<string | null>(null);
  let scenarioId = $state('');

  const pct = (ppm: number) => `${(ppm / 10_000).toFixed(1)}%`;
  const signedPct = (ppm: number) =>
    `${ppm >= 0 ? '+' : ''}${(ppm / 10_000).toFixed(1)}`;

  async function loadConsensus() {
    if (!symbol) return;
    loading = true;
    error = null;
    try {
      consensus = await invoke<ConsensusView>('consensus_for', {
        target: symbol,
        horizonDays: horizon,
      });
    } catch (e) {
      error = String(e);
      consensus = null;
    } finally {
      loading = false;
    }
  }

  async function runTape() {
    if (!symbol) return;
    tapeBusy = true;
    tapeResult = null;
    tapeError = null;
    try {
      tapeResult = await invoke<string>('tape_file_forecast', {
        symbol,
        horizonDays: horizon,
      });
      await loadConsensus();
    } catch (e) {
      tapeError = String(e);
    } finally {
      tapeBusy = false;
    }
  }

  async function connectCrowd() {
    crowdBusy = true;
    crowdError = null;
    try {
      await invoke<string>('crowd_connect', { baseUrl: null });
      crowd = await invoke<CrowdConfig>('crowd_status');
    } catch (e) {
      crowdError = String(e);
    } finally {
      crowdBusy = false;
    }
  }

  async function fileCrowd() {
    if (!symbol || !scenarioId.trim()) return;
    crowdBusy = true;
    crowdResult = null;
    crowdError = null;
    try {
      crowdResult = await invoke<string>('crowd_file_forecast', {
        scenarioId: scenarioId.trim(),
        symbol,
        horizonDays: horizon,
      });
      await loadConsensus();
    } catch (e) {
      crowdError = String(e);
    } finally {
      crowdBusy = false;
    }
  }

  onMount(async () => {
    try {
      instruments = await invoke<TrackedInstrument[]>('get_tracked_instruments');
      instrumentsLoaded = true;
      if (instruments.length > 0) {
        symbol = instruments[0].symbol;
        await loadConsensus();
      }
    } catch (e) {
      // Failing to read the list is not the same as the list being empty.
      // Rendering "nothing tracked" here would report a backend failure as a
      // fact about the user's portfolio.
      instrumentsError = String(e);
    }
    try {
      crowd = await invoke<CrowdConfig>('crowd_status');
    } catch {
      // Crowd being unreachable is an expected state, not an error worth a
      // banner — the panel below reports it in place.
    }
  });
</script>

<svelte:head><title>Estimators · PRISMATIK</title></svelte:head>

<div class="estimators prismatik">
  <header>
    <h1>Estimators</h1>
    <p>
      Three forecasters reaching the same question by different routes. Where they agree, they
      agree for independent reasons; where they split, the split is the finding. None of them is
      blended into the others.
    </p>
  </header>

  {#if instrumentsError}
    <NoData
      title="COULD NOT READ THE TRACKED LIST"
      detail={instrumentsError}
      tone="warn"
    />
  {:else if !instrumentsLoaded}
    <NoData title="READING TRACKED INSTRUMENTS" detail="…" />
  {:else if instruments.length === 0}
    <NoData
      title="NOTHING TRACKED"
      detail="Estimators forecast instruments the desk follows. Track one and its claims appear here."
      action="Go to Markets"
      href="/workspace"
    />
  {:else}
    <section class="controls">
      <label>
        <span>Instrument</span>
        <select bind:value={symbol} onchange={loadConsensus}>
          {#each instruments as row (row.symbol)}
            <option value={row.symbol}>{row.symbol}</option>
          {/each}
        </select>
      </label>
      <label>
        <span>Horizon</span>
        <select bind:value={horizon} onchange={loadConsensus}>
          {#each HORIZONS as days (days)}
            <option value={days}>{days} day{days === 1 ? '' : 's'}</option>
          {/each}
        </select>
      </label>
      <button onclick={loadConsensus} disabled={loading}>
        {#if loading}<Loader2 size={13} class="spin" />{:else}<Scale size={13} />{/if}
        Refresh
      </button>
    </section>

    {#if error}
      <NoData title="COULD NOT READ CLAIMS" detail={error} tone="warn" />
    {:else if consensus}
      <section class="verdict" class:unanimous={consensus.unanimous}>
        <p>{consensus.verdict}</p>
        <span class="counts">
          {consensus.reporting} of 3 reporting · {consensus.provenReporting} with a positive
          measured record
        </span>
      </section>

      {#if consensus.estimators.length === 0}
        <NoData
          title="NO OPEN CLAIMS"
          detail="No estimator has filed an unresolved {horizon}-day claim on {symbol}. Run one below."
        />
      {:else}
        <section class="cards">
          {#each consensus.estimators as e (e.providerId)}
            <article class="card" class:up={e.direction === 'up'} class:down={e.direction === 'down'}>
              <header>
                <b>{e.name}</b>
                <span class="dir">{e.direction}</span>
              </header>
              <div class="numbers">
                <div>
                  <dt>Claims</dt>
                  <dd>{pct(e.probabilityPpm)}</dd>
                </div>
                <div>
                  <dt>Base rate</dt>
                  <dd>{pct(e.climatologyPpm)}</dd>
                </div>
                <div>
                  <dt>Edge</dt>
                  <dd class:positive={e.edgePpm > 0} class:negative={e.edgePpm < 0}>
                    {signedPct(e.edgePpm)}
                  </dd>
                </div>
              </div>
              <p class="standing" class:proven={e.skillPpm !== null && e.skillPpm > 0 && e.resolvedCount > 0}>
                {e.standing}
              </p>
              <p class="summary">{e.summary}</p>
              <span class="cohort">{e.cohortModel}</span>
            </article>
          {/each}
        </section>
      {/if}
    {/if}

    <section class="runners">
      <article class="runner">
        <h2><Activity size={14} /> Tape — sequence</h2>
        <p class="what">
          Continues {symbol}'s bars with the Kronos foundation model and files the fraction of
          sampled paths finishing outside the flat band. Needs the sidecar on port 8766.
        </p>
        <button onclick={runTape} disabled={tapeBusy || !symbol}>
          {#if tapeBusy}<Loader2 size={13} class="spin" />{:else}<Play size={13} />{/if}
          Run and file {horizon}d claim
        </button>
        {#if tapeResult}<p class="result">{tapeResult}</p>{/if}
        {#if tapeError}<p class="failure">{tapeError}</p>{/if}
      </article>

      <article class="runner">
        <h2><Users size={14} /> Crowd — population</h2>
        <p class="what">
          Files a completed swarm simulation as a claim, using the share of agents holding the
          plurality view. Needs the service on port 5001 and a finished scenario.
        </p>
        {#if crowd?.connected}
          <p class="status ok">connected to {crowd.baseUrl}</p>
          <label class="scenario">
            <span>Scenario id</span>
            <input bind:value={scenarioId} placeholder="from a completed simulation" />
          </label>
          <button onclick={fileCrowd} disabled={crowdBusy || !scenarioId.trim()}>
            {#if crowdBusy}<Loader2 size={13} class="spin" />{:else}<FileUp size={13} />{/if}
            File {horizon}d claim
          </button>
        {:else}
          <p class="status">not connected</p>
          <button onclick={connectCrowd} disabled={crowdBusy}>
            {#if crowdBusy}<Loader2 size={13} class="spin" />{:else}<Plug size={13} />{/if}
            Connect
          </button>
        {/if}
        {#if crowdResult}<p class="result">{crowdResult}</p>{/if}
        {#if crowdError}<p class="failure">{crowdError}</p>{/if}
      </article>
    </section>
  {/if}
</div>

<style>
  .estimators {
    max-width: 74rem;
    margin: 0 auto;
    padding: 2rem 1.75rem 4rem;
    color: var(--p-text);
  }
  h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 300;
    letter-spacing: 0.2em;
  }
  header p {
    margin: 0.4rem 0 0;
    color: var(--p-dim);
    font-size: 0.76rem;
    line-height: 1.6;
    max-width: 54rem;
  }

  .controls {
    display: flex;
    align-items: flex-end;
    gap: 1rem;
    margin: 1.75rem 0 1.25rem;
    flex-wrap: wrap;
  }
  .controls label {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .controls span,
  .scenario span {
    color: var(--p-dim);
    font: 600 0.58rem var(--font-mono);
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }
  select,
  input {
    background: var(--p-surface);
    border: 1px solid var(--p-border);
    border-radius: 4px;
    color: var(--p-text);
    font: 0.76rem var(--font-mono);
    padding: 0.4rem 0.55rem;
    min-width: 9rem;
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: var(--p-surface2);
    border: 1px solid var(--p-border);
    border-radius: 4px;
    color: var(--p-text);
    font: 600 0.72rem var(--font-ui);
    padding: 0.45rem 0.8rem;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  :global(.estimators .spin) {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .verdict {
    border: 1px solid var(--p-border);
    border-left-width: 3px;
    border-radius: var(--p-card-radius);
    background: var(--p-panel-fill);
    padding: 0.9rem 1.1rem;
    margin-bottom: 1.25rem;
  }
  /* Unanimity earns an accent; a split does not get a warning colour, because
     disagreement is a finding rather than a fault. */
  .verdict.unanimous {
    border-left-color: var(--p-accent);
  }
  .verdict p {
    margin: 0;
    font-size: 0.82rem;
  }
  .counts {
    display: block;
    margin-top: 0.35rem;
    color: var(--p-dim);
    font: 0.64rem var(--font-mono);
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(19rem, 1fr));
    gap: 1rem;
  }
  .card {
    border: 1px solid var(--p-border);
    border-top-width: 2px;
    border-radius: var(--p-card-radius);
    background: var(--p-panel-fill);
    padding: 0.9rem 1rem;
  }
  .card.up {
    border-top-color: var(--p-up);
  }
  .card.down {
    border-top-color: var(--p-down);
  }
  .card > header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .card b {
    font-size: 0.86rem;
    letter-spacing: 0.06em;
  }
  .dir {
    font: 600 0.62rem var(--font-mono);
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--p-dim);
  }
  .numbers {
    display: flex;
    gap: 1.25rem;
    margin: 0.7rem 0;
  }
  dt {
    color: var(--p-dim);
    font: 600 0.55rem var(--font-mono);
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }
  dd {
    margin: 0.15rem 0 0;
    font: 0.9rem var(--font-mono);
  }
  dd.positive {
    color: var(--p-up);
  }
  dd.negative {
    color: var(--p-down);
  }
  .standing {
    margin: 0 0 0.5rem;
    color: #ffb84d;
    font-size: 0.66rem;
    line-height: 1.5;
  }
  /* Only a scored, positive cohort loses the amber. */
  .standing.proven {
    color: var(--p-up);
  }
  .summary {
    margin: 0;
    color: var(--p-dim);
    font-size: 0.7rem;
    line-height: 1.6;
  }
  .cohort {
    display: block;
    margin-top: 0.6rem;
    color: var(--p-dim);
    font: 0.58rem var(--font-mono);
    opacity: 0.7;
  }

  .runners {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(21rem, 1fr));
    gap: 1rem;
    margin-top: 1.75rem;
  }
  .runner {
    border: 1px solid var(--p-border);
    border-radius: var(--p-card-radius);
    background: var(--p-panel-fill);
    padding: 1rem;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0 0 0.5rem;
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.1em;
  }
  .what {
    margin: 0 0 0.8rem;
    color: var(--p-dim);
    font-size: 0.7rem;
    line-height: 1.6;
  }
  .status {
    margin: 0 0 0.6rem;
    color: var(--p-dim);
    font: 0.64rem var(--font-mono);
  }
  .status.ok {
    color: var(--p-up);
  }
  .scenario {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin-bottom: 0.7rem;
  }
  .result {
    margin: 0.7rem 0 0;
    color: var(--p-up);
    font-size: 0.7rem;
    line-height: 1.5;
  }
  .failure {
    margin: 0.7rem 0 0;
    color: #ffb84d;
    font-size: 0.7rem;
    line-height: 1.5;
  }
</style>
