<script lang="ts">
  // Live session control. Paper simulator by default; Alpaca routing optional.
  // The live endpoint is reachable only when the server was started with the
  // acknowledgment phrase in its environment AND the operator retypes the same
  // phrase here. The UI never weakens what the server enforces.
  import { onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';
  import {
    startSession, stopSession, sessionStatus, sessionSocket, fmtUsd, fmtNum
  } from '../api';
  import type { JournalEvent, SessionStatus, StrategySpec } from '../types';

  export let symbol: string;
  export let granularityS: number;
  export let strategy: StrategySpec;
  export let equity: number;
  export let alpacaConfigured = false;
  export let liveUnlocked = false;
  export let ackPhrase = '';

  let execution: 'paper' | 'alpaca' = 'paper';
  let wantLive = false;
  let typedAck = '';
  let sessionId: string | null = null;
  let mode = '';
  let status: SessionStatus | null = null;
  let events: JournalEvent[] = [];
  let error = '';
  let busy = false;
  let ws: WebSocket | null = null;
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  $: liveBlocked = wantLive && (!liveUnlocked || typedAck !== ackPhrase);
  $: startDisabled = busy || (execution === 'alpaca' && !alpacaConfigured) || liveBlocked;

  async function start(): Promise<void> {
    busy = true;
    error = '';
    try {
      const resp = await startSession({
        symbol,
        granularity_s: granularityS,
        strategy,
        initial_equity_usd: equity,
        window_bars: 200,
        execution,
        live: execution === 'alpaca' && wantLive,
        live_confirm: typedAck
      });
      sessionId = resp.session_id;
      mode = resp.mode;
      events = [];
      ws = sessionSocket(sessionId);
      ws.onmessage = (m) => {
        try {
          events = [...events.slice(-149), JSON.parse(m.data)];
        } catch { /* non-JSON frame, ignore */ }
      };
      pollTimer = setInterval(refresh, 4000);
      await refresh();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function refresh(): Promise<void> {
    if (!sessionId) return;
    try {
      status = await sessionStatus(sessionId);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function stop(): Promise<void> {
    if (!sessionId) return;
    busy = true;
    try {
      await stopSession(sessionId);
      await refresh();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
      teardown();
    }
  }

  function teardown(): void {
    ws?.close();
    ws = null;
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = null;
  }

  onDestroy(teardown);
</script>

<section class="panel">
  <h3>Live session</h3>

  {#if !sessionId}
    <div class="controls">
      <label class="field">
        <span>Execution</span>
        <select bind:value={execution}>
          <option value="paper">Paper simulator (built in)</option>
          <option value="alpaca" disabled={!alpacaConfigured}>
            Alpaca {alpacaConfigured ? '' : '(set ALPACA_KEY_ID / ALPACA_SECRET_KEY)'}
          </option>
        </select>
      </label>

      {#if execution === 'alpaca'}
        <label class="check" transition:fade={{ duration: 150 }}>
          <input type="checkbox" bind:checked={wantLive} />
          <span>Route to the <strong class="neg">live</strong> endpoint instead of Alpaca paper</span>
        </label>
        {#if wantLive}
          <div class="gate" transition:fade={{ duration: 150 }}>
            {#if !liveUnlocked}
              <p class="neg mono small">
                Server gate closed. Start the server with PRISMATIK_LIVE_TRADING set to the
                acknowledgment phrase to unlock live routing.
              </p>
            {:else}
              <p class="mono small">Type the acknowledgment phrase to confirm this session:</p>
              <input
                type="text"
                bind:value={typedAck}
                placeholder={ackPhrase}
                aria-label="Live trading acknowledgment phrase"
                autocomplete="off"
                spellcheck="false"
              />
            {/if}
          </div>
        {/if}
      {/if}

      <button class="btn btn-primary" on:click={start} disabled={startDisabled}>
        {busy ? 'Starting…' : 'Start session'}
      </button>
      <p class="mono small dim">
        Sessions stream the real Coinbase ticker, journal every event, and halt at the
        drawdown kill switch{execution === 'alpaca' ? ' or on any ambiguous order' : ''}.
      </p>
    </div>
  {:else}
    <div class="running" in:fade={{ duration: 200 }}>
      <div class="statusline mono">
        <span class="badge" class:live={mode === 'alpaca_LIVE'}>
          {mode === 'alpaca_LIVE' ? 'ALPACA LIVE' : mode === 'paper' ? 'PAPER' : 'ALPACA PAPER'}
        </span>
        <span>{symbol}</span>
        {#if status}
          <span>eq {fmtUsd(status.equity)}</span>
          <span>px {fmtNum(status.last_price, 2)}</span>
          <span>{status.halted ? 'HALTED' : status.running ? 'running' : 'stopped'}</span>
        {/if}
      </div>
      <div class="feed mono" role="log" aria-live="polite">
        {#if events.length === 0}
          <p class="dim">Waiting for the first tick…</p>
        {/if}
        {#each events as ev}
          <div class="ev">
            <span class="dim">{ev.ts ?? ''}</span>
            <span class:neg={ev.event === 'kill_switch' || ev.event === 'broker_halt'}
                  class:pos={ev.event === 'fill'}>{ev.event}</span>
            {#if ev.event === 'fill'}<span>Δ {fmtNum(Number(ev.delta_units), 6)} @ {fmtNum(Number(ev.price), 2)}</span>{/if}
            {#if ev.event === 'bar_close'}<span>px {fmtNum(Number(ev.price), 2)} eq {fmtUsd(Number(ev.equity))}</span>{/if}
          </div>
        {/each}
      </div>
      <button class="btn btn-quiet" on:click={stop} disabled={busy}>Stop session</button>
    </div>
  {/if}

  {#if error}
    <p class="neg mono small" role="alert" transition:fade>{error}</p>
  {/if}
</section>

<style>
  .panel {
    border: 1px solid var(--gilt-faint);
    border-radius: var(--radius);
    padding: 16px 18px;
    background: var(--ink-2);
  }
  h3 { font-family: var(--display); font-weight: 400; font-size: 16px; margin: 0 0 12px; color: var(--gilt); }
  .controls { display: grid; gap: 12px; }
  .field span {
    display: block; font-family: var(--mono); font-size: 11px;
    letter-spacing: 0.18em; text-transform: uppercase; color: var(--porcelain-2);
    margin-bottom: 5px;
  }
  .check { display: flex; align-items: center; gap: 9px; font-size: 13.5px; }
  .check input { width: auto; }
  .gate { border-left: 2px solid var(--oxblood); padding-left: 12px; display: grid; gap: 8px; }
  .small { font-size: 12px; }
  .dim { color: var(--porcelain-2); }
  .statusline { display: flex; gap: 14px; align-items: center; flex-wrap: wrap; font-size: 12.5px; margin-bottom: 10px; }
  .badge {
    border: 1px solid var(--gilt-dim); color: var(--gilt);
    padding: 3px 8px; border-radius: var(--radius); letter-spacing: 0.14em; font-size: 11px;
  }
  .badge.live { border-color: var(--oxblood); color: var(--oxblood); }
  .feed {
    max-height: 220px; overflow-y: auto; font-size: 12px;
    border: 1px solid var(--gilt-faint); border-radius: var(--radius);
    padding: 10px 12px; margin-bottom: 12px; background: var(--ink);
  }
  .ev { display: flex; gap: 10px; padding: 2px 0; flex-wrap: wrap; }
</style>
