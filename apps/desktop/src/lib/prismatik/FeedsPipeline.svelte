<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke, isTauri } from '@tauri-apps/api/core';
  import { importCoreFeeds, fetchAllFeeds, extractEntitiesSimple, scoreSentimentSimple, type ParsedArticle } from '$lib/rss/ingestion';
  import { Rss, Play, Loader2, CheckCircle, AlertCircle, Globe, TrendingUp, Newspaper } from 'lucide-svelte';

  let importing = $state(false);
  let fetching = $state(false);
  let feedCount = $state(0);
  let articleCount = $state(0);
  let lastFetch = $state<string | null>(null);
  let articles = $state<ParsedArticle[]>([]);
  let error = $state('');
  let status = $state<'idle' | 'importing' | 'fetching' | 'ready' | 'error'>('idle');

  // entity/sentiment aggregation
  let entitySentiment = $state<Map<string, { count: number; avgSentiment: number; sources: Set<string> }>>(new Map());

  async function doImport() {
    importing = true;
    error = '';
    status = 'importing';
    try {
      const result = await importCoreFeeds();
      feedCount = result.imported;
      if (result.errors.length > 0) {
        error = result.errors.join('; ');
      }
      status = 'idle';
    } catch (e) {
      error = String(e);
      status = 'error';
    } finally {
      importing = false;
    }
  }

  async function doFetch() {
    fetching = true;
    error = '';
    status = 'fetching';
    try {
      // get feed sources from backend
      const sources = await invoke<Array<{ id: string; url: string; enabled: boolean }>>('list_feed_sources');
      const enabled = sources.filter(s => s.enabled);

      if (enabled.length === 0) {
        error = 'No feeds configured. Run "Import Core Feeds" first.';
        status = 'idle';
        return;
      }

      // fetch and parse all feeds
      const parsed = await fetchAllFeeds(enabled.map(s => ({ id: s.id, url: s.url })), 8);
      articles = parsed;
      articleCount = parsed.length;
      lastFetch = new Date().toISOString();

      // aggregate entity sentiment
      const agg = new Map<string, { count: number; totalSentiment: number; sources: Set<string> }>();
      parsed.forEach(article => {
        const entities = extractEntitiesSimple(article.title + ' ' + article.description);
        const sentiment = scoreSentimentSimple(article.title + ' ' + article.description);
        entities.forEach(entity => {
          const existing = agg.get(entity) ?? { count: 0, totalSentiment: 0, sources: new Set() };
          existing.count++;
          existing.totalSentiment += sentiment;
          existing.sources.add(article.source);
          agg.set(entity, existing);
        });
      });

      const result = new Map<string, { count: number; avgSentiment: number; sources: Set<string> }>();
      agg.forEach((val, key) => {
        result.set(key, {
          count: val.count,
          avgSentiment: val.totalSentiment / val.count,
          sources: val.sources,
        });
      });
      entitySentiment = result;

      status = 'ready';
    } catch (e) {
      error = String(e);
      status = 'error';
    } finally {
      fetching = false;
    }
  }

  // top entities by mention count
  let topEntities = $derived(
    [...entitySentiment.entries()]
      .sort((a, b) => b[1].count - a[1].count)
      .slice(0, 20)
  );

  // sentiment distribution
  let sentimentBuckets = $derived(() => {
    const buckets = { strong_negative: 0, negative: 0, neutral: 0, positive: 0, strong_positive: 0 };
    entitySentiment.forEach(val => {
      if (val.avgSentiment < -0.3) buckets.strong_negative++;
      else if (val.avgSentiment < -0.05) buckets.negative++;
      else if (val.avgSentiment < 0.05) buckets.neutral++;
      else if (val.avgSentiment < 0.3) buckets.positive++;
      else buckets.strong_positive++;
    });
    return buckets;
  });

  onMount(() => {
    // auto-check if feeds are already configured
    if (isTauri()) {
      invoke<Array<{ id: string }>>('list_feed_sources').then(sources => {
        feedCount = sources.length;
      }).catch(() => {});
    }
  });
</script>

