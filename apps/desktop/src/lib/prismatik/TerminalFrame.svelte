<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import './prismatik.css';
  import { aesthetics } from './aesthetics.svelte';
  import { market } from './market.svelte';
  import TopBar from './TopBar.svelte';
  import TickerTape from './TickerTape.svelte';
  import MissionRail from './MissionRail.svelte';
  import PositionsStrip from './PositionsStrip.svelte';
  import AestheticsPanel from './AestheticsPanel.svelte';
  import CommandDeck from './CommandDeck.svelte';
  import ChatPanel from './ChatPanel.svelte';
  import { chat } from './chat.svelte';

  let { children }: { children: Snippet } = $props();
  let rootEl: HTMLDivElement;

  $effect(() => {
    const vars = aesthetics.vars;
    for (const [key, value] of Object.entries(vars)) rootEl.style.setProperty(key, value);
  });

  onMount(() => {
    // The store owns loading the tracked list, the quote poll and its own
    // failure reporting — there is no client-side fallback to fall back to.
    void market.start();

    // Cmd+Shift+C toggles chat
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'c') {
        e.preventDefault();
        chat.toggle();
      }
    };
    window.addEventListener('keydown', onKey);

    return () => {
      market.stop();
      window.removeEventListener('keydown', onKey);
    };
  });
</script>

<div bind:this={rootEl} class="prismatik" data-theme={aesthetics.theme} data-density={aesthetics.density} data-motion={aesthetics.motion} data-layout={aesthetics.layout}>
  <TopBar />
  <TickerTape />
  <div class="pk-workspace" class:chat-open={chat.open}>
    <MissionRail />
    <div class="pk-route-stage">{@render children()}</div>
    {#if chat.open}
      <ChatPanel />
    {/if}
  </div>
  <PositionsStrip />
  <CommandDeck />
  {#if aesthetics.panelOpen}<AestheticsPanel />{/if}
</div>


