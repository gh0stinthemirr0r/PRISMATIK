<script lang="ts">
  import { onMount } from 'svelte';
  import {
    sessions,
    activeSessions,
    sessionsLoading,
    sessionsError,
    selectedSessionId,
    syncSessions,
    initializeAPI,
  } from '$lib/stores';
  import { startSession, stopSession, getSessionEvents } from '$lib/api';

  let selectedSession: any = null;
  let sessionEvents: any[] = [];
  let starting = false;
  let stopping = false;
  let startError = '';

  onMount(async () => {
    const token = localStorage.getItem('api_token');
    if (token) {
      initializeAPI(token);
    }
    await syncSessions();
  });

  async function handleStartSession(mode: 'paper' | 'live') {
    starting = true;
    startError = '';
    try {
      const response = await startSession({ mode });
      await syncSessions();
    } catch (err) {
      startError = err instanceof Error ? err.message : 'Failed to start session';
    } finally {
      starting = false;
    }
  }

  async function handleStopSession() {
    if (!selectedSession) return;
    stopping = true;
    try {
      await stopSession(selectedSession.id);
      await syncSessions();
      selectedSession = null;
      sessionEvents = [];
    } catch (err) {
      console.error('Failed to stop session', err);
    } finally {
      stopping = false;
    }
  }

  async function handleSelectSession(session: any) {
    selectedSessionId.set(session.id);
    selectedSession = session;
    try {
      const response = await getSessionEvents(session.id, 50);
      sessionEvents = response.events;
    } catch (err) {
      console.error('Failed to load session events', err);
    }
  }
</script>

<svelte:head>
  <title>Sessions - PRISMATIK</title>
</svelte:head>

