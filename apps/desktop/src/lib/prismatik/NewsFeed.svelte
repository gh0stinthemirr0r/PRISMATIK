<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface FeedSourceConfig {
    id: string;
    url: string;
    enabled: boolean;
    minimumPollSeconds: number;
    validUntil: string;
    version: number;
  }

  interface FeedRuntimeStatus {
    sourceId: string;
    policyVersion: number;
    state: string;
    lastAttemptAt: string | null;
    lastSuccessAt: string | null;
    nextAttemptAt: string;
    consecutiveFailures: number;
    statusCode: number | null;
    bytesReceived: number;
    etag: string | null;
    lastModified: string | null;
    message: string;
  }

  interface AuditEvent {
    id: string;
    occurredAt: string;
    domain: string;
    severity: string;
    state: string;
    title: string;
    summary: string;
    evidenceId: string | null;
    route: string | null;
    durable: boolean;
  }

  interface TerminalFeedQuote {
    symbol: string;
    price: number;
    provider: string;
    observedAt: string;
  }

  let sources = $state<FeedSourceConfig[]>([]);
  let statuses = $state<FeedRuntimeStatus[]>([]);
  let events = $state<AuditEvent[]>([]);
  let quotes = $state<TerminalFeedQuote[]>([]);
  let loading = $state(true);
  let mode = $state<'feed' | 'audit'>('feed');

  async function refresh() {
    try {
      const [src, sts, evs, snap] = await Promise.all([
        invoke<FeedSourceConfig[]>('list_feed_sources').catch(() => []),
        invoke<FeedRuntimeStatus[]>('feed_runtime_status').catch(() => []),
        invoke<AuditEvent[]>('get_audit_timeline').catch(() => []),
        invoke<{ quotes: TerminalFeedQuote[] }>('get_terminal_feed').catch(() => ({ quotes: [] })),
      ]);
      sources = src;
      statuses = sts;
      events = evs.filter((e) => e.domain === 'feed' || e.domain === 'research' || e.domain === 'execution').slice(0, 20);
      quotes = snap.quotes;
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    refresh();
    const interval = setInterval(refresh, 30_000);
    return () => clearInterval(interval);
  });

  function ago(iso: string | null): string {
    if (!iso) return 'never';
    const diff = Date.now() - new Date(iso).getTime();
    if (diff < 0) return 'pending';
    const s = Math.floor(diff / 1000);
    if (s < 60) return `${s}s ago`;
    const m = Math.floor(s / 60);
    if (m < 60) return `${m}m ago`;
    return `${Math.floor(m / 60)}h ago`;
  }

  function stateColor(state: string): string {
    if (state.includes('success') || state.includes('not_modified')) return '#34d399';
    if (state.includes('backoff') || state.includes('fail')) return '#fbbf24';
    return '#94a3b8';
  }

  const activeSources = $derived(sources.filter((s) => s.enabled));
  const hasRealData = $derived(activeSources.length > 0 || events.length > 0);
</script>

