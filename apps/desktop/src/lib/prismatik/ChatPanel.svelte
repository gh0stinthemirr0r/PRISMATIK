<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke, isTauri } from '@tauri-apps/api/core';
  import { Send, Trash2, ChevronDown, Loader2, MessageSquare, X } from 'lucide-svelte';

  interface ChatMessage {
    role: string;
    content: string;
    timestamp: number;
    provider?: string;
    model?: string;
    tokens?: number;
  }

  interface ChatState {
    messages: ChatMessage[];
    activeProvider: string | null;
    activeModel: string | null;
  }

  let messages = $state<ChatMessage[]>([]);
  let input = $state('');
  let sending = $state(false);
  let error = $state('');
  let activeProvider = $state<string | null>(null);
  let activeModel = $state<string | null>(null);
  let providers = $state<string[]>([]);
  let showProviderSelect = $state(false);
  let scrollEl: HTMLDivElement;

  const PROVIDER_LABELS: Record<string, string> = {
    openai: 'ChatGPT',
    anthropic: 'Claude',
    google: 'Gemini',
    local: 'Local (Ollama/LM Studio)',
    xai: 'Grok',
    deepseek: 'DeepSeek',
    groq: 'Groq',
    cohere: 'Cohere',
    openrouter: 'OpenRouter',
    together: 'Together',
    fireworks: 'Fireworks',
  };

  function scrollToBottom() {
    if (scrollEl) requestAnimationFrame(() => scrollEl.scrollTop = scrollEl.scrollHeight);
  }

  async function loadState() {
    if (!isTauri()) return;
    try {
      const state = await invoke<ChatState>('chat_get_state');
      messages = state.messages;
      activeProvider = state.activeProvider;
      activeModel = state.activeModel;
      // get connected providers
      const rows = await invoke<Array<{ providerId: string; active: boolean }>>('model_runtime_status');
      providers = rows.filter(r => r.active).map(r => r.providerId);
      if (!activeProvider && providers.length > 0) {
        activeProvider = providers[0];
      }
      scrollToBottom();
    } catch (e) {
      error = String(e);
    }
  }

  async function send() {
    const text = input.trim();
    if (!text || sending) return;
    input = '';
    error = '';
    sending = true;

    // optimistic user message
    messages = [...messages, {
      role: 'user',
      content: text,
      timestamp: Date.now(),
    }];
    scrollToBottom();

    try {
      const resp = await invoke<{ reply: ChatMessage; state: ChatState }>('chat_send', {
        message: text,
        providerId: activeProvider,
        model: activeModel,
      });
      messages = resp.state.messages;
      activeProvider = resp.state.activeProvider;
      activeModel = resp.state.activeModel;
      scrollToBottom();
    } catch (e) {
      error = String(e);
      // remove optimistic message on failure
      messages = messages.slice(0, -1);
    } finally {
      sending = false;
    }
  }

  async function clear() {
    if (!isTauri()) return;
    try {
      const state = await invoke<ChatState>('chat_clear');
      messages = state.messages;
      error = '';
    } catch (e) {
      error = String(e);
    }
  }

  async function selectProvider(id: string) {
    activeProvider = id;
    showProviderSelect = false;
    if (isTauri()) {
      try {
        await invoke('chat_set_active_provider', { providerId: id, model: null });
      } catch {}
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  function formatTime(ts: number): string {
    return new Date(ts).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
  }

  function formatContent(content: string): string {
    // basic markdown: bold, italic, code blocks, inline code
    return content
      .replace(/```[\s\S]*?```/g, (m) => `<pre class="pk-code-block">${m.slice(3, -3).trim()}</pre>`)
      .replace(/`([^`]+)`/g, '<code class="pk-inline-code">$1</code>')
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      .replace(/\n/g, '<br>');
  }

  onMount(() => { void loadState(); });
</script>

<div class="pk-chat">
  <div class="pk-chat-header">
    <div class="pk-chat-title">
      <MessageSquare size={14} />
      <span>PRISMATIK Command</span>
    </div>
    <div class="pk-chat-controls">
      {#if activeProvider}
        <button class="pk-chat-provider" onclick={() => showProviderSelect = !showProviderSelect}>
          {PROVIDER_LABELS[activeProvider] ?? activeProvider}
          {#if activeModel}<span class="pk-chat-model">{activeModel}</span>{/if}
          <ChevronDown size={12} />
        </button>
      {/if}
      <button class="pk-chat-icon-btn" onclick={clear} title="Clear conversation">
        <Trash2 size={13} />
      </button>
    </div>
  </div>

  {#if showProviderSelect}
    <div class="pk-chat-provider-dropdown">
      {#each providers as pid}
        <button class:active={pid === activeProvider} onclick={() => selectProvider(pid)}>
          {PROVIDER_LABELS[pid] ?? pid}
        </button>
      {/each}
    </div>
  {/if}

  <div class="pk-chat-messages" bind:this={scrollEl}>
    {#if messages.length === 0}
      <div class="pk-chat-empty">
        <p>Ask anything about markets, instruments, or your portfolio.</p>
        <p class="pk-chat-empty-hint">Connected: {providers.length} provider{providers.length !== 1 ? 's' : ''}</p>
      </div>
    {/if}
    {#each messages as msg}
      <div class="pk-chat-msg" class:user={msg.role === 'user'} class:assistant={msg.role === 'assistant'}>
        <div class="pk-chat-msg-meta">
          {#if msg.role === 'user'}You{:else}PRISMATIK{/if}
          <span class="pk-chat-msg-time">{formatTime(msg.timestamp)}</span>
          {#if msg.tokens}<span class="pk-chat-msg-tokens">{msg.tokens} tok</span>{/if}
        </div>
        <div class="pk-chat-msg-body">{@html formatContent(msg.content)}</div>
      </div>
    {/each}
    {#if sending}
      <div class="pk-chat-msg assistant pk-chat-thinking">
        <div class="pk-chat-msg-meta">PRISMATIK</div>
        <div class="pk-chat-msg-body"><Loader2 size={14} class="pk-spin" /> Thinking…</div>
      </div>
    {/if}
  </div>

  {#if error}
    <div class="pk-chat-error">
      <span>{error}</span>
      <button onclick={() => error = ''}><X size={12} /></button>
    </div>
  {/if}

  <div class="pk-chat-input-row">
    <textarea
      class="pk-chat-input"
      bind:value={input}
      {onkeydown}
      placeholder={activeProvider ? `Message ${PROVIDER_LABELS[activeProvider] ?? activeProvider}…` : 'Connect a model provider first…'}
      disabled={!activeProvider || sending}
      rows="1"
    ></textarea>
    <button class="pk-chat-send" onclick={send} disabled={!input.trim() || sending || !activeProvider}>
      <Send size={14} />
    </button>
  </div>
</div>

<style>
  .pk-chat {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--p-surface0);
    border-left: 1px solid var(--p-border);
    font-family: var(--font-sans);
    font-size: 0.8125rem;
    color: var(--p-text);
  }
  .pk-chat-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--p-border);
    background: var(--p-surface1);
    backdrop-filter: blur(12px);
  }
  .pk-chat-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    font-size: 0.75rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--p-text);
  }
  .pk-chat-controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .pk-chat-provider {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border: 1px solid var(--p-border);
    border-radius: 4px;
    background: var(--p-surface0);
    color: var(--p-accent);
    font-size: 0.6875rem;
    font-family: var(--font-mono);
    cursor: pointer;
    transition: border-color 0.15s;
  }
  .pk-chat-provider:hover { border-color: var(--p-accent); }
  .pk-chat-model {
    color: var(--p-text-dim);
    font-size: 0.625rem;
  }
  .pk-chat-icon-btn {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--p-text-dim);
    cursor: pointer;
    transition: color 0.15s, background 0.15s;
  }
  .pk-chat-icon-btn:hover { color: var(--p-text); background: var(--p-surface2); }
  .pk-chat-provider-dropdown {
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--p-border);
    background: var(--p-surface1);
    max-height: 200px;
    overflow-y: auto;
  }
  .pk-chat-provider-dropdown button {
    padding: 8px 14px;
    border: none;
    background: transparent;
    color: var(--p-text);
    text-align: left;
    font-size: 0.75rem;
    font-family: var(--font-mono);
    cursor: pointer;
    transition: background 0.1s;
  }
  .pk-chat-provider-dropdown button:hover { background: var(--p-surface2); }
  .pk-chat-provider-dropdown button.active { color: var(--p-accent); background: var(--p-surface2); }
  .pk-chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .pk-chat-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--p-text-dim);
    text-align: center;
    gap: 8px;
  }
  .pk-chat-empty p { margin: 0; }
  .pk-chat-empty-hint { font-size: 0.6875rem; font-family: var(--font-mono); }
  .pk-chat-msg {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-width: 92%;
  }
  .pk-chat-msg.user { align-self: flex-end; }
  .pk-chat-msg.assistant { align-self: flex-start; }
  .pk-chat-msg-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.625rem;
    font-family: var(--font-mono);
    color: var(--p-text-dim);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .pk-chat-msg.user .pk-chat-msg-meta { justify-content: flex-end; }
  .pk-chat-msg-time { opacity: 0.5; }
  .pk-chat-msg-tokens { opacity: 0.4; }
  .pk-chat-msg-body {
    padding: 10px 14px;
    border-radius: 8px;
    line-height: 1.55;
    word-break: break-word;
  }
  .pk-chat-msg.user .pk-chat-msg-body {
    background: var(--p-accent);
    color: #000;
    border-bottom-right-radius: 2px;
  }
  .pk-chat-msg.assistant .pk-chat-msg-body {
    background: var(--p-surface2);
    color: var(--p-text);
    border-bottom-left-radius: 2px;
  }
  .pk-chat-msg-body :global(.pk-code-block) {
    display: block;
    padding: 8px 10px;
    margin: 6px 0;
    background: var(--p-surface0);
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.75rem;
    overflow-x: auto;
    white-space: pre;
  }
  .pk-chat-msg-body :global(.pk-inline-code) {
    padding: 1px 5px;
    background: var(--p-surface0);
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 0.8em;
  }
  .pk-chat-thinking .pk-chat-msg-body {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--p-text-dim);
  }
  .pk-chat-error {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 14px;
    background: rgba(239, 68, 68, 0.1);
    border-top: 1px solid rgba(239, 68, 68, 0.2);
    color: #f87171;
    font-size: 0.6875rem;
    font-family: var(--font-mono);
  }
  .pk-chat-error button {
    border: none;
    background: transparent;
    color: #f87171;
    cursor: pointer;
    padding: 2px;
  }
  .pk-chat-input-row {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    padding: 10px 14px;
    border-top: 1px solid var(--p-border);
    background: var(--p-surface1);
  }
  .pk-chat-input {
    flex: 1;
    resize: none;
    border: 1px solid var(--p-border);
    border-radius: 6px;
    padding: 10px 12px;
    background: var(--p-surface0);
    color: var(--p-text);
    font-family: var(--font-sans);
    font-size: 0.8125rem;
    line-height: 1.4;
    outline: none;
    min-height: 38px;
    max-height: 120px;
    transition: border-color 0.15s;
  }
  .pk-chat-input:focus { border-color: var(--p-accent); }
  .pk-chat-input::placeholder { color: var(--p-text-dim); }
  .pk-chat-input:disabled { opacity: 0.4; }
  .pk-chat-send {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border: none;
    border-radius: 6px;
    background: var(--p-accent);
    color: #000;
    cursor: pointer;
    transition: opacity 0.15s;
    flex-shrink: 0;
  }
  .pk-chat-send:disabled { opacity: 0.3; cursor: default; }
  .pk-chat-send:not(:disabled):hover { opacity: 0.85; }
  :global(.pk-spin) { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
