<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    createChart,
    type IChartApi,
    type ISeriesApi,
    type CandlestickData,
    type UTCTimestamp,
  } from "lightweight-charts";
  import {
    chartThemeFromTokens,
    type CandlestickPoint,
  } from "@prismatik/chart-contracts";

  let {
    candles = [],
    height = 280,
  }: {
    candles?: CandlestickPoint[];
    height?: number;
  } = $props();

  let host: HTMLDivElement | undefined = $state();
  let chart: IChartApi | undefined;
  let series: ISeriesApi<"Candlestick"> | undefined;
  let ro: ResizeObserver | undefined;
  let themeObserver: MutationObserver | undefined;

  function toData(points: CandlestickPoint[]): CandlestickData[] {
    return points.map((p) => ({
      time: p.time as UTCTimestamp,
      open: p.open,
      high: p.high,
      low: p.low,
      close: p.close,
    }));
  }

  function syncSeries(points: CandlestickPoint[]) {
    if (!series) return;
    series.setData(toData(points));
    chart?.timeScale().fitContent();
  }

  function syncTheme() {
    if (!chart || !series) return;
    const theme = chartThemeFromTokens();
    chart.applyOptions({
      layout: { background: { color: theme.background }, textColor: theme.text },
      grid: {
        vertLines: { color: theme.grid },
        horzLines: { color: theme.grid },
      },
      rightPriceScale: { borderColor: theme.border },
      timeScale: { borderColor: theme.border },
      crosshair: {
        vertLine: { color: theme.crosshair },
        horzLine: { color: theme.crosshair },
      },
    });
    series.applyOptions({
      upColor: theme.up,
      downColor: theme.down,
      borderUpColor: theme.up,
      borderDownColor: theme.down,
      wickUpColor: theme.up,
      wickDownColor: theme.down,
    });
  }

  onMount(() => {
    if (!host) return;
    const theme = chartThemeFromTokens();
    chart = createChart(host, {
      height,
      width: host.clientWidth || 640,
      layout: {
        background: { color: theme.background },
        textColor: theme.text,
        fontFamily: "IBM Plex Sans, sans-serif",
      },
      grid: {
        vertLines: { color: theme.grid },
        horzLines: { color: theme.grid },
      },
      rightPriceScale: { borderColor: theme.border },
      timeScale: { borderColor: theme.border },
      crosshair: {
        vertLine: { color: theme.crosshair },
        horzLine: { color: theme.crosshair },
      },
    });
    series = chart.addCandlestickSeries({
      upColor: theme.up,
      downColor: theme.down,
      borderUpColor: theme.up,
      borderDownColor: theme.down,
      wickUpColor: theme.up,
      wickDownColor: theme.down,
    });
    syncSeries(candles);
    ro = new ResizeObserver(() => {
      if (!host || !chart) return;
      chart.applyOptions({ width: host.clientWidth });
    });
    ro.observe(host);
    themeObserver = new MutationObserver(syncTheme);
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
  });

  $effect(() => {
    syncSeries(candles);
  });

  $effect(() => {
    if (chart) chart.applyOptions({ height });
  });

  onDestroy(() => {
    ro?.disconnect();
    themeObserver?.disconnect();
    chart?.remove();
    chart = undefined;
    series = undefined;
  });
</script>

<div class="price-chart" bind:this={host} role="img" aria-label="Price chart"></div>

<style>
  .price-chart {
    width: 100%;
    min-height: 200px;
  }
</style>
