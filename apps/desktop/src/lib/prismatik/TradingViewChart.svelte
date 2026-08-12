<script lang="ts">
  /**
   * TradingView chart, fed exclusively by PRISMATIK's governed data.
   *
   * This uses TradingView's **Lightweight Charts** (Apache-2.0), bundled into
   * the app, rather than their embeddable widget. That is a deliberate choice
   * and not merely a convenience:
   *
   * - The widget loads a remote script and an iframe from tradingview.com. The
   *   Tauri CSP is `script-src 'self'` with `frame-ancestors 'none'`, so it
   *   cannot load without gutting the app's security posture.
   * - The widget's prices come from TradingView, not from the providers whose
   *   observations PRISMATIK cites. Charting one source while reasoning over
   *   another would quietly break the evidence chain that everything else here
   *   depends on.
   * - A widget is opaque. This library lets the regime classification be drawn
   *   *underneath the price*, which is the entire point of the integration.
   *
   * The ribbon below the candles is the volatility regime for each bar, from
   * the same deterministic classifier the trader sizes on. Reading price and
   * regime on one time axis is what makes "this instrument has been in calm
   * mean-reversion for twenty bars" legible rather than a number in a badge.
   */
  import { onMount, untrack } from 'svelte';
  import {
    createChart,
    ColorType,
    CrosshairMode,
    type IChartApi,
    type ISeriesApi,
    type UTCTimestamp,
    type SeriesMarker,
    type Time,
  } from 'lightweight-charts';
  import { aesthetics } from './aesthetics.svelte';
  import { market, type Candle } from './market.svelte';
  import NoData from './NoData.svelte';

  interface RegimeLabel {
    id: string;
    t: number;
  }

  interface InstrumentAnalytics {
    currentRegime: string | null;
    timeline: RegimeLabel[];
    message: string;
  }

  const REGIME_COLORS: Record<string, string> = {
    calm_trending: '#34d399',
    calm_mean_revert: '#4dc8ff',
    volatile_trending: '#fbbf24',
    volatile_mean_revert: '#f87171',
    crisis: '#ef4444',
  };

  const REGIME_LABELS: Record<string, string> = {
    calm_trending: 'Calm trend',
    calm_mean_revert: 'Calm revert',
    volatile_trending: 'Vol trend',
    volatile_mean_revert: 'Vol revert',
    crisis: 'Crisis',
  };

  let host: HTMLDivElement;
  let chart: IChartApi | null = null;
  let candleSeries: ISeriesApi<'Candlestick'> | null = null;
  let regimeSeries: ISeriesApi<'Histogram'> | null = null;

  let analytics = $state<InstrumentAnalytics | null>(null);
  let analyticsKey = '';

  const inst = $derived(market.selected);
  const series = $derived(market.getCandles(inst));

  /** Regimes present in the current view, for the legend. */
  const legend = $derived(
    analytics
      ? [...new Set(analytics.timeline.map((row) => row.id))].filter((id) => id in REGIME_COLORS)
      : [],
  );

  function themeVar(name: string, fallback: string): string {
    return aesthetics.vars[name] ?? fallback;
  }

  function buildChart(): void {
    if (!host || chart) return;
    chart = createChart(host, {
      autoSize: true,
      layout: {
        // Transparent so the terminal's own panel background and any theme
        // change show through without rebuilding the chart.
        background: { type: ColorType.Solid, color: 'transparent' },
        textColor: themeVar('--p-dim', '#6b7690'),
        fontFamily: 'IBM Plex Mono, ui-monospace, monospace',
        fontSize: 10,
      },
      grid: {
        vertLines: { color: themeVar('--p-grid', 'rgba(120,140,175,0.1)') },
        horzLines: { color: themeVar('--p-grid', 'rgba(120,140,175,0.1)') },
      },
      crosshair: { mode: CrosshairMode.Normal },
      rightPriceScale: { borderColor: themeVar('--p-border', '#1d2433') },
      timeScale: {
        borderColor: themeVar('--p-border', '#1d2433'),
        timeVisible: false,
      },
    });

    candleSeries = chart.addCandlestickSeries({
      upColor: themeVar('--p-up', '#23e08b'),
      downColor: themeVar('--p-down', '#ff4d67'),
      wickUpColor: themeVar('--p-up', '#23e08b'),
      wickDownColor: themeVar('--p-down', '#ff4d67'),
      borderVisible: false,
    });

    // The regime ribbon: a constant-height histogram on its own overlay scale,
    // pinned to the bottom so it reads as a band under the price rather than
    // as a second data series competing with it.
    regimeSeries = chart.addHistogramSeries({
      priceScaleId: 'regime',
      priceLineVisible: false,
      lastValueVisible: false,
    });
    chart.priceScale('regime').applyOptions({
      scaleMargins: { top: 0.92, bottom: 0 },
    });
  }

  /** Seconds since epoch — the unit Lightweight Charts expects. */
  function toTime(epochMs: number): UTCTimestamp {
    return Math.floor(epochMs / 1000) as UTCTimestamp;
  }

  function paint(): void {
    if (!candleSeries || !regimeSeries) return;
    const bars = Array.isArray(series) ? series : [];
    candleSeries.setData(
      bars.map((bar: Candle) => ({
        time: toTime(bar.t),
        open: bar.o,
        high: bar.h,
        low: bar.l,
        close: bar.c,
      })),
    );

    const labels = analytics?.timeline ?? [];
    if (labels.length === 0) {
      regimeSeries.setData([]);
      candleSeries.setMarkers([]);
      return;
    }

    const byTime = new Map(labels.map((row) => [toTime(row.t), row.id]));
    regimeSeries.setData(
      bars
        .map((bar) => {
          const time = toTime(bar.t);
          const regime = byTime.get(time);
          return regime
            ? { time, value: 1, color: REGIME_COLORS[regime] ?? '#6b7690' }
            : null;
        })
        .filter((row): row is { time: UTCTimestamp; value: number; color: string } => row !== null),
    );

    // Mark only the bars where the regime actually changed. Marking every bar
    // would bury the transitions, which are the informative moments.
    const markers: SeriesMarker<Time>[] = [];
    let previous: string | null = null;
    for (const row of labels) {
      if (previous !== null && row.id !== previous) {
        markers.push({
          time: toTime(row.t),
          position: 'belowBar',
          color: REGIME_COLORS[row.id] ?? '#6b7690',
          shape: 'arrowUp',
          text: REGIME_LABELS[row.id] ?? row.id,
        });
      }
      previous = row.id;
    }
    candleSeries.setMarkers(markers.slice(-40));
  }

  async function loadAnalytics(): Promise<void> {
    const current = market.selected;
    if (!current) {
      analytics = null;
      return;
    }
    const key = `${current.kind}:${current.providerId}`;
    if (key === analyticsKey) return;
    analyticsKey = key;
    try {
      const { invoke, isTauri } = await import('@tauri-apps/api/core');
      if (!isTauri()) return;
      const result = await invoke<InstrumentAnalytics>('analyze_instrument', {
        kind: current.kind,
        providerId: current.providerId,
        symbol: current.symbol,
      });
      if (analyticsKey === key) analytics = result;
    } catch {
      // The chart is still useful without the ribbon; leave it empty rather
      // than failing the whole panel.
      if (analyticsKey === key) analytics = null;
    }
  }

  onMount(() => {
    buildChart();
    return () => {
      chart?.remove();
      chart = null;
      candleSeries = null;
      regimeSeries = null;
    };
  });

  // Pull history and analytics whenever the instrument or timeframe changes.
  $effect(() => {
    const current = market.selected;
    const tf = market.timeframe;
    if (current) void market.loadCandles(current, tf);
    void loadAnalytics();
  });

  $effect(() => {
    void series;
    void analytics;
    untrack(() => paint());
  });

  // Rebuild on theme change: colours are baked into series options at
  // construction, so a repaint alone would keep the old palette.
  $effect(() => {
    void aesthetics.theme;
    void aesthetics.accent;
    untrack(() => {
      if (!chart) return;
      chart.remove();
      chart = null;
      candleSeries = null;
      regimeSeries = null;
      buildChart();
      paint();
    });
  });
