/**
 * Vendor-neutral chart contracts (P1-EX-02).
 * Spec: DESIGN_SYSTEM §9, Architecture Evolution §40–43.
 * Implementations (Lightweight Charts, wgpu, SVG) live outside this package.
 */

export type ChartBackendKind =
  | "lightweight-charts"
  | "prismatik-wgpu"
  | "accessible-svg"
  | "export-renderer";

export interface ChartTheme {
  background: string;
  text: string;
  grid: string;
  border: string;
  up: string;
  down: string;
  crosshair: string;
}

export interface CandlestickPoint {
  /** Unix seconds (UTC). */
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
}

/** Minimal Wave 1 ChartDocument — candles + provenance only. */
export interface ChartDocument {
  id: string;
  assetId: string;
  symbol: string;
  interval: string;
  candles: CandlestickPoint[];
  provider: string;
  retrievedAt: string;
}

export interface ChartCapabilities {
  candlesticks: boolean;
  line: boolean;
  volume: boolean;
  liveAppend: boolean;
  themes: boolean;
}

export type ChartCommand =
  | { kind: "setCandles"; candles: CandlestickPoint[] }
  | { kind: "setTheme"; theme: ChartTheme }
  | { kind: "fitContent" };

export interface ChartBackend {
  readonly kind: ChartBackendKind;
  readonly capabilities: ChartCapabilities;
  mount(target: HTMLElement): Promise<void>;
  apply(command: ChartCommand): Promise<void>;
  dispose(): Promise<void>;
}

/** Read CSS design tokens into a ChartTheme (light shell default). */
export function chartThemeFromTokens(
  style: CSSStyleDeclaration = getComputedStyle(document.documentElement),
): ChartTheme {
  const read = (name: string, fallback: string) =>
    style.getPropertyValue(name).trim() || fallback;
  return {
    background: read("--color-surface-1", "#ffffff"),
    text: read("--color-text-secondary", "#4a5263"),
    grid: read("--color-border-default", "#d5dbe6"),
    border: read("--color-border-default", "#d5dbe6"),
    up: read("--color-up", "#0f9f6e"),
    down: read("--color-down", "#d92d2d"),
    crosshair: read("--color-text-tertiary", "#7a8499"),
  };
}
