<script lang="ts">
  /**
   * Feed-state pill. The one place the terminal declares how much of what it
   * shows is actually observed, so the answer is never more than a glance away.
   */
  import type { FeedMode } from './market.svelte';

  let {
    mode,
    providers,
    message,
    quoted,
    tracked,
  }: {
    mode: FeedMode;
    providers: string[];
    message: string;
    quoted: number;
    tracked: number;
  } = $props();

  const label = $derived(
    mode === 'live'
      ? `LIVE ${quoted}/${tracked} · ${providers.join(' + ')}`
      : mode === 'degraded'
        ? `PARTIAL ${quoted}/${tracked}`
        : mode === 'empty'
          ? 'NOTHING TRACKED'
          : 'NO MARKET DATA',
  );
</script>

<a
  class="pk-input-pill"
  class:live={mode === 'live'}
  class:degraded={mode === 'degraded'}
  href="/workspace/integrations"
  aria-label={`${label}. ${message}. Open integrations.`}
  title={message}
>
  <span></span>
  {label}
</a>

<style>
  .live {
    border-color: color-mix(in srgb, #28e7a4 55%, transparent);
    color: #7fffd4;
  }
  .degraded {
    border-color: color-mix(in srgb, #ffb84d 55%, transparent);
    color: #ffd18b;
  }
</style>