</script>

<div class="pk-tv">
  <div class="pk-tv-host" bind:this={host}></div>

  {#if legend.length > 0}
    <div class="pk-tv-legend">
      <span class="pk-tv-legend-title">REGIME</span>
      {#each legend as id (id)}
        <span class="pk-tv-legend-item">
          <i style:background={REGIME_COLORS[id]}></i>{REGIME_LABELS[id] ?? id}
        </span>
      {/each}
    </div>
  {/if}

  {#if !inst}
    <div class="pk-tv-overlay">
      <NoData title="No instrument selected" detail="Track an instrument to chart it." />
    </div>
  {:else if series === undefined}
    <div class="pk-tv-overlay"><NoData title="Loading {inst.symbol} history" compact /></div>
  {:else if series === null || series.length === 0}
    <div class="pk-tv-overlay">
      <NoData
        title="No price history for {inst.symbol}"
        detail={market.candleError(inst) ?? 'The history provider returned no bars.'}
        tone="warn"
      />
    </div>
  {/if}
</div>

<style>
  .pk-tv {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 0;
  }
  .pk-tv-host {
    width: 100%;
    height: 100%;
  }
  .pk-tv-legend {
    position: absolute;
    top: 8px;
    left: 10px;
    z-index: 3;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 3px 8px;
    border: 1px solid var(--p-border);
    border-radius: 5px;
    background: color-mix(in srgb, var(--p-bg) 78%, transparent);
    font-family: var(--font-mono);
    font-size: var(--fz-sm);
    pointer-events: none;
  }
  .pk-tv-legend-title {
    color: var(--p-dim);
    letter-spacing: 0.14em;
  }
  .pk-tv-legend-item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--p-text);
  }
  .pk-tv-legend-item i {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }
  .pk-tv-overlay {
    position: absolute;
    inset: 0;
    display: grid;
    background: color-mix(in srgb, var(--p-bg) 70%, transparent);
    place-items: center;
  }
</style>
