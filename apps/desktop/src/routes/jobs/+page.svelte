<script lang="ts">
  import { onMount } from 'svelte';
  import {
    jobs,
    jobsLoading,
    jobsError,
    selectedJobId,
    syncJobs,
    initializeAPI,
  } from '$lib/stores';
  import { searchJobs, submitBacktest, submitBacktestBatch, getJob } from '$lib/api';
  import type { SubmitJobRequest } from '$lib/api';

  let searchTerm = '';
  let statusFilter = 'all';
  let selectedJob: any = null;
  let showSubmitForm = false;
  let formData: Partial<SubmitJobRequest> = {};
  let submitting = false;
  let submitError = '';
  let filteredJobs: any[] = [];

  onMount(async () => {
    const token = localStorage.getItem('api_token');
    if (token) {
      initializeAPI(token);
    }
    await syncJobs();
  });

  $: {
    let results = $jobs;
    
    if (searchTerm) {
      results = results.filter(j => 
        j.id.includes(searchTerm) || j.symbol.includes(searchTerm)
      );
    }
    
    if (statusFilter !== 'all') {
      results = results.filter(j => j.status === statusFilter);
    }
    
    filteredJobs = results;
  }

  async function handleSubmitJob() {
    if (!formData.symbol || !formData.strategy || !formData.initial_capital) {
      submitError = 'Please fill in all required fields';
      return;
    }

    submitting = true;
    submitError = '';
    
    try {
      const response = await submitBacktest(formData as SubmitJobRequest);
      showSubmitForm = false;
      formData = {};
      await syncJobs();
    } catch (err) {
      submitError = err instanceof Error ? err.message : 'Failed to submit job';
    } finally {
      submitting = false;
    }
  }

  async function handleSelectJob(job: any) {
    selectedJobId.set(job.id);
    try {
      selectedJob = await getJob(job.id);
    } catch (err) {
      console.error('Failed to load job details', err);
    }
  }
</script>

<svelte:head>
  <title>Job Management - PRISMATIK</title>
</svelte:head>

