<script lang="ts">
  /**
   * The selected instrument's regime and its regime-conditional forecast.
   *
   * This is the terminal's primary derived signal, so it states its own
   * evidence inline: how long the regime has held, how many historical
   * episodes the probability rests on, and — when the sample is thin — that
   * the number is not yet worth acting on.
   */
  import { market } from './market.svelte';

  interface EmpiricalForecastRow {
    regime: string;
    horizonBars: number;
    sampleSize: number;
    direction: 'up' | 'down' | 'flat';
    probabilityPpm: number;
    climatologyPpm: number;
    edgePpm: number;
    medianMoveBps: number;
    p10MoveBps: number;
    p90MoveBps: number;
    sufficient: boolean;
  }

  interface InstrumentAnalytics {
    symbol: string;
    barCount: number;
    currentRegime: string | null;
    currentRunLength: number;
    realizedVol: number;
    volPercentile: number;
    forecasts: EmpiricalForecastRow[];
    message: string;
  }

  const LABELS: Record<string, string> = {
    calm_trending: 'CALM TREND',
    calm_mean_revert: 'CALM REVERT',
    volatile_trending: 'VOL TREND',
    volatile_mean_revert: 'VOL REVERT',
    crisis: 'CRISIS',
  };

  let analytics = $state<InstrumentAnalytics | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);

  // One in-flight request per instrument; the key guards against a slow
  // response for a previous symbol overwriting the current one.
  let requestKey = '';

  $effect(() => {
    const inst = market.selected;
    if (!inst) {
      analytics = null;
      return;
    }
    const key = `${inst.kind}:${inst.providerId}`;
    if (key === requestKey) return;
    requestKey = key;
    void load(key, inst.kind, inst.providerId, inst.symbol);
  });

  async function load(
    key: string,
    kind: string,
    providerId: string,
    symbol: string,
  ): Promise<void> {
    loading = true;
    error = null;
    try {
      const { invoke, isTauri } = await import('@tauri-apps/api/core');
      if (!isTauri()) return;
      const result = await invoke<InstrumentAnalytics>('analyze_instrument', {
        kind,
        providerId,
        symbol,
      });
      if (requestKey === key) analytics = result;
    } catch (e) {
      if (requestKey === key) {
        analytics = null;
        error = e instanceof Error ? e.message : String(e);
      }
    } finally {
      loading = false;
    }
  }

  const regime = $derived(analytics?.currentRegime ?? null);
  /** The 5-day view: long enough to be actionable, short enough to score often. */
  const headline = $derived(analytics?.forecasts.find((f) => f.horizonBars === 5) ?? null);

  function pct(ppm: number): string {
    return `${(ppm / 10_000).toFixed(0)}%`;
  }

  /** Edge in percentage points — the part of the call that is information. */
  function edgePp(ppm: number): string {
    const pp = ppm / 10_000;
    return `${pp >= 0 ? '+' : '−'}${Math.abs(pp).toFixed(1)}pp`;
  }

  /**
   * An edge inside this band is indistinguishable from the base rate at the
   * sample sizes involved, so the forecast is shown as carrying no information
   * rather than as a weak signal.
   */
  const NOISE_BAND_PPM = 20_000;
</script>

<div class="pk-regime">
  {#if loading && !analytics}
    <span class="pk-regime-idle">classifying…</span>
  {:else if error}
    <span class="pk-regime-idle" title={error}>regime unavailable</span>
  {:else if !regime}
    <span class="pk-regime-idle" title={analytics?.message ?? ''}>unclassified</span>
  {:else}
    <span
      class="pk-regime-tag"
      data-regime={regime}
      title={`Held ${analytics?.currentRunLength ?? 0} bars · realized vol ${((analytics?.realizedVol ?? 0) * 100).toFixed(1)}% (${((analytics?.volPercentile ?? 0) * 100).toFixed(0)}th pct of own history)`}
    >
      {LABELS[regime] ?? regime}
      <small>{analytics?.currentRunLength ?? 0}d</small>
    </span>
    {#if headline}
      {@const flat = Math.abs(headline.edgePpm) < NOISE_BAND_PPM}
      <span
        class="pk-regime-fc"
        class:thin={!headline.sufficient}
        class:noedge={flat}
        data-dir={headline.direction}
        title={`Over ${headline.horizonBars} bars: ${pct(headline.probabilityPpm)} vs a ${pct(headline.climatologyPpm)} base rate for the same claim — an edge of ${edgePp(headline.edgePpm)}. Based on ${headline.sampleSize} historical episodes of this regime; median ${headline.medianMoveBps.toFixed(0)} bps, 10-90 range ${headline.p10MoveBps.toFixed(0)} to ${headline.p90MoveBps.toFixed(0)} bps.${headline.sufficient ? '' : ' Sample too small to act on.'}${flat ? ' Edge is inside the noise band — this call is not distinguishable from the base rate.' : ''}`}
      >
        5d {headline.direction} {pct(headline.probabilityPpm)}
        <b class="pk-regime-edge">{flat ? 'no edge' : edgePp(headline.edgePpm)}</b>
        <small>n={headline.sampleSize}{headline.sufficient ? '' : ' ·thin'}</small>
      </span>
    {/if}
  {/if}
</div>

<style>
  .pk-regime {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .pk-regime-idle {
    color: var(--p-dim);
    font-family: var(--font-mono);
    font-size: var(--fz-sm);
    opacity: 0.7;
  }
  .pk-regime-tag,
  .pk-regime-fc {
    display: inline-flex;
    align-items: baseline;
    gap: 5px;
    padding: 2px 7px;
    border: 1px solid currentColor;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: var(--fz-sm);
    letter-spacing: 0.08em;
    white-space: nowrap;
  }
  .pk-regime-tag small,
  .pk-regime-fc small {
    font-size: 8.5px;
    opacity: 0.7;
  }
  .pk-regime-tag[data-regime='calm_trending'] {
    color: #34d399;
  }
  .pk-regime-tag[data-regime='calm_mean_revert'] {
    color: #4dc8ff;
  }
  .pk-regime-tag[data-regime='volatile_trending'] {
    color: #fbbf24;
  }
  .pk-regime-tag[data-regime='volatile_mean_revert'] {
    color: #f87171;
  }
  .pk-regime-tag[data-regime='crisis'] {
    color: #ef4444;
    background: color-mix(in srgb, #ef4444 14%, transparent);
  }
  .pk-regime-fc[data-dir='up'] {
    color: var(--p-up);
  }
  .pk-regime-fc[data-dir='down'] {
    color: var(--p-down);
  }
  .pk-regime-fc[data-dir='flat'] {
    color: var(--p-dim);
  }
  /* A thin sample is shown but visibly provisional. */
  .pk-regime-fc.thin {
    border-style: dashed;
    opacity: 0.65;
  }
  /* No edge over the base rate: the probability is real but uninformative, so
     the whole chip drops to a neutral colour and stops reading as a call. */
  .pk-regime-fc.noedge {
    color: var(--p-dim);
  }
  .pk-regime-edge {
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
  }
</style>
