<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import UnavailableExperience from '$lib/UnavailableExperience.svelte';
  import Visualizations from '$lib/prismatik/Visualizations.svelte';

  interface CalibrationHealth {
    cohortId: string;
    providerId: string;
    model: string;
    target: string;
    horizonMinutes: number;
    sampleCount: number;
    overallBrierPpm: number | null;
    overallEcePpm: number | null;
    baselineBrierPpm: number | null;
    climatologyBrierPpm: number | null;
    skillPpm: number | null;
    skillSampleCount: number;
    recentBrierPpm: number | null;
    driftPpm: number | null;
    state: string;
    executionEligible: boolean;
  }

  let cohorts = $state<CalibrationHealth[]>([]);
  let loading = $state(true);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      cohorts = await invoke<CalibrationHealth[]>('forecast_calibration_health');
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

  const hasData = $derived(cohorts.length > 0);
  const ppm = (v: number | null) => (v === null ? '—' : (v / 1_000_000).toFixed(4));
  const stateColor = (s: string) =>
    s === 'stable' ? '#34d399' : s === 'regressed' ? '#f87171' : '#94a3b8';

  /**
   * Skill is the only column that answers "is this cohort worth running".
   * Positive beat the base rate, zero matched it, negative lost to it.
   */
  function skillLabel(c: CalibrationHealth): string {
    if (c.skillPpm === null) return '—';
    const pct = (c.skillPpm / 10_000).toFixed(1);
    return `${c.skillPpm >= 0 ? '+' : ''}${pct}%`;
  }

  function skillColor(skillPpm: number | null): string {
    if (skillPpm === null) return 'var(--p-text-dim)';
    if (skillPpm > 20_000) return '#34d399';
    if (skillPpm < -20_000) return '#f87171';
    return 'var(--p-text-dim)';
  }

  function skillTitle(c: CalibrationHealth): string {
    if (c.skillPpm === null) {
      return 'This forecaster did not record a base rate, so its skill cannot be measured. Only the statistical forecaster files one.';
    }
    return `Brier ${ppm(c.overallBrierPpm)} against a climatology Brier of ${ppm(c.climatologyBrierPpm)}, over ${c.skillSampleCount} resolved forecasts. Zero means no better than always predicting the base rate.`;
  }
</script>

<svelte:head><title>Calibration · PRISMATIK</title></svelte:head>

<div class="pk-page">
  <div class="pk-page-head">
    <h1>Forecast calibration</h1>
    <p>
      Realized calibration of every forecast cohort (provider × model × target × horizon).
      Brier scores and expected calibration error compare recent vs. baseline performance.
      Calibration is the difference between a model that claims 80% confidence and one that is right 80% of the time.
    </p>
  </div>

  {#if error}
    <div class="pk-error">{error}</div>
  {/if}

  {#if loading}
    <div class="pk-empty">Loading calibration…</div>
  {:else if hasData}
    <section class="pk-panel">
      <div class="pk-panel-head">
        <span>Skill atlas — cohort skill against baseline</span>
      </div>
      <div class="pk-viz-host"><Visualizations only={['atlas']} /></div>
    </section>

    <section class="pk-panel">
      <div class="pk-panel-head">
        <span>Calibration cohorts ({cohorts.length})</span>
        <button class="pk-refresh" onclick={load}>Refresh</button>
      </div>
      <table class="pk-table">
        <thead>
          <tr>
            <th>Cohort</th><th>Provider</th><th>Model</th><th>Target</th>
            <th>Horizon</th><th>Samples</th><th>Brier</th><th>ECE</th>
            <th title="Brier skill against the base rate. 0 means the forecaster added nothing.">Skill</th>
            <th>Drift</th><th>State</th><th>Exec</th>
          </tr>
        </thead>
        <tbody>
          {#each cohorts as c (c.cohortId)}
            <tr>
              <td class="pk-mono">{c.cohortId}</td>
              <td class="pk-mono">{c.providerId}</td>
              <td class="pk-mono">{c.model}</td>
              <td class="pk-mono">{c.target}</td>
              <td class="pk-mono">{c.horizonMinutes}m</td>
              <td class="pk-mono">{c.sampleCount}</td>
              <td class="pk-mono">{ppm(c.overallBrierPpm)}</td>
              <td class="pk-mono">{ppm(c.overallEcePpm)}</td>
              <td class="pk-mono" style="color: {skillColor(c.skillPpm)}" title={skillTitle(c)}>
                {skillLabel(c)}
              </td>
              <td class="pk-mono" style="color: {c.driftPpm !== null && c.driftPpm > 50_000 ? '#f87171' : 'var(--p-text-dim)'}">{ppm(c.driftPpm)}</td>
              <td class="pk-mono" style="color: {stateColor(c.state)}">{c.state}</td>
              <td class="pk-mono pk-dim">{c.executionEligible ? '✓' : '✗'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>
    <p class="pk-note">
      Execution eligibility is permanently false. Calibration does not make a forecast tradeable — it measures honesty.
      Brier and ECE say whether a forecaster's probabilities are truthful; <strong>Skill</strong> says whether they carry
      information. A forecaster can be perfectly calibrated and still score zero skill by restating the base rate.
      Generate and resolve forecasts from <a href="/workspace/models">Models</a> to populate this table.
    </p>
  {:else}
    <UnavailableExperience
      active="calibration"
      title="Calibration"
      description="Calibration evidence appears only after real forecasts and resolved outcomes have been recorded. No fixture data is shown."
    />
  {/if}
</div>

<style>
  .pk-page { padding: 24px; max-width: 1200px; }
  .pk-page-head h1 { font-size: 1.4rem; font-weight: 600; margin: 0 0 6px; color: var(--p-text); }
  .pk-page-head p { font-size: 0.8125rem; color: var(--p-text-dim); margin: 0 0 20px; max-width: 65ch; line-height: 1.5; }
  .pk-error { background: rgba(255,80,80,0.12); border: 1px solid rgba(255,80,80,0.3); color: #ff9090; padding: 10px 14px; border-radius: 4px; font-size: 0.8125rem; margin-bottom: 16px; }
  .pk-empty { padding: 40px; text-align: center; color: var(--p-text-dim); }
  .pk-panel { background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 8px; margin-bottom: 16px; }
  .pk-panel-head { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--p-border); font-family: var(--p-mono); font-size: 0.625rem; letter-spacing: 0.14em; text-transform: uppercase; color: var(--p-text-dim); }
  .pk-refresh { background: none; border: 1px solid var(--p-border); color: var(--p-text-dim); padding: 4px 10px; border-radius: 3px; cursor: pointer; font-size: 0.625rem; }
  .pk-refresh:hover { color: var(--p-text); border-color: var(--p-accent); }
  .pk-table { width: 100%; border-collapse: collapse; }
  .pk-table th { text-align: left; padding: 10px 16px; font-size: 0.625rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--p-text-dim); border-bottom: 1px solid var(--p-border); }
  .pk-table td { padding: 8px 16px; font-size: 0.75rem; border-bottom: 1px solid var(--p-border); }
  .pk-viz-host { height: 320px; }
  .pk-mono { font-family: var(--p-mono); }
  .pk-dim { color: var(--p-text-dim); }
  .pk-note { font-size: 0.75rem; color: var(--p-text-dim); line-height: 1.5; }
  .pk-note a { color: var(--p-accent); }
</style>
