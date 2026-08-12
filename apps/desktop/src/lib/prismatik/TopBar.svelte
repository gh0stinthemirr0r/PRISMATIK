<script lang="ts">
  import { onMount } from 'svelte';
  import { Command, Minus, MessageSquare, Search, Settings, Square, Wifi, X } from 'lucide-svelte';
  import { aesthetics } from './aesthetics.svelte';
  import { platform } from './platform.svelte';
  import { market, NO_VALUE } from './market.svelte';
  import NoMarketInputs from './NoMarketInputs.svelte';
  import { chat } from './chat.svelte';
  import { command } from './command.svelte';

  let now = $state(Date.now());

  // Native window chrome: on Windows and Linux the window runs decorationless
  // and PRISMATIK draws its own controls. macOS keeps its traffic lights (see
  // platform.svelte.ts), so drawing ours would duplicate them on the wrong
  // side. In plain browser dev they stay hidden entirely.
  const inTauri =
    typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  const showWindowControls = $derived(
    inTauri && platform.resolved && !platform.usesSystemWindowControls,
  );
  let maximized = $state(false);

  async function tauriWindow() {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    return getCurrentWindow();
  }

  async function minimize() {
    try {
      await (await tauriWindow()).minimize();
    } catch {}
  }

  async function toggleMaximize() {
    try {
      const win = await tauriWindow();
      await win.toggleMaximize();
      maximized = await win.isMaximized();
    } catch {}
  }

  async function closeWindow() {
    try {
      await (await tauriWindow()).close();
    } catch {}
  }

  onMount(() => {
    const clock = setInterval(() => (now = Date.now()), 1000);
    return () => {
      clearInterval(clock);
    };
  });

  interface Desk {
    city: string;
    tz: string;
    openMin: number; // minutes after midnight, local
    closeMin: number;
  }

  const DESKS: Desk[] = [
    { city: 'NYC', tz: 'America/New_York', openMin: 9 * 60 + 30, closeMin: 16 * 60 },
    { city: 'LDN', tz: 'Europe/London', openMin: 8 * 60, closeMin: 16 * 60 + 30 },
    { city: 'TYO', tz: 'Asia/Tokyo', openMin: 9 * 60, closeMin: 15 * 60 },
  ];

  const timeFmt = new Map(
    DESKS.map((d) => [
      d.tz,
      new Intl.DateTimeFormat('en-GB', {
        timeZone: d.tz,
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        hour12: false,
      }),
    ]),
  );

  const probeFmt = new Map(
    DESKS.map((d) => [
      d.tz,
      new Intl.DateTimeFormat('en-US', {
        timeZone: d.tz,
        weekday: 'short',
        hour: '2-digit',
        minute: '2-digit',
        hour12: false,
      }),
    ]),
  );

  function localTime(tz: string): string {
    return timeFmt.get(tz)!.format(now);
  }

  function isOpen(tz: string, openMin: number, closeMin: number): boolean {
    const parts = probeFmt.get(tz)!.formatToParts(now);
    const get = (type: string) => parts.find((p) => p.type === type)?.value ?? '';
    const weekday = get('weekday');
    if (weekday === 'Sat' || weekday === 'Sun') return false;
    const minutes = parseInt(get('hour'), 10) * 60 + parseInt(get('minute'), 10);
    return minutes >= openMin && minutes < closeMin;
  }
</script>

<header
  class="pk-topbar"
  class:pk-topbar-inset={platform.usesSystemWindowControls}
  data-tauri-drag-region
>
  <div class="pk-logo" data-tauri-drag-region>
    <svg width="26" height="26" viewBox="0 0 26 26" fill="none" aria-hidden="true">
      <path d="M13 2 L24 22 L2 22 Z" fill="none" stroke="var(--p-accent)" stroke-width="1.6" />
      <path d="M13 8 L19 19 L7 19 Z" fill="var(--p-accent2)" opacity="0.55" />
      <path d="M13 2 L13 22" stroke="var(--p-accent)" stroke-width="0.8" opacity="0.6" />
    </svg>
    <div>
      <div class="pk-logo-text">PRISMATIK</div>
      <div class="pk-logo-sub">PRISMATIC TRADING SURFACE</div>
    </div>
  </div>

  <div class="pk-clocks">
    {#each DESKS as desk (desk.tz)}
      <div class="pk-clock pk-mono" class:open={isOpen(desk.tz, desk.openMin, desk.closeMin)}>
        <span class="dot"></span>
        <span class="pk-clock-city">{desk.city}</span>
        <span>{localTime(desk.tz)}</span>
      </div>
    {/each}
  </div>

  <div
    class="pk-latency pk-mono"
    title="Measured round-trip of the last quote poll: IPC plus provider time. Not exchange latency, which nothing here observes."
  >
    <Wifi size={13} />
    <span>{market.lastPollMs === null ? `POLL ${NO_VALUE}` : `POLL ${market.lastPollMs}ms`}</span>
  </div>

  <NoMarketInputs
    mode={market.feedMode}
    providers={market.feedProviders}
    message={market.feedMessage}
    quoted={market.quotedCount}
    tracked={market.tracked.length}
  />

  <button class="pk-command-trigger" onclick={() => (command.open = true)} title="Open command deck">
    <Search size={13} /><span>Search systems</span><kbd>{platform.modifierLabel}K</kbd>
  </button>

  <button
    class="pk-icon-btn"
    class:active={chat.open}
    title={`Chat (${platform.modifierLabel}+Shift+C)`}
    onclick={() => chat.toggle()}
  >
    <MessageSquare size={15} />
  </button>

  <button
    class="pk-icon-btn"
    class:active={aesthetics.panelOpen}
    title="Aesthetics"
    onclick={() => (aesthetics.panelOpen = !aesthetics.panelOpen)}
  >
    <Settings size={15} />
  </button>

  {#if showWindowControls}
    <div class="pk-window-controls">
      <button class="pk-win-btn" onclick={minimize} title="Minimize" aria-label="Minimize window">
        <Minus size={13} strokeWidth={1.6} />
      </button>
      <button
        class="pk-win-btn"
        onclick={toggleMaximize}
        title={maximized ? 'Restore' : 'Maximize'}
        aria-label={maximized ? 'Restore window' : 'Maximize window'}
      >
        <Square size={11} strokeWidth={1.6} />
      </button>
      <button class="pk-win-btn pk-win-close" onclick={closeWindow} title="Close" aria-label="Close window">
        <X size={13} strokeWidth={1.6} />
      </button>
    </div>
  {/if}
</header>

<style>
  /* macOS keeps its traffic lights over the webview, so the bar's own content
     starts clear of them. Set as a class, not an inline style: the Tauri CSP
     is `style-src 'self'` and would reject a style attribute. */
  .pk-topbar-inset {
    padding-left: 78px;
  }

  .pk-window-controls {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: 6px;
  }

  .pk-win-btn {
    display: grid;
    place-items: center;
    width: 30px;
    height: 26px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--p-text-dim);
    cursor: pointer;
    transition:
      background 0.15s ease,
      color 0.15s ease;
  }

  .pk-win-btn:hover {
    background: var(--p-surface2);
    color: var(--p-text);
  }

  .pk-win-close:hover {
    background: var(--p-down);
    color: #fff;
  }
</style>