<section class="pk-panel pk-scenario">
  <div class="pk-panel-head">
    <div class="pk-tabs">
      <button class:active={mode === 'feed'} onclick={() => (mode = 'feed')}>Feed sources</button>
      <button class:active={mode === 'audit'} onclick={() => (mode = 'audit')}>Activity</button>
    </div>
    <span class="pk-live" class:connected={hasRealData}>{hasRealData ? 'LIVE' : 'NO SOURCES'}</span>
  </div>

  <div class="pk-scroll" style="flex:1">
    {#if loading}
      <div class="pk-scenario-empty">Loading…</div>
    {:else if mode === 'feed'}
      {#if activeSources.length === 0}
        <div class="pk-scenario-empty">
          <p>No RSS/Atom feeds connected.</p>
          <p class="pk-dim">Connect reviewed sources in <a href="/workspace/feeds">Feeds</a> to stream real headlines here.</p>
          <p class="pk-dim pk-small">Mock scenario headlines have been removed. This surface only shows real governed feed activity.</p>
        </div>
      {:else}
        {#each activeSources as source (source.id)}
          {@const status = statuses.find((s) => s.sourceId === source.id)}
          <article class="pk-feed-source">
            <div class="pk-feed-meta">
              <span class="pk-feed-dot" style="background: {status ? stateColor(status.state) : '#94a3b8'}"></span>
              <span class="pk-feed-url">{source.url}</span>
              <span style="margin-left:auto" class="pk-dim">{status ? ago(status.lastSuccessAt) : '—'}</span>
            </div>
            <div class="pk-feed-state">{status?.state ?? 'pending'} · {status?.bytesReceived ?? 0} bytes{#if status?.statusCode} · HTTP {status.statusCode}{/if}</div>
            {#if status?.message}
              <div class="pk-feed-msg pk-dim">{status.message}</div>
            {/if}
          </article>
        {/each}
      {/if}
    {:else}
      {#if events.length === 0}
        <div class="pk-scenario-empty">
          <p>No recent activity.</p>
          <p class="pk-dim">Feed acquisitions, research runs, and paper fills will appear here as they happen.</p>
        </div>
      {:else}
        {#each events as event (event.id)}
          <article class="pk-activity" class:critical={event.severity === 'critical'} class:warning={event.severity === 'warning'}>
            <div class="pk-activity-meta">
              <span class="pk-activity-domain">{event.domain}</span>
              <span class="pk-dim">{ago(event.occurredAt)}</span>
            </div>
            <div class="pk-activity-title">{event.title}</div>
            {#if event.summary}
              <div class="pk-activity-summary pk-dim">{event.summary}</div>
            {/if}
          </article>
        {/each}
      {/if}
    {/if}
  </div>
</section>

<style>
  .pk-scenario { display: flex; flex-direction: column; }
  .pk-panel-head { display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; border-bottom: 1px solid var(--p-border); }
  .pk-tabs { display: flex; gap: 2px; }
  .pk-tabs button { background: none; border: none; color: var(--p-text-dim); font-family: var(--p-mono); font-size: 0.625rem; letter-spacing: 0.1em; text-transform: uppercase; cursor: pointer; padding: 4px 8px; border-radius: 3px; }
  .pk-tabs button.active { background: var(--p-surface2); color: var(--p-text); }
  .pk-live { font-family: var(--p-mono); font-size: 0.5625rem; letter-spacing: 0.1em; padding: 2px 8px; border-radius: 3px; background: rgba(148, 163, 184, 0.1); color: #94a3b8; }
  .pk-live.connected { background: rgba(52, 211, 153, 0.12); color: #34d399; }
  .pk-scenario-empty { padding: 32px 16px; text-align: center; color: var(--p-text-dim); font-size: 0.8125rem; line-height: 1.6; }
  .pk-scenario-empty a { color: var(--p-accent); }
  .pk-dim { color: var(--p-text-dim); }
  .pk-small { font-size: 0.6875rem; opacity: 0.7; }
  .pk-feed-source { padding: 10px 14px; border-bottom: 1px solid var(--p-border); }
  .pk-feed-meta { display: flex; align-items: center; gap: 8px; font-size: 0.75rem; }
  .pk-feed-dot { width: 7px; height: 7px; border-radius: 50%; flex: none; }
  .pk-feed-url { color: var(--p-text); font-family: var(--p-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 200px; }
  .pk-feed-state { font-size: 0.6875rem; color: var(--p-text-dim); font-family: var(--p-mono); margin-top: 3px; }
  .pk-feed-msg { font-size: 0.6875rem; margin-top: 3px; line-height: 1.4; }
  .pk-activity { padding: 10px 14px; border-bottom: 1px solid var(--p-border); border-left: 2px solid transparent; }
  .pk-activity.critical { border-left-color: #f87171; background: rgba(248, 113, 113, 0.03); }
  .pk-activity.warning { border-left-color: #fbbf24; background: rgba(251, 191, 36, 0.03); }
  .pk-activity-meta { display: flex; align-items: center; gap: 8px; font-size: 0.625rem; font-family: var(--p-mono); text-transform: uppercase; letter-spacing: 0.08em; }
  .pk-activity-domain { color: var(--p-accent); }
  .pk-activity-title { font-size: 0.8125rem; color: var(--p-text); margin-top: 3px; line-height: 1.4; }
  .pk-activity-summary { font-size: 0.6875rem; margin-top: 2px; line-height: 1.4; }
</style>
