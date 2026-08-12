import {
  ColorType,
  createChart,
  type CandlestickData,
  type IChartApi,
  type ISeriesApi,
  type UTCTimestamp,
} from 'lightweight-charts';
import type {
  ChartBackend,
  ChartCommand,
  ChartTheme,
} from './index.js';

const DEFAULT_THEME: ChartTheme = {
  background: '#ffffff',
  text: '#4a5263',
  grid: '#d5dbe6',
  border: '#d5dbe6',
  up: '#0f9f6e',
  down: '#d92d2d',
  crosshair: '#7a8499',
};

/** Lightweight Charts v4 adapter. */
export class LightWeightChartAdapter implements ChartBackend {
  readonly kind = 'lightweight-charts' as const;
  readonly capabilities = {
    candlesticks: true,
    line: true,
    volume: false,
    liveAppend: true,
    themes: true,
  };

  private chart: IChartApi | null = null;
  private candleSeries: ISeriesApi<'Candlestick'> | null = null;

  constructor(containerId: string, initialTheme: ChartTheme = DEFAULT_THEME) {
    const container = document.getElementById(containerId);
    if (!container) return;

    this.chart = createChart(container, {
      width: container.clientWidth || 640,
      height: container.clientHeight || 360,
      localization: {
        locale: 'en-US',
      },
      layout: {
        background: { type: ColorType.Solid, color: initialTheme.background },
        textColor: initialTheme.text,
        fontFamily: 'IBM Plex Mono, monospace',
      },
      grid: {
        vertLines: { color: initialTheme.grid },
        horzLines: { color: initialTheme.grid },
      },
      rightPriceScale: { borderColor: initialTheme.border },
      timeScale: { borderColor: initialTheme.border },
      crosshair: {
        vertLine: { color: initialTheme.crosshair },
        horzLine: { color: initialTheme.crosshair },
      },
    });

    this.candleSeries = this.chart.addCandlestickSeries();
    this.applyTheme(initialTheme);
  }

  async apply(command: ChartCommand): Promise<void> {
    if (command.kind === 'setCandles') {
      const data: CandlestickData[] = command.candles.map((candle) => ({
        time: Math.floor(candle.time > 10_000_000_000 ? candle.time / 1000 : candle.time) as UTCTimestamp,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
      }));
      this.candleSeries?.setData(data);
      this.chart?.timeScale().fitContent();
    } else if (command.kind === 'setTheme') {
      this.applyTheme(command.theme);
    } else if (command.kind === 'fitContent') {
      this.chart?.timeScale().fitContent();
    }
  }

  private applyTheme(theme: ChartTheme): void {
    this.chart?.applyOptions({
      layout: {
        background: { type: ColorType.Solid, color: theme.background },
        textColor: theme.text,
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
    this.candleSeries?.applyOptions({
      upColor: theme.up,
      downColor: theme.down,
      borderUpColor: theme.up,
      borderDownColor: theme.down,
      wickUpColor: theme.up,
      wickDownColor: theme.down,
    });
  }

  async mount(_target: HTMLElement): Promise<void> {}

  async dispose(): Promise<void> {
    this.chart?.remove();
    this.chart = null;
    this.candleSeries = null;
  }
}

export default LightWeightChartAdapter;