<div class="pk-feeds">
  <div class="pk-feeds-header">
    <div class="pk-feeds-title">
      <Rss size={14} />
      <span>News Intelligence Pipeline</span>
    </div>
    <div class="pk-feeds-stats">
      {#if feedCount > 0}
        <span class="stat"><Globe size={11} /> {feedCount} feeds</span>
      {/if}
      {#if articleCount > 0}
        <span class="stat"><Newspaper size={11} /> {articleCount} articles</span>
      {/if}
      {#if lastFetch}
        <span class="stat">Last: {new Date(lastFetch).toLocaleTimeString()}</span>
      {/if}
    </div>
  </div>

  <div class="pk-feeds-actions">
    <button onclick={doImport} disabled={importing}>
      {#if importing}<Loader2 size={14} class="pk-spin" />{:else}<Rss size={14} />{/if}
      Import Core Feeds ({feedCount > 0 ? feedCount + ' loaded' : 'first run'})
    </button>
    <button onclick={doFetch} disabled={fetching || feedCount === 0}>
      {#if fetching}<Loader2 size={14} class="pk-spin" />{:else}<Play size={14} />{/if}
      Fetch & Analyze
    </button>
  </div>

  {#if error}
    <div class="pk-feeds-error"><AlertCircle size={12} /> {error}</div>
  {/if}

  {#if status === 'ready' && topEntities.length > 0}
    <div class="pk-feeds-section">
      <h4>Top Entities by Mentions</h4>
      <div class="pk-entity-grid">
        {#each topEntities as [entity, data]}
          <div class="pk-entity-card" style="border-color: {data.avgSentiment > 0.1 ? '#34d399' : data.avgSentiment < -0.1 ? '#f87171' : '#374151'}">
            <div class="pk-entity-name">{entity}</div>
            <div class="pk-entity-meta">
              <span>{data.count} mentions</span>
              <span style="color: {data.avgSentiment > 0.1 ? '#34d399' : data.avgSentiment < -0.1 ? '#f87171' : '#6b7280'}">
                {data.avgSentiment > 0 ? '+' : ''}{(data.avgSentiment * 100).toFixed(0)}% sentiment
              </span>
            </div>
            <div class="pk-entity-sources">{[...data.sources].slice(0, 3).join(', ')}</div>
          </div>
        {/each}
      </div>
    </div>

    <div class="pk-feeds-section">
      <h4>Recent Articles</h4>
      <div class="pk-article-list">
        {#each articles.slice(0, 30) as article}
          <div class="pk-article">
            <div class="pk-article-source">{article.source}</div>
            <div class="pk-article-title">{article.title}</div>
            <div class="pk-article-desc">{article.description?.substring(0, 120)}...</div>
            {#if article.categories.length > 0}
              <div class="pk-article-tags">
                {#each article.categories.slice(0, 3) as cat}
                  <span class="pk-tag">{cat}</span>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .pk-feeds { display: flex; flex-direction: column; height: 100%; background: #050510; overflow-y: auto; font-family: 'IBM Plex Mono', monospace; }
  .pk-feeds-header { padding: 14px 16px; border-bottom: 1px solid #1a2332; display: flex; justify-content: space-between; align-items: center; }
  .pk-feeds-title { display: flex; align-items: center; gap: 8px; font-size: 0.75rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.04em; color: #e5e7eb; }
  .pk-feeds-stats { display: flex; gap: 12px; font-size: 0.625rem; color: #6b7280; }
  .stat { display: flex; align-items: center; gap: 4px; }
  .pk-feeds-actions { display: flex; gap: 8px; padding: 10px 16px; border-bottom: 1px solid #1a2332; }
  .pk-feeds-actions button { display: flex; align-items: center; gap: 6px; padding: 8px 16px; border: 1px solid #2d3748; border-radius: 6px; background: #0a0f1e; color: #e5e7eb; font-size: 0.6875rem; font-family: 'IBM Plex Mono', monospace; cursor: pointer; transition: all 0.15s; }
  .pk-feeds-actions button:hover { border-color: #00f0ff; color: #00f0ff; }
  .pk-feeds-actions button:disabled { opacity: 0.3; cursor: default; }
  .pk-feeds-error { display: flex; align-items: center; gap: 6px; padding: 8px 16px; background: rgba(239,68,68,0.1); color: #f87171; font-size: 0.6875rem; }
  .pk-feeds-section { padding: 16px; }
  .pk-feeds-section h4 { margin: 0 0 12px; font-size: 0.6875rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: #00f0ff; }
  .pk-entity-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 8px; }
  .pk-entity-card { padding: 10px 12px; background: #0a0f1e; border: 1px solid #1a2332; border-radius: 6px; border-left: 3px solid; }
  .pk-entity-name { font-size: 0.75rem; font-weight: 700; color: #e5e7eb; margin-bottom: 4px; }
  .pk-entity-meta { display: flex; justify-content: space-between; font-size: 0.625rem; color: #9ca3af; margin-bottom: 4px; }
  .pk-entity-sources { font-size: 0.5625rem; color: #4b5563; }
  .pk-article-list { display: flex; flex-direction: column; gap: 8px; }
  .pk-article { padding: 10px 12px; background: #0a0f1e; border: 1px solid #1a2332; border-radius: 6px; }
  .pk-article-source { font-size: 0.5625rem; color: #00f0ff; text-transform: uppercase; margin-bottom: 4px; }
  .pk-article-title { font-size: 0.75rem; font-weight: 600; color: #e5e7eb; margin-bottom: 4px; line-height: 1.4; }
  .pk-article-desc { font-size: 0.625rem; color: #6b7280; line-height: 1.4; margin-bottom: 6px; }
  .pk-article-tags { display: flex; gap: 4px; }
  .pk-tag { padding: 2px 6px; background: #1a2332; border-radius: 3px; font-size: 0.5rem; color: #9ca3af; }
  :global(.pk-spin) { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