<div class="sessions-page">
  <div class="sessions-header">
    <div>
      <h1>Sessions</h1>
      <p class="subtitle">Manage paper and live trading sessions</p>
    </div>
    <div class="start-buttons">
      <button
        class="btn-primary"
        on:click={() => handleStartSession('paper')}
        disabled={starting}
      >
        + Paper Session
      </button>
      <button
        class="btn-danger"
        on:click={() => handleStartSession('live')}
        disabled={starting}
      >
        + Live Session
      </button>
    </div>
  </div>

  {#if startError}
    <div class="error">{startError}</div>
  {/if}

  <div class="sessions-content">
    <div class="sessions-list-section">
      <h2>All Sessions</h2>
      {#if $sessionsLoading}
        <div class="loading">Loading sessions...</div>
      {:else if $sessionsError}
        <div class="error">{$sessionsError}</div>
      {:else if $sessions.length === 0}
        <div class="empty">No sessions</div>
      {:else}
        <ul class="sessions-list">
          {#each $sessions as session (session.id)}
            <li>
              <button
                type="button"
                class="session-card"
                class:selected={$selectedSessionId === session.id}
                on:click={() => handleSelectSession(session)}
              >
              <div class="session-card-header">
                <div class="session-mode" class:paper={session.mode === 'paper'} class:live={session.mode === 'live'}>
                  {session.mode}
                </div>
                <span class="session-status" class:running={session.status === 'running'}>
                  {session.status}
                </span>
              </div>
              <div class="session-card-body">
                <div class="session-id">{session.id}</div>
                <div class="session-time">{new Date(session.started_at).toLocaleString()}</div>
              </div>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    {#if selectedSession}
      <div class="session-detail-section">
        <div class="detail-header">
          <h2>Session Details</h2>
          {#if selectedSession.status === 'running'}
            <button
              class="btn-danger"
              on:click={handleStopSession}
              disabled={stopping}
            >
              {stopping ? 'Stopping...' : 'Stop Session'}
            </button>
          {/if}
        </div>

        <div class="detail-card">
          <div class="detail-row">
            <span class="label">ID:</span>
            <span class="value">{selectedSession.id}</span>
          </div>
          <div class="detail-row">
            <span class="label">Mode:</span>
            <span class="value">{selectedSession.mode}</span>
          </div>
          <div class="detail-row">
            <span class="label">Status:</span>
            <span class="value">{selectedSession.status}</span>
          </div>
          <div class="detail-row">
            <span class="label">Started:</span>
            <span class="value">{new Date(selectedSession.started_at).toLocaleString()}</span>
          </div>
          {#if selectedSession.stopped_at}
            <div class="detail-row">
              <span class="label">Stopped:</span>
              <span class="value">{new Date(selectedSession.stopped_at).toLocaleString()}</span>
            </div>
          {/if}
        </div>

        <div class="events-section">
          <h3>Events ({sessionEvents.length})</h3>
          {#if sessionEvents.length === 0}
            <p class="empty-state">No events</p>
          {:else}
            <ul class="events-list">
              {#each sessionEvents as event}
                <li class="event-item">
                  <div class="event-time">{new Date(event.timestamp).toLocaleTimeString()}</div>
                  <div class="event-type">{event.event_type}</div>
                  {#if Object.keys(event.data).length > 0}
                    <pre class="event-data">{JSON.stringify(event.data, null, 2)}</pre>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .sessions-page {
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  .sessions-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
    border-bottom: 1px solid var(--color-border);
    padding-bottom: 1rem;
  }

  .sessions-header h1 {
    font-size: 2rem;
    font-weight: 600;
    margin: 0;
  }

  .subtitle {
    color: var(--color-text-secondary);
    margin: 0.5rem 0 0 0;
  }

  .start-buttons {
    display: flex;
    gap: 1rem;
  }

  .btn-primary,
  .btn-danger {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-primary {
    background: var(--color-brand-primary);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--color-brand-primary-dark);
  }

  .btn-danger {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  .btn-danger:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.3);
  }

  .btn-primary:disabled,
  .btn-danger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid #fca5a5;
    color: #991b1b;
    padding: 1rem;
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .sessions-content {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
  }

  .sessions-list-section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 1.5rem;
  }

  .sessions-list-section h2 {
    margin: 0 0 1rem 0;
    font-size: 1.25rem;
  }

  .loading,
  .empty {
    text-align: center;
    color: var(--color-text-tertiary);
    padding: 2rem;
  }

  .sessions-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .session-card {
    background: var(--color-border);
    border: 2px solid transparent;
    border-radius: 6px;
    padding: 1rem;
    margin-bottom: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .session-card:hover {
    background: var(--color-border-hover);
  }

  .session-card.selected {
    border-color: var(--color-brand-primary);
    background: rgba(0, 240, 255, 0.05);
  }

  .session-card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .session-mode {
    padding: 0.25rem 0.75rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
  }

  .session-mode.paper {
    background: rgba(59, 130, 246, 0.1);
    color: #3b82f6;
  }

  .session-mode.live {
    background: rgba(239, 68, 68, 0.1);
    color: #ef4444;
  }

  .session-status {
    font-size: 0.75rem;
    padding: 0.25rem 0.75rem;
    border-radius: 4px;
    background: var(--color-border);
    color: var(--color-text-secondary);
  }

  .session-status.running {
    background: rgba(34, 197, 94, 0.1);
    color: #22c55e;
  }

  .session-card-body {
    font-size: 0.875rem;
    color: var(--color-text-secondary);
  }

  .session-id {
    font-family: monospace;
    font-size: 0.8rem;
  }

  .session-time {
    font-size: 0.75rem;
    color: var(--color-text-tertiary);
  }

  .session-detail-section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 1.5rem;
  }

  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .detail-header h2 {
    margin: 0;
    font-size: 1.25rem;
  }

  .detail-card {
    background: var(--color-border);
    border-radius: 6px;
    padding: 1.5rem;
    margin-bottom: 1.5rem;
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

  .events-section h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
  }

  .empty-state {
    text-align: center;
    color: var(--color-text-tertiary);
    padding: 2rem;
    margin: 0;
  }

  .events-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .event-item {
    background: var(--color-border);
    border-radius: 6px;
    padding: 1rem;
    margin-bottom: 0.75rem;
    font-size: 0.875rem;
  }

  .event-time {
    color: var(--color-brand-primary);
    font-weight: 600;
    font-family: monospace;
    margin-bottom: 0.25rem;
  }

  .event-type {
    color: var(--color-text-secondary);
    text-transform: uppercase;
    font-size: 0.75rem;
    margin-bottom: 0.5rem;
  }

  .event-data {
    background: var(--color-surface);
    border-radius: 4px;
    padding: 0.75rem;
    font-size: 0.75rem;
    margin: 0;
    overflow-x: auto;
  }

  @media (max-width: 1024px) {
    .sessions-content {
      grid-template-columns: 1fr;
    }
  }
</style>
