<script lang="ts">
  /**
   * Specialist analysts.
   *
   * Every analyst is a calibration cohort, which is what separates this from a
   * gallery of personas: the standing line under each name is a Brier skill
   * score against climatology, not a claim about how good the prompt sounds.
   * An unproven analyst says so, in plain language, everywhere it appears.
   */
  import { onMount } from 'svelte';
  import { Plus, Trash2, Send, Loader2, BookOpen } from 'lucide-svelte';
  import NoData from '$lib/prismatik/NoData.svelte';
  import { NO_VALUE } from '$lib/prismatik/market.svelte';

  type Scope =
    | { kind: 'instrument'; symbol: string }
    | { kind: 'sector'; label: string; symbols: string[] }
    | { kind: 'global' };

  interface AnalystView {
    id: string;
    name: string;
    scope: Scope;
    mandate: string;
    mandateVersion: number;
    enabled: boolean;
    cohortModel: string;
    scopeLabel: string;
    skillPpm: number | null;
    resolvedCount: number;
    standing: string;
    untrackedSymbols: string[];
    coverage: string;
  }

  interface AnalystAnswer {
    analystId: string;
    analystName: string;
    cohortModel: string;
    answer: string;
    contextSections: string[];
    standing: string;
    answeredAt: string;
  }

  interface ProviderStatus {
    providerId: string;
    active: boolean;
  }

  let analysts = $state<AnalystView[]>([]);
  let activeProviders = $state<string[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(false);

  // composer
  let editingId = $state<string | null>(null);
  let name = $state('');
  let scopeKind = $state<'instrument' | 'sector' | 'global'>('instrument');
  let symbol = $state('');
  let sectorLabel = $state('');
  let sectorSymbols = $state('');
  let mandate = $state('');
  let composerOpen = $state(false);

  // conversation
  let selectedId = $state<string | null>(null);
  let providerId = $state('');
  let model = $state('');
  let question = $state('');
  let asking = $state(false);
  let answers = $state<AnalystAnswer[]>([]);

  const selected = $derived(analysts.find((a) => a.id === selectedId) ?? null);

  async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke: call, isTauri } = await import('@tauri-apps/api/core');
    if (!isTauri()) throw new Error('desktop runtime required');
    return call<T>(command, args);
  }

  async function load(): Promise<void> {
    loading = true;
    try {
      analysts = await invoke<AnalystView[]>('list_analysts');
      const rows = await invoke<ProviderStatus[]>('model_runtime_status');
      activeProviders = rows.filter((r) => r.active).map((r) => r.providerId);
      if (!providerId) providerId = activeProviders[0] ?? '';
      if (!selectedId && analysts.length > 0) selectedId = analysts[0].id;
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function buildScope(): Scope {
    if (scopeKind === 'instrument') return { kind: 'instrument', symbol: symbol.trim() };
    if (scopeKind === 'sector')
      return {
        kind: 'sector',
        label: sectorLabel.trim(),
        symbols: sectorSymbols
          .split(',')
          .map((s) => s.trim())
          .filter(Boolean),
      };
    return { kind: 'global' };
  }

  function resetComposer(): void {
    editingId = null;
    name = '';
    scopeKind = 'instrument';
    symbol = '';
    sectorLabel = '';
    sectorSymbols = '';
    mandate = '';
  }

  function edit(a: AnalystView): void {
    editingId = a.id;
    name = a.name;
    mandate = a.mandate;
    scopeKind = a.scope.kind;
    if (a.scope.kind === 'instrument') symbol = a.scope.symbol;
    if (a.scope.kind === 'sector') {
      sectorLabel = a.scope.label;
      sectorSymbols = a.scope.symbols.join(', ');
    }
    composerOpen = true;
  }

  async function save(): Promise<void> {
    try {
      analysts = await invoke<AnalystView[]>('upsert_analyst', {
        draft: { id: editingId, name, scope: buildScope(), mandate, enabled: true },
      });
      resetComposer();
      composerOpen = false;
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function remove(id: string): Promise<void> {
    try {
      analysts = await invoke<AnalystView[]>('delete_analyst', { id });
      if (selectedId === id) selectedId = analysts[0]?.id ?? null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function ask(): Promise<void> {
    if (!selectedId || !question.trim()) return;
    asking = true;
    error = null;
    try {
      const answer = await invoke<AnalystAnswer>('ask_analyst', {
        question: {
          analystId: selectedId,
          providerId,
          model: model.trim(),
          question: question.trim(),
        },
      });
      answers = [answer, ...answers].slice(0, 20);
      question = '';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      asking = false;
    }
  }

  function skillLabel(a: AnalystView): string {
    if (a.skillPpm === null) return NO_VALUE;
    return `${a.skillPpm >= 0 ? '+' : ''}${(a.skillPpm / 10_000).toFixed(1)}%`;
  }

  onMount(() => {
    void load();
  });
</script>

<svelte:head><title>Analysts · PRISMATIK</title></svelte:head>

<main class="pk-analysts">
  <header>
    <div>
      <p class="eyebrow">SPECIALIST DESK</p>
      <h1>Analysts</h1>
      <p class="lede">
        Analysts you author and keep. Each one is a calibration cohort: its standing is a Brier
        skill score against the base rate, so "is this analyst any good" has an answer rather than
        an impression. Editing a mandate starts a fresh cohort — a changed mandate is a changed
        estimator.
      </p>
    </div>
    <div class="actions">
      <a href="/workspace/knowledge"><BookOpen size={13} /> Knowledge base</a>
      <button onclick={() => { resetComposer(); composerOpen = !composerOpen; }}>
        <Plus size={13} /> New analyst
      </button>
    </div>
  </header>

  {#if error}<p class="error">{error}</p>{/if}

  {#if composerOpen}
    <section class="composer">
      <div class="row">
        <label><span>Name</span><input bind:value={name} placeholder="NVDA semis specialist" /></label>
        <label>
          <span>Scope</span>
          <select bind:value={scopeKind}>
            <option value="instrument">One instrument</option>
            <option value="sector">Sector</option>
            <option value="global">All markets</option>
          </select>
        </label>
        {#if scopeKind === 'instrument'}
          <label><span>Symbol</span><input bind:value={symbol} placeholder="NVDA" /></label>
        {:else if scopeKind === 'sector'}
          <label><span>Sector label</span><input bind:value={sectorLabel} placeholder="Semis" /></label>
          <label class="wide">
            <span>Symbols</span><input bind:value={sectorSymbols} placeholder="NVDA, AMD, AVGO" />
          </label>
        {/if}
      </div>
      <label class="wide">
        <span>Mandate — what this analyst specializes in and how it should reason</span>
        <textarea rows="4" bind:value={mandate}
          placeholder="Cover NVDA with a focus on datacenter demand and supply constraints. Weigh the regime and the measured edge above narrative. Flag when the edge is inside the noise band."
        ></textarea>
      </label>
      <div class="composer-actions">
        {#if editingId}<span class="warn">Changing the mandate starts a new scoring cohort.</span>{/if}
        <button class="ghost" onclick={() => { resetComposer(); composerOpen = false; }}>Cancel</button>
        <button class="primary" onclick={() => void save()}>{editingId ? 'Save' : 'Create'}</button>
      </div>
    </section>
  {/if}

  {#if loading && analysts.length === 0}
    <NoData title="Loading analysts" compact />
  {:else if analysts.length === 0}
    <NoData
      title="No analysts yet"
      detail="Create a specialist scoped to an instrument, a sector, or the whole desk. Its forecasts are scored the same way the statistical forecaster's are, so you can compare them directly."
    />
  {:else}
    <div class="layout">
      <aside class="roster">
        {#each analysts as a (a.id)}
          <button
            class="analyst"
            class:active={selectedId === a.id}
            class:proven={a.skillPpm !== null && a.skillPpm > 0 && a.resolvedCount >= 20}
            onclick={() => (selectedId = a.id)}
          >
            <b>{a.name}</b>
            <span class="scope">{a.scopeLabel}</span>
            <span class="standing">{a.standing}</span>
            <span class="skill">{skillLabel(a)}</span>
            {#if a.untrackedSymbols.length > 0}
              <span class="uncovered">{a.coverage}</span>
            {/if}
          </button>
        {/each}
      </aside>

      <section class="conversation">
        {#if selected}
          <div class="selected-head">
            <div>
              <h2>{selected.name}</h2>
              <p>{selected.scopeLabel} · cohort {selected.cohortModel} · {selected.standing}</p>
            </div>
            <div class="head-actions">
              <button onclick={() => edit(selected)}>Edit mandate</button>
              <button class="danger" onclick={() => void remove(selected.id)} aria-label="Delete analyst">
                <Trash2 size={13} />
              </button>
            </div>
          </div>

          <div class="composer-row">
            <label>
              <span>Provider</span>
              <select bind:value={providerId}>
                <option value="">Select active session</option>
                {#each activeProviders as id (id)}<option value={id}>{id}</option>{/each}
              </select>
            </label>
            <label><span>Model</span><input bind:value={model} placeholder="model id" /></label>
            <label class="grow">
              <span>Question</span>
              <input bind:value={question} placeholder="What is the case for and against here?" />
            </label>
            <button
              class="primary"
              disabled={asking || !providerId || !model.trim() || !question.trim()}
              onclick={() => void ask()}
            >
              {#if asking}<span class="spin"><Loader2 size={13} /></span>{:else}<Send size={13} />{/if}
              Ask
            </button>
          </div>

          {#if answers.length === 0}
            <NoData
              title="No answers yet"
              detail="The analyst is given its mandate, the measured regime and edge for everything in scope, and retrieved knowledge with its gaps stated."
            />
          {:else}
            {#each answers as a, i (a.answeredAt + i)}
              <article class="answer">
                <header>
                  <b>{a.analystName}</b>
                  <span>{a.cohortModel} · {a.standing}</span>
                </header>
                <pre>{a.answer}</pre>
                <details>
                  <summary>Evidence it was given ({a.contextSections.length} sections)</summary>
                  {#each a.contextSections as section, s (s)}<pre class="context">{section}</pre>{/each}
                </details>
              </article>
            {/each}
          {/if}
        {:else}
          <NoData title="Select an analyst" compact />
        {/if}
      </section>
    </div>
  {/if}
</main>

<style>
  .pk-analysts {
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
  .actions {
    display: flex;
    height: fit-content;
    gap: 8px;
  }
  .actions button,
  .actions a {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border: 1px solid var(--p-border);
    border-radius: 6px;
    background: var(--p-panel-fill);
    color: var(--p-accent);
    cursor: pointer;
    font: 700 0.62rem var(--font-mono);
    text-decoration: none;
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

  .composer {
    display: grid;
    gap: 10px;
    margin: 16px 0;
    padding: 14px;
    border: 1px solid var(--p-accent);
    border-radius: 9px;
    background: color-mix(in srgb, var(--p-accent) 5%, transparent);
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
  .wide {
    flex: 1 1 100%;
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
    background: var(--p-panel-fill);
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
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }
  .warn {
    margin-right: auto;
    color: #fbbf24;
    font-size: 0.68rem;
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

  .layout {
    display: grid;
    grid-template-columns: 260px minmax(0, 1fr);
    gap: 12px;
    margin-top: 16px;
  }
  .roster {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .analyst {
    display: grid;
    gap: 3px;
    padding: 10px 11px;
    border: 1px solid var(--p-border);
    border-left: 2px solid var(--p-border);
    border-radius: 7px;
    background: var(--p-panel-fill);
    color: var(--p-text);
    cursor: pointer;
    text-align: left;
  }
  .analyst.active {
    border-left-color: var(--p-accent);
    background: color-mix(in srgb, var(--p-accent) 8%, transparent);
  }
  /* Only a scored, positive cohort earns the accent. */
  .analyst.proven {
    border-left-color: var(--p-up);
  }
  .analyst b {
    font-size: 0.78rem;
  }
  .analyst .scope {
    color: var(--p-accent);
    font: 600 0.62rem var(--font-mono);
  }
  .analyst .standing,
  .analyst .skill {
    color: var(--p-dim);
    font-size: 0.62rem;
    line-height: 1.4;
  }
  .analyst .skill {
    font-family: var(--font-mono);
  }
  /* An analyst scoped to an untracked symbol still works, but reasons from
     prose alone. Saying so beats letting it look like the scored ones. */
  .analyst .uncovered {
    color: #ffb84d;
    font-size: 0.6rem;
    line-height: 1.4;
  }
  .analyst.proven .skill {
    color: var(--p-up);
  }

  .conversation {
    display: grid;
    align-content: start;
    gap: 10px;
    min-width: 0;
  }
  .selected-head {
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    border: 1px solid var(--p-border);
    border-radius: 9px;
    background: var(--p-panel-fill);
  }
  .selected-head h2 {
    margin: 0 0 4px;
    font-size: 1rem;
  }
  .selected-head p {
    margin: 0;
    color: var(--p-dim);
    font: 0.66rem var(--font-mono);
  }
  .head-actions {
    display: flex;
    gap: 6px;
  }
  .head-actions button {
    padding: 6px 10px;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
    font: 700 0.6rem var(--font-mono);
  }
  .head-actions .danger:hover {
    border-color: var(--p-down);
    color: var(--p-down);
  }
  .composer-row {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 8px;
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
    border: 1px solid var(--p-border);
    border-radius: 9px;
    background: var(--p-panel-fill);
    overflow: hidden;
  }
  .answer header {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--p-border);
  }
  .answer header span {
    color: var(--p-dim);
    font: 0.62rem var(--font-mono);
  }
  .answer pre {
    margin: 0;
    padding: 14px;
    font: inherit;
    font-size: 0.78rem;
    line-height: 1.6;
    white-space: pre-wrap;
  }
  .answer details {
    padding: 10px 14px;
    border-top: 1px solid var(--p-border);
    color: var(--p-dim);
    font-size: 0.66rem;
  }
  .answer .context {
    padding: 8px 0;
    color: var(--p-dim);
    font-size: 0.64rem;
  }

  @media (max-width: 1000px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }
</style>
