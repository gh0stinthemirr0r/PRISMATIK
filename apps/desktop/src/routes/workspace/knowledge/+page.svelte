<script lang="ts">
  /**
   * The knowledge base.
   *
   * Documents are things a human or a provider *said* — notes, filings, calls,
   * research. Nothing PRISMATIK computes goes in here: regime, edge and skill
   * are recomputed on every read and would be stale the moment they were
   * written down.
   *
   * Every write wires its own instrument links with no model call, and every
   * query reports what it could not cover. The gap list is the point: retrieval
   * that returns five documents for a question the corpus knows nothing about
   * launders absence into confidence.
   */
  import { onMount } from 'svelte';
  import { Plus, Trash2, Search, Loader2 } from 'lucide-svelte';
  import NoData from '$lib/prismatik/NoData.svelte';

  interface DocumentView {
    id: string;
    kind: string;
    title: string;
    body: string;
    about: string[];
    mentions: string[];
    createdAt: string;
  }

  interface KnowledgeAnswer {
    rendered: string;
    hitCount: number;
    uncoveredTerms: string[];
    uncoveredSymbols: string[];
    corpusSize: number;
  }

  const KINDS = ['note', 'filing', 'news', 'research', 'transcript', 'decision'];

  let documents = $state<DocumentView[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(false);

  let kind = $state('note');
  let title = $state('');
  let body = $state('');
  let about = $state('');
  let composerOpen = $state(false);

  let query = $state('');
  let answer = $state<KnowledgeAnswer | null>(null);
  let searching = $state(false);

  async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke: call, isTauri } = await import('@tauri-apps/api/core');
    if (!isTauri()) throw new Error('desktop runtime required');
    return call<T>(command, args);
  }

  async function load(): Promise<void> {
    loading = true;
    try {
      documents = await invoke<DocumentView[]>('list_documents', { symbol: null });
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function add(): Promise<void> {
    try {
      await invoke<number>('add_document', {
        draft: {
          kind,
          title,
          body,
          about: about.split(',').map((s) => s.trim()).filter(Boolean),
        },
      });
      title = '';
      body = '';
      about = '';
      composerOpen = false;
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function remove(id: string): Promise<void> {
    try {
      await invoke<number>('remove_document', { id });
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function search(): Promise<void> {
    if (!query.trim()) return;
    searching = true;
    try {
      answer = await invoke<KnowledgeAnswer>('query_knowledge', { query: query.trim() });
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      searching = false;
    }
  }
</script>

<svelte:head><title>Knowledge · PRISMATIK</title></svelte:head>

<main class="pk-knowledge">
  <header>
    <div>
      <p class="eyebrow">INSTITUTIONAL MEMORY</p>
      <h1>Knowledge base</h1>
      <p class="lede">
        Notes, filings, calls and research the desk should not have to re-read. Instrument links are
        wired on write with no model call, so a link can never be hallucinated — only a symbol the
        desk already tracks can be linked. Every search reports what it could not cover.
      </p>
    </div>
    <button class="add" onclick={() => (composerOpen = !composerOpen)}>
      <Plus size={13} /> Add document
    </button>
  </header>

  {#if error}<p class="error">{error}</p>{/if}

  {#if composerOpen}
    <section class="composer">
      <div class="row">
        <label>
          <span>Kind</span>
          <select bind:value={kind}>{#each KINDS as k (k)}<option value={k}>{k}</option>{/each}</select>
        </label>
        <label class="grow"><span>Title</span><input bind:value={title} /></label>
        <label>
          <span>About (symbols)</span>
          <input bind:value={about} placeholder="NVDA, AMD" />
        </label>
      </div>
      <label>
        <span>Body</span>
        <textarea rows="6" bind:value={body} placeholder="Paste the note, filing extract or transcript."></textarea>
      </label>
      <div class="composer-actions">
        <button class="ghost" onclick={() => (composerOpen = false)}>Cancel</button>
        <button class="primary" onclick={() => void add()}>Store</button>
      </div>
    </section>
  {/if}

  <section class="search">
    <label class="grow">
      <span>Ask the knowledge base</span>
      <input bind:value={query} placeholder="datacenter demand constraints" onkeydown={(e) => e.key === 'Enter' && search()} />
    </label>
    <button class="primary" disabled={searching || !query.trim()} onclick={() => void search()}>
      {#if searching}<span class="spin"><Loader2 size={13} /></span>{:else}<Search size={13} />{/if}
      Search
    </button>
  </section>

  {#if answer}
    <section class="answer">
      <div class="answer-head">
        <b>{answer.hitCount} of {answer.corpusSize} documents matched</b>
        {#if answer.uncoveredSymbols.length > 0 || answer.uncoveredTerms.length > 0}
          <span class="gaps">
            Gaps —
            {#if answer.uncoveredSymbols.length}nothing about {answer.uncoveredSymbols.join(', ')}{/if}
            {#if answer.uncoveredSymbols.length && answer.uncoveredTerms.length}·{/if}
            {#if answer.uncoveredTerms.length}no document contains {answer.uncoveredTerms.join(', ')}{/if}
          </span>
        {/if}
      </div>
      <pre>{answer.rendered}</pre>
    </section>
  {/if}

  {#if loading && documents.length === 0}
    <NoData title="Loading documents" compact />
  {:else if documents.length === 0}
    <NoData
      title="Nothing stored yet"
      detail="Add a note, a filing extract or a call transcript. Analysts retrieve from here, and are told explicitly when it has nothing to say."
    />
  {:else}
    <div class="docs">
      {#each documents as doc (doc.id)}
        <article>
          <header>
            <div>
              <span class="kind">{doc.kind}</span>
              <b>{doc.title}</b>
            </div>
            <button aria-label="Remove document" onclick={() => void remove(doc.id)}>
              <Trash2 size={12} />
            </button>
          </header>
          <p>{doc.body.length > 320 ? `${doc.body.slice(0, 320)}…` : doc.body}</p>
          <footer>
            {#each doc.about as s (s)}<span class="about">{s}</span>{/each}
            {#each doc.mentions as s (s)}<span class="mention">{s}</span>{/each}
            <time>{doc.createdAt}</time>
          </footer>
        </article>
      {/each}
    </div>
  {/if}
</main>

<style>
  .pk-knowledge {
    height: 100%;
    overflow: auto;
    padding: 28px clamp(18px, 3vw, 40px) 60px;
    color: var(--p-text);
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 22px;
  }
  .eyebrow {
    margin: 0;
    color: var(--p-accent);
    font: 700 9px var(--font-mono);
    letter-spacing: 0.18em;
  }
  h1 {
    margin: 8px 0 6px;
    font-size: clamp(1.8rem, 3vw, 2.6rem);
    letter-spacing: -0.04em;
  }
  .lede {
    max-width: 78ch;
    margin: 0;
    color: var(--p-dim);
    line-height: 1.55;
  }
  .add {
    display: flex;
    align-items: center;
    gap: 6px;
    height: fit-content;
    padding: 8px 12px;
    border: 1px solid var(--p-border);
    border-radius: 6px;
    background: var(--p-panel-fill);
    color: var(--p-accent);
    cursor: pointer;
    font: 700 0.62rem var(--font-mono);
    white-space: nowrap;
  }
  .error {
    margin: 14px 0 0;
    padding: 10px 12px;
    border: 1px solid color-mix(in srgb, var(--p-down) 45%, transparent);
    border-radius: 7px;
    color: var(--p-down);
    font-size: 0.75rem;
  }
  .composer,
  .search {
    display: grid;
    gap: 10px;
    margin: 16px 0;
    padding: 14px;
    border: 1px solid var(--p-border);
    border-radius: 9px;
    background: var(--p-panel-fill);
  }
  .search {
    display: flex;
    align-items: end;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  label {
    display: grid;
    gap: 4px;
  }
  label > span {
    color: var(--p-dim);
    font: 600 8px var(--font-mono);
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .grow {
    flex: 1;
  }
  input,
  select,
  textarea {
    padding: 8px 9px;
    border: 1px solid var(--p-border);
    border-radius: 6px;
    background: var(--p-surface2);
    color: var(--p-text);
    font-family: var(--font-mono);
    font-size: 0.75rem;
  }
  textarea {
    resize: vertical;
    line-height: 1.5;
  }
  .composer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  button.primary {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border: 1px solid var(--p-accent);
    border-radius: 6px;
    background: color-mix(in srgb, var(--p-accent) 12%, transparent);
    color: var(--p-accent);
    cursor: pointer;
    font: 700 0.64rem var(--font-mono);
  }
  button.primary:disabled {
    cursor: default;
    opacity: 0.5;
  }
  button.ghost {
    padding: 8px 14px;
    border: 1px solid var(--p-border);
    border-radius: 6px;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
    font: 700 0.64rem var(--font-mono);
  }
  .spin {
    display: inline-flex;
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }

  .answer {
    margin-bottom: 16px;
    border: 1px solid var(--p-border);
    border-radius: 9px;
    background: var(--p-panel-fill);
    overflow: hidden;
  }
  .answer-head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 12px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--p-border);
    font-size: 0.72rem;
  }
  /* Gaps are amber, not grey: they are the part people skip. */
  .gaps {
    color: #fbbf24;
    font-size: 0.66rem;
  }
  .answer pre {
    margin: 0;
    padding: 14px;
    font: inherit;
    font-size: 0.72rem;
    line-height: 1.55;
    white-space: pre-wrap;
  }

  .docs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 10px;
  }
  .docs article {
    display: grid;
    align-content: start;
    border: 1px solid var(--p-border);
    border-radius: 9px;
    background: var(--p-panel-fill);
    overflow: hidden;
  }
  .docs header {
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--p-border);
  }
  .kind {
    display: block;
    color: var(--p-accent);
    font: 700 8px var(--font-mono);
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .docs header b {
    font-size: 0.78rem;
  }
  .docs header button {
    border: none;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
  }
  .docs header button:hover {
    color: var(--p-down);
  }
  .docs p {
    margin: 0;
    padding: 10px 12px;
    color: var(--p-dim);
    font-size: 0.72rem;
    line-height: 1.5;
  }
  .docs footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 5px;
    padding: 8px 12px;
    border-top: 1px solid var(--p-grid);
  }
  .about,
  .mention {
    padding: 2px 6px;
    border-radius: 4px;
    font: 700 0.6rem var(--font-mono);
  }
  /* Subject and passing mention are visually distinct — the difference drives
     retrieval ranking. */
  .about {
    background: color-mix(in srgb, var(--p-accent) 18%, transparent);
    color: var(--p-accent);
  }
  .mention {
    background: var(--p-surface2);
    color: var(--p-dim);
  }
  .docs time {
    margin-left: auto;
    color: var(--p-dim);
    font: 0.58rem var(--font-mono);
  }
</style>