<div class="jobs-page">
  <div class="jobs-header">
    <div>
      <h1>Job Management</h1>
      <p class="subtitle">Submit and monitor backtests and walkforwards</p>
    </div>
    <button class="btn-primary" on:click={() => (showSubmitForm = !showSubmitForm)}>
      {showSubmitForm ? 'Cancel' : '+ New Job'}
    </button>
  </div>

  {#if showSubmitForm}
    <div class="submit-form">
      <h2>Submit New Backtest</h2>
      {#if submitError}
        <div class="error">{submitError}</div>
      {/if}
      
      <div class="form-grid">
        <div class="form-group">
          <label for="symbol">Symbol *</label>
          <input
            id="symbol"
            type="text"
            bind:value={formData.symbol}
            placeholder="e.g., BTC/USD"
            disabled={submitting}
          />
        </div>

        <div class="form-group">
          <label for="strategy">Strategy *</label>
          <input
            id="strategy"
            type="text"
            bind:value={formData.strategy}
            placeholder="Strategy name"
            disabled={submitting}
          />
        </div>

        <div class="form-group">
          <label for="granularity">Granularity</label>
          <select id="granularity" bind:value={formData.granularity} disabled={submitting}>
            <option value="">Select granularity</option>
            <option value="1m">1 minute</option>
            <option value="5m">5 minutes</option>
            <option value="15m">15 minutes</option>
            <option value="1h">1 hour</option>
            <option value="1d">1 day</option>
          </select>
        </div>

        <div class="form-group">
          <label for="capital">Initial Capital *</label>
          <input
            id="capital"
            type="number"
            bind:value={formData.initial_capital}
            placeholder="10000"
            disabled={submitting}
          />
        </div>

        <div class="form-group">
          <label for="start">Start Date</label>
          <input
            id="start"
            type="date"
            bind:value={formData.start_date}
            disabled={submitting}
          />
        </div>

        <div class="form-group">
          <label for="end">End Date</label>
          <input
            id="end"
            type="date"
            bind:value={formData.end_date}
            disabled={submitting}
          />
        </div>

        <div class="form-group">
          <label for="stopLoss">Stop Loss %</label>
          <input
            id="stopLoss"
            type="number"
            bind:value={formData.stop_loss_pct}
            placeholder="2.5"
            disabled={submitting}
          />
        </div>

        <div class="form-group">
          <label for="takeProfit">Take Profit %</label>
          <input
            id="takeProfit"
            type="number"
            bind:value={formData.take_profit_pct}
            placeholder="5.0"
            disabled={submitting}
          />
        </div>
      </div>

      <button
        class="btn-primary"
        on:click={handleSubmitJob}
        disabled={submitting}
      >
        {submitting ? 'Submitting...' : 'Submit Backtest'}
      </button>
    </div>
  {/if}

  <div class="jobs-content">
    <div class="jobs-list-section">
      <div class="list-controls">
        <input
          type="text"
          placeholder="Search by ID or symbol..."
          bind:value={searchTerm}
          class="search-input"
        />
        <select bind:value={statusFilter} class="status-filter">
          <option value="all">All Status</option>
          <option value="pending">Pending</option>
          <option value="complete">Complete</option>
        </select>
      </div>

      {#if $jobsLoading}
        <div class="loading">Loading jobs...</div>
      {:else if $jobsError}
        <div class="error">{$jobsError}</div>
      {:else if filteredJobs.length === 0}
        <div class="empty">No jobs found</div>
      {:else}
        <ul class="jobs-list">
          {#each filteredJobs as job (job.id)}
            <li>
              <button
                type="button"
                class="job-card"
                class:selected={$selectedJobId === job.id}
                on:click={() => handleSelectJob(job)}
              >
              <div class="job-card-header">
                <span class="symbol">{job.symbol}</span>
                <span class="status" class:pending={job.status === 'pending'}>
                  {job.status}
                </span>
              </div>
              <div class="job-card-body">
                <div class="job-id">{job.id}</div>
                <div class="job-type">{job.job_type}</div>
                <div class="job-date">{new Date(job.created_at).toLocaleString()}</div>
              </div>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    {#if selectedJob}
      <div class="job-detail-section">
        <h2>Job Details</h2>
        <div class="detail-card">
          <div class="detail-row">
            <span class="label">ID:</span>
            <span class="value">{selectedJob.id}</span>
          </div>
          <div class="detail-row">
            <span class="label">Type:</span>
            <span class="value">{selectedJob.job_type}</span>
          </div>
          <div class="detail-row">
            <span class="label">Symbol:</span>
            <span class="value">{selectedJob.symbol}</span>
          </div>

          {#if selectedJob.verdict}
            <div class="detail-section">
              <h3>Verdict</h3>
              <pre>{JSON.stringify(JSON.parse(selectedJob.verdict), null, 2)}</pre>
            </div>
          {/if}

          {#if selectedJob.metrics}
            <div class="detail-section">
              <h3>Metrics</h3>
              <div class="metrics-grid">
                {#each Object.entries(selectedJob.metrics) as [key, value]}
                  <div class="metric">
                    <span class="metric-key">{key}</span>
                    <span class="metric-value">{value}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .jobs-page {
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  .jobs-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
    border-bottom: 1px solid var(--color-border);
    padding-bottom: 1rem;
  }

  .jobs-header h1 {
    font-size: 2rem;
    font-weight: 600;
    margin: 0;
  }

  .subtitle {
    color: var(--color-text-secondary);
    margin: 0.5rem 0 0 0;
  }

  .btn-primary {
    padding: 0.75rem 1.5rem;
    background: var(--color-brand-primary);
    color: white;
    border: none;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--color-brand-primary-dark);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .submit-form {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 2rem;
    margin-bottom: 2rem;
  }

  .submit-form h2 {
    margin: 0 0 1rem 0;
    font-size: 1.25rem;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .form-group label {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--color-text-secondary);
  }

  .form-group input,
  .form-group select {
    padding: 0.75rem;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    font-size: 0.875rem;
    font-family: inherit;
  }

  .error {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid #fca5a5;
    color: #991b1b;
    padding: 1rem;
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .jobs-content {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
  }

  .jobs-list-section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 1.5rem;
  }

  .list-controls {
    display: flex;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .search-input,
  .status-filter {
    flex: 1;
    padding: 0.75rem;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    font-size: 0.875rem;
    font-family: inherit;
  }

  .status-filter {
    flex: 0 0 auto;
    min-width: 150px;
  }

  .loading,
  .empty {
    text-align: center;
    color: var(--color-text-tertiary);
    padding: 2rem;
  }

  .jobs-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .job-card {
    background: var(--color-border);
    border: 2px solid transparent;
    border-radius: 6px;
    padding: 1rem;
    margin-bottom: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .job-card:hover {
    background: var(--color-border-hover);
  }

  .job-card.selected {
    border-color: var(--color-brand-primary);
    background: rgba(0, 240, 255, 0.05);
  }

  .job-card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .symbol {
    font-weight: 600;
    color: var(--color-brand-primary);
  }

  .status {
    font-size: 0.75rem;
    padding: 0.25rem 0.75rem;
    border-radius: 4px;
    background: var(--color-border);
    color: var(--color-text-secondary);
  }

  .status.pending {
    background: rgba(245, 158, 11, 0.1);
    color: #f59e0b;
  }

  .job-card-body {
    font-size: 0.875rem;
    color: var(--color-text-secondary);
  }

  .job-id {
    font-family: monospace;
    font-size: 0.8rem;
  }

  .job-type,
  .job-date {
    font-size: 0.75rem;
    color: var(--color-text-tertiary);
  }

  .job-detail-section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 1.5rem;
  }

  .job-detail-section h2 {
    margin: 0 0 1rem 0;
    font-size: 1.25rem;
  }

  .detail-card {
    background: var(--color-border);
    border-radius: 6px;
    padding: 1.5rem;
  }

  .detail-row {
    display: flex;
    justify-content: space-between;
    padding: 0.75rem 0;
    border-bottom: 1px solid var(--color-border);
  }

  .detail-row:last-child {
    border-bottom: none;
  }

  .label {
    font-weight: 600;
    color: var(--color-text-secondary);
  }

  .value {
    color: var(--color-text);
    font-family: monospace;
  }

  .detail-section {
    margin-top: 1.5rem;
    padding-top: 1.5rem;
    border-top: 1px solid var(--color-border);
  }

  .detail-section h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
  }

  .detail-section pre {
    background: var(--color-surface);
    border-radius: 6px;
    padding: 1rem;
    font-size: 0.8rem;
    overflow-x: auto;
    margin: 0;
  }

  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1rem;
  }

  .metric {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .metric-key {
    font-size: 0.75rem;
    color: var(--color-text-tertiary);
    text-transform: uppercase;
  }

  .metric-value {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-brand-primary);
  }

  @media (max-width: 1024px) {
    .jobs-content {
      grid-template-columns: 1fr;
    }
  }
</style>
