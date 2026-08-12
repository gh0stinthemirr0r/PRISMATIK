<script lang="ts">
  /**
   * The agent room — the discussion surface inside PRISMATIK COMMAND.
   *
   * The value here is not that agents answer; three surfaces already do that.
   * It is that they answer *each other*, in one attributed transcript, so a
   * bear case is a response to the bull's actual argument rather than a
   * paragraph written in isolation. Each speaker carries its own measured
   * standing, so a confident specialist with no track record reads as exactly
   * that.
   */
  import { onMount } from 'svelte';
  import { Send, Loader2, X, Plus, Users } from 'lucide-svelte';
  import NoData from './NoData.svelte';
  import { market } from './market.svelte';

  type Speaker =
    | { kind: 'operator' }
    | { kind: 'analyst'; id: string; name: string }
    | { kind: 'critic'; role: string; name: string }
    | { kind: 'desk' };

  interface Message {
    id: string;
    speaker: Speaker;
    body: string;
    at: string;
    standing: string | null;
  }

  interface Participant {
    speaker: Speaker;
    mandate: string;
  }

  interface Room {
    id: string;
    title: string;
    subjects: string[];
    participants: Participant[];
    messages: Message[];
    createdAt: string;
    nextTurn: number;
  }

  interface AnalystView {
    id: string;
    name: string;
    scopeLabel: string;
    standing: string;
  }

  let rooms = $state<Room[]>([]);
  let analysts = $state<AnalystView[]>([]);
  let critics = $state<[string, string, string][]>([]);
  let activeProviders = $state<string[]>([]);

  let roomId = $state<string | null>(null);
  let providerId = $state('');
  let model = $state('');
  let draft = $state('');
  let turns = $state(2);
  let busy = $state(false);
  let error = $state<string | null>(null);

  // convening
  let convening = $state(false);
  let title = $state('');
  let subjects = $state('');
  let pickedAnalysts = $state<string[]>([]);
  let pickedCritics = $state<string[]>(['bear', 'regime_critic']);

  const room = $derived(rooms.find((r) => r.id === roomId) ?? null);

  async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke: call, isTauri } = await import('@tauri-apps/api/core');
    if (!isTauri()) throw new Error('desktop runtime required');
    return call<T>(command, args);
  }

  async function load(): Promise<void> {
    try {
      const [r, a, c, p] = await Promise.all([
        invoke<Room[]>('list_rooms'),
        invoke<AnalystView[]>('list_analysts'),
        invoke<[string, string, string][]>('list_critic_roles'),
        invoke<Array<{ providerId: string; active: boolean }>>('model_runtime_status'),
      ]);
      rooms = r;
      analysts = a;
      critics = c;
      activeProviders = p.filter((x) => x.active).map((x) => x.providerId);
      if (!providerId) providerId = activeProviders[0] ?? '';
      if (!roomId && rooms.length > 0) roomId = rooms[0].id;
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function toggle(list: string[], value: string): string[] {
    return list.includes(value) ? list.filter((v) => v !== value) : [...list, value];
  }

  async function convene(): Promise<void> {
    try {
      const created = await invoke<Room>('open_room', {
        draft: {
          title: title.trim() || (market.selected?.symbol ?? 'Discussion'),
          subjects: subjects
            .split(',')
            .map((s) => s.trim())
            .filter(Boolean),
          analystIds: pickedAnalysts,
          criticRoles: pickedCritics,
        },
      });
      rooms = [...rooms, created];
      roomId = created.id;
      convening = false;
      title = '';
      subjects = '';
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function send(): Promise<void> {
    if (!roomId || !draft.trim()) return;
    busy = true;
    error = null;
    try {
      const updated = await invoke<Room>('say', { roomId, body: draft.trim() });
      rooms = rooms.map((r) => (r.id === updated.id ? updated : r));
      draft = '';
      // Posting a question and then letting the room answer is one action from
      // the operator's point of view.
      const advanced = await invoke<Room>('advance_room', {
        request: { roomId, providerId, model: model.trim(), turns },
      });
      rooms = rooms.map((r) => (r.id === advanced.id ? advanced : r));
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function advance(): Promise<void> {
    if (!roomId) return;
    busy = true;
    error = null;
    try {
      const advanced = await invoke<Room>('advance_room', {
        request: { roomId, providerId, model: model.trim(), turns },
      });
      rooms = rooms.map((r) => (r.id === advanced.id ? advanced : r));
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function close(id: string): Promise<void> {
    try {
      rooms = await invoke<Room[]>('close_room', { id });
      if (roomId === id) roomId = rooms[0]?.id ?? null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function speakerName(s: Speaker): string {
    if (s.kind === 'operator') return 'You';
    if (s.kind === 'desk') return 'PRISMATIK';
    return s.name;
  }

  onMount(() => {
    void load();
  });
</script>

<div class="room">
  <div class="room-head">
    <div class="tabs">
      {#each rooms as r (r.id)}
        <button class="tab" class:active={roomId === r.id} onclick={() => (roomId = r.id)}>
          {r.title}
          <span class="count">{r.participants.length}</span>
        </button>
      {/each}
      <button class="tab new" onclick={() => (convening = !convening)}>
        <Plus size={12} /> Convene
      </button>
    </div>
    {#if room}
      <button class="close" aria-label="Close room" onclick={() => void close(room.id)}>
        <X size={13} />
      </button>
    {/if}
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  {#if convening}
    <div class="convene">
      <div class="row">
        <label><span>Title</span><input bind:value={title} placeholder="NVDA into earnings" /></label>
        <label>
          <span>Subjects</span>
          <input bind:value={subjects} placeholder="NVDA, AMD" />
        </label>
      </div>
      <div class="picker">
        <span class="picker-label">Critics</span>
        {#each critics as [id, name] (id)}
          <button
            class="chip"
            class:on={pickedCritics.includes(id)}
            onclick={() => (pickedCritics = toggle(pickedCritics, id))}
          >{name}</button>
        {/each}
      </div>
      <div class="picker">
        <span class="picker-label">Analysts</span>
        {#if analysts.length === 0}
          <span class="hint">None yet — create one under Analysts.</span>
        {:else}
          {#each analysts as a (a.id)}
            <button
              class="chip"
              class:on={pickedAnalysts.includes(a.id)}
              title={a.standing}
              onclick={() => (pickedAnalysts = toggle(pickedAnalysts, a.id))}
            >{a.name}</button>
          {/each}
        {/if}
      </div>
      <div class="convene-actions">
        <button class="ghost" onclick={() => (convening = false)}>Cancel</button>
        <button class="primary" onclick={() => void convene()}><Users size={12} /> Open room</button>
      </div>
    </div>
  {/if}

  {#if room}
    <div class="participants">
      {#each room.participants as p, i (i)}
        <span class="participant" class:next={i === room.nextTurn % room.participants.length}>
          {speakerName(p.speaker)}
        </span>
      {/each}
    </div>

    <div class="transcript">
      {#if room.messages.length === 0}
        <NoData
          title="Nothing said yet"
          detail="Ask a question. Participants take turns and each one sees what the others said."
          compact
        />
      {:else}
        {#each room.messages as m (m.id)}
          <article class="msg" data-kind={m.speaker.kind}>
            <header>
              <b>{speakerName(m.speaker)}</b>
              {#if m.standing}<span class="standing">{m.standing}</span>{/if}
            </header>
            <p>{m.body}</p>
          </article>
        {/each}
      {/if}
    </div>

    <div class="composer">
      <div class="controls">
        <select bind:value={providerId} aria-label="Provider">
          <option value="">provider</option>
          {#each activeProviders as id (id)}<option value={id}>{id}</option>{/each}
        </select>
        <input class="model" bind:value={model} placeholder="model id" />
        <select bind:value={turns} aria-label="Turns per round">
          {#each [1, 2, 3, 4] as n (n)}<option value={n}>{n} turn{n > 1 ? 's' : ''}</option>{/each}
        </select>
        <button class="ghost" disabled={busy || !providerId || !model.trim()} onclick={() => void advance()}>
          Continue
        </button>
      </div>
      <div class="say">
        <input
          bind:value={draft}
          placeholder="Ask the room…"
          onkeydown={(e) => e.key === 'Enter' && !busy && send()}
        />
        <button class="primary" disabled={busy || !draft.trim() || !providerId || !model.trim()} onclick={() => void send()}>
          {#if busy}<span class="spin"><Loader2 size={13} /></span>{:else}<Send size={13} />{/if}
        </button>
      </div>
    </div>
  {:else if !convening}
    <NoData
      title="No room open"
      detail="Convene a room to put specialists and critics in one thread, where they answer each other rather than you individually."
    />
  {/if}
</div>

<style>
  .room {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    gap: 8px;
  }
  .room-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .tabs {
    display: flex;
    flex: 1;
    flex-wrap: wrap;
    gap: 4px;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 9px;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
    font: 600 var(--fz-sm) var(--font-mono);
  }
  .tab.active {
    border-color: var(--p-accent);
    background: color-mix(in srgb, var(--p-accent) 10%, transparent);
    color: var(--p-accent);
  }
  .tab .count {
    padding: 0 4px;
    border-radius: 3px;
    background: var(--p-surface2);
    font-size: 8px;
  }
  .close {
    border: none;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
  }
  .error {
    margin: 0;
    color: var(--p-down);
    font-size: var(--fz-sm);
  }

  .convene {
    display: grid;
    gap: 8px;
    padding: 10px;
    border: 1px solid var(--p-accent);
    border-radius: 7px;
    background: color-mix(in srgb, var(--p-accent) 6%, transparent);
  }
  .row {
    display: flex;
    gap: 8px;
  }
  label {
    display: grid;
    flex: 1;
    gap: 3px;
  }
  label > span,
  .picker-label {
    color: var(--p-dim);
    font: 600 8px var(--font-mono);
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  input,
  select {
    padding: 6px 8px;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    background: var(--p-surface2);
    color: var(--p-text);
    font-family: var(--font-mono);
    font-size: var(--fz-sm);
  }
  .picker {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 5px;
  }
  .chip {
    padding: 3px 8px;
    border: 1px solid var(--p-border);
    border-radius: 12px;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
    font: 600 var(--fz-sm) var(--font-mono);
  }
  .chip.on {
    border-color: var(--p-accent);
    background: color-mix(in srgb, var(--p-accent) 14%, transparent);
    color: var(--p-accent);
  }
  .hint {
    color: var(--p-dim);
    font-size: var(--fz-sm);
  }
  .convene-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }

  .participants {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  .participant {
    padding: 2px 7px;
    border: 1px solid var(--p-border);
    border-radius: 10px;
    color: var(--p-dim);
    font: 600 8.5px var(--font-mono);
  }
  /* Whose turn is next, so the round order is legible before it runs. */
  .participant.next {
    border-color: var(--p-accent);
    color: var(--p-accent);
  }

  .transcript {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    gap: 7px;
    overflow-y: auto;
    padding-right: 3px;
  }
  .msg {
    padding: 8px 10px;
    border: 1px solid var(--p-border);
    border-left: 2px solid var(--p-border);
    border-radius: 6px;
    background: var(--p-panel-fill);
  }
  .msg[data-kind='operator'] {
    border-left-color: var(--p-accent);
  }
  .msg[data-kind='critic'] {
    border-left-color: #fbbf24;
  }
  .msg[data-kind='analyst'] {
    border-left-color: var(--p-accent2);
  }
  .msg header {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 4px;
  }
  .msg header b {
    font-size: var(--fz-sm);
  }
  .msg .standing {
    color: var(--p-dim);
    font-size: 8.5px;
  }
  .msg p {
    margin: 0;
    font-size: var(--fz-sm);
    line-height: 1.55;
    white-space: pre-wrap;
  }

  .composer {
    display: grid;
    flex: none;
    gap: 6px;
  }
  .controls,
  .say {
    display: flex;
    gap: 6px;
  }
  .say input {
    flex: 1;
  }
  .model {
    width: 110px;
  }
  button.primary,
  button.ghost {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 6px 11px;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    background: transparent;
    color: var(--p-dim);
    cursor: pointer;
    font: 700 var(--fz-sm) var(--font-mono);
  }
  button.primary {
    border-color: var(--p-accent);
    background: color-mix(in srgb, var(--p-accent) 12%, transparent);
    color: var(--p-accent);
  }
  button:disabled {
    cursor: default;
    opacity: 0.5;
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
</style>
