<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { aesthetics } from './aesthetics.svelte';
  import {
    market,
    fmtPrice,
    fmtPct,
    fmtNum,
    fmtCompact,
    TIMEFRAMES,
    VIZ_MODES,
    type Candle,
  } from './market.svelte';

  let host: HTMLDivElement;
  let canvas: HTMLCanvasElement;

  // crosshair state (plain — read imperatively by the draw loop)
  let mouseX = -1;
  let mouseY = -1;

  // instrument/timeframe/mode transition animation
  let animStart = performance.now();
  $effect(() => {
    void market.selected;
    void market.timeframe;
    void market.viz;
    animStart = performance.now();
  });

  // ---- Indicator overlays (#12) ----
  type IndicatorKind =
    | 'sma' | 'ema' | 'bb_upper' | 'bb_mid' | 'bb_lower' | 'vwap'
    | 'rsi' | 'macd_line' | 'macd_signal' | 'macd_hist' | 'stoch_k' | 'stoch_d' | 'atr';
  type IndicatorPane = 'price' | 'sub';

  interface IndicatorSeries {
    kind: IndicatorKind;
    label: string;
    period: number;
    pane: IndicatorPane;
    values: (number | null)[];
  }

  interface IndicatorToggle {
    kind: IndicatorKind;
    label: string;
    period: number;
    active: boolean;
  }

  const INDICATOR_MENU: IndicatorToggle[] = [
    { kind: 'sma', label: 'SMA 20', period: 20, active: false },
    { kind: 'sma', label: 'SMA 50', period: 50, active: false },
    { kind: 'ema', label: 'EMA 12', period: 12, active: false },
    { kind: 'ema', label: 'EMA 26', period: 26, active: false },
    { kind: 'bb_upper', label: 'BB(20,2)', period: 20, active: false },
    { kind: 'vwap', label: 'VWAP', period: 20, active: false },
    { kind: 'rsi', label: 'RSI 14', period: 14, active: false },
    { kind: 'macd_line', label: 'MACD 12', period: 12, active: false },
    { kind: 'stoch_k', label: 'Stoch 14', period: 14, active: false },
    { kind: 'atr', label: 'ATR 14', period: 14, active: false },
  ];

  let indicators = $state<IndicatorToggle[]>(INDICATOR_MENU.map((i) => ({ ...i })));
  let indicatorSeries = $state<IndicatorSeries[]>([]);
  let showIndicatorMenu = $state(false);

  // Cache key to avoid recompute when candles haven't changed.
  let lastComputeKey = '';
  let inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  function activeRequests(): { kind: IndicatorKind; period: number }[] {
    return indicators.filter((i) => i.active).map((i) => ({ kind: i.kind, period: i.period }));
  }

  async function recomputeIndicators(candles: Candle[]) {
    const reqs = activeRequests();
    if (!inTauri || reqs.length === 0 || candles.length < 2) {
      indicatorSeries = [];
      return;
    }
    const bars = candles.map((c) => ({
      open: c.o, high: c.h, low: c.l, close: c.c, volume: c.v,
    }));
    try {
      indicatorSeries = await invoke<IndicatorSeries[]>('compute_chart_indicators', {
        bars,
        requests: reqs,
      });
    } catch {
      indicatorSeries = [];
    }
  }

  // Recompute when toggles change or symbol/timeframe changes.
  let recomputeTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    // Track indicator toggles + symbol + timeframe for recompute trigger.
    void indicators.map((i) => i.active).join(',');
    void market.selected;
    void market.timeframe;
    if (recomputeTimer) clearTimeout(recomputeTimer);
    recomputeTimer = setTimeout(() => {
      const candles = market.getCandles();
      const key = `${market.selected.def.symbol}:${market.timeframe}:${candles.length}`;
      if (key !== lastComputeKey || indicators.some((i) => i.active)) {
        lastComputeKey = key;
        void recomputeIndicators(candles);
      }
    }, 100);
  });

  // Overlay colors per indicator kind.
  const INDICATOR_COLORS: Record<string, string> = {
    sma: '#f59e0b', ema: '#a78bfa', bb_upper: '#4dc8ff', bb_mid: '#6b7690',
    bb_lower: '#4dc8ff', vwap: '#34d399', rsi: '#f59e0b', macd_line: '#4dc8ff',
    macd_signal: '#f87171', macd_hist: '#6b7690', stoch_k: '#4dc8ff', stoch_d: '#f59e0b',
    atr: '#a78bfa',
  };

  function drawPriceOverlays(ctx: CanvasRenderingContext2D, candles: Candle[], W: number, H: number, min: number, max: number) {
    const plotW = W - AXIS_W;
    const n = candles.length;
    if (n === 0) return;
    const y = (v: number) => PAD_T + ((max - v) / (max - min)) * (H - PAD_T - PAD_B);
    for (const series of indicatorSeries.filter((s) => s.pane === 'price')) {
      const color = INDICATOR_COLORS[series.kind] ?? '#4dc8ff';
      ctx.strokeStyle = color;
      ctx.lineWidth = 1.2;
      ctx.globalAlpha = 0.85;
      ctx.beginPath();
      let started = false;
      for (let i = 0; i < series.values.length && i < n; i++) {
        const v = series.values[i];
        if (v === null || !Number.isFinite(v)) {
          started = false;
          continue;
        }
        const x = (i / n) * plotW + plotW / n / 2;
        const py = y(v);
        if (!started) {
          ctx.moveTo(x, py);
          started = true;
        } else {
          ctx.lineTo(x, py);
        }
      }
      ctx.stroke();
      ctx.globalAlpha = 1;
    }
  }

  function drawSubPane(ctx: CanvasRenderingContext2D, pal: any, candles: Candle[], W: number, H: number) {
    const subSeries = indicatorSeries.filter((s) => s.pane === 'sub');
    if (subSeries.length === 0) return;
    const plotW = W - AXIS_W;
    const n = candles.length;
    if (n === 0) return;
    // Sub-pane occupies bottom 20% of chart.
    const subH = Math.min(H * 0.2, 80);
    const subTop = H - subH - PAD_B;
    const subBottom = H - PAD_B;
    // Draw separator + background.
    ctx.strokeStyle = pal.border;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, subTop);
    ctx.lineTo(plotW, subTop);
    ctx.stroke();
    ctx.fillStyle = pal.surface;
    ctx.fillRect(0, subTop + 1, plotW, subH - 1);
    // For each sub-pane series, compute its own min/max.
    for (const series of subSeries) {
      const vals = series.values.filter((v): v is number => v !== null && Number.isFinite(v));
      if (vals.length === 0) continue;
      let lo = Math.min(...vals);
      let hi = Math.max(...vals);
      // RSI/Stoch: fixed 0-100 scale with padding.
      if (series.kind === 'rsi' || series.kind === 'stoch_k' || series.kind === 'stoch_d') {
        lo = 0; hi = 100;
      }
      if (hi - lo < 1e-9) { hi = lo + 1; }
      const py = (v: number) => subTop + ((hi - v) / (hi - lo)) * subH;
      const color = INDICATOR_COLORS[series.kind] ?? '#4dc8ff';
      // For RSI/Stoch, draw 30/70 reference lines.
      if (series.kind === 'rsi' || series.kind === 'stoch_k' || series.kind === 'stoch_d') {
        ctx.strokeStyle = pal.dim;
        ctx.globalAlpha = 0.3;
        ctx.setLineDash([2, 4]);
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(0, py(70)); ctx.lineTo(plotW, py(70));
        ctx.moveTo(0, py(30)); ctx.lineTo(plotW, py(30));
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.globalAlpha = 1;
      }
      // Draw the series line.
      ctx.strokeStyle = color;
      ctx.lineWidth = 1.2;
      ctx.globalAlpha = 0.9;
      ctx.beginPath();
      let started = false;
      for (let i = 0; i < series.values.length && i < n; i++) {
        const v = series.values[i];
        if (v === null || !Number.isFinite(v)) { started = false; continue; }
        const x = (i / n) * plotW + plotW / n / 2;
        const yPos = py(v);
        if (!started) { ctx.moveTo(x, yPos); started = true; }
        else { ctx.lineTo(x, yPos); }
      }
      ctx.stroke();
      ctx.globalAlpha = 1;
      // Label.
      ctx.fillStyle = color;
      ctx.font = `9px ${MONO}`;
      ctx.fillText(series.label, 8, subTop + 10);
      // Scale labels.
      ctx.fillStyle = pal.dim;
      ctx.textAlign = 'right';
      ctx.fillText(fmtNum(hi, 2), W - AXIS_W + 4, subTop + 6);
      ctx.fillText(fmtNum(lo, 2), W - AXIS_W + 4, subBottom - 2);
      ctx.textAlign = 'left';
    }
  }

  /* ------------------------------------------------------------ colors */

  type RGB = [number, number, number];

  function hexToRgb(hex: string): RGB {
    const h = hex.replace('#', '');
    const v = parseInt(h.length === 3 ? h.split('').map((c) => c + c).join('') : h, 16);
    return [(v >> 16) & 255, (v >> 8) & 255, v & 255];
  }

  function rgba(c: RGB, a: number): string {
    return `rgba(${c[0]},${c[1]},${c[2]},${a})`;
  }

  function mix(a: RGB, b: RGB, t: number): RGB {
    return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
  }

  interface Pal {
    up: RGB;
    down: RGB;
    accent: RGB;
    accent2: RGB;
    text: string;
    dim: string;
    grid: string;
    surface: string;
    glow: string;
    bg: string;
  }

  function palette(): Pal {
    const v = aesthetics.vars;
    return {
      up: hexToRgb(v['--p-up']),
      down: hexToRgb(v['--p-down']),
      accent: hexToRgb(v['--p-accent']),
      accent2: hexToRgb(v['--p-accent2']),
      text: v['--p-text'],
      dim: v['--p-dim'],
      grid: v['--p-grid'],
      surface: v['--p-surface'],
      glow: v['--p-glow'],
      bg: v['--p-bg'],
    };
  }

  /* ------------------------------------------------------------ drawing */

  const AXIS_W = 66;
  const PAD_T = 12;
  const PAD_B = 10;
  const MONO = "'IBM Plex Mono', ui-monospace, monospace";

  function range(candles: Candle[]): [number, number] {
    let min = Infinity;
    let max = -Infinity;
    for (const c of candles) {
      if (c.l < min) min = c.l;
      if (c.h > max) max = c.h;
    }
    const pad = (max - min) * 0.06 || max * 0.01 || 1;
    return [min - pad, max + pad];
  }

  function drawGrid(
    ctx: CanvasRenderingContext2D,
    pal: Pal,
    W: number,
    H: number,
    min: number,
    max: number,
    dec: number,
  ) {
    const plotW = W - AXIS_W;
    ctx.strokeStyle = pal.grid;
    ctx.lineWidth = 1;
    ctx.font = `10px ${MONO}`;
    ctx.textBaseline = 'middle';
    for (let i = 0; i <= 5; i++) {
      const y = PAD_T + ((H - PAD_T - PAD_B) * i) / 5;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(plotW, y);
      ctx.stroke();
      const price = max - ((max - min) * i) / 5;
      ctx.fillStyle = pal.dim;
      ctx.fillText(fmtNum(price, dec), plotW + 8, y);
    }
    for (let i = 1; i < 6; i++) {
      const x = (plotW * i) / 6;
      ctx.beginPath();
      ctx.moveTo(x, PAD_T);
      ctx.lineTo(x, H - PAD_B);
      ctx.stroke();
    }
  }

  function drawLastPrice(
    ctx: CanvasRenderingContext2D,
    pal: Pal,
    W: number,
    H: number,
    min: number,
    max: number,
  ) {
    const inst = market.selected;
    const plotW = W - AXIS_W;
    const y = PAD_T + ((max - inst.price) / (max - min)) * (H - PAD_T - PAD_B);
    const col = inst.up ? pal.up : pal.down;
    ctx.setLineDash([4, 4]);
    ctx.strokeStyle = rgba(col, 0.55);
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(plotW, y);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.fillStyle = rgba(col, 0.9);
    const label = fmtPrice(inst);
    ctx.font = `10px ${MONO}`;
    const tw = ctx.measureText(label).width + 10;
    ctx.fillRect(plotW + 1, y - 8, tw, 16);
    ctx.fillStyle = pal.bg;
    ctx.fillText(label, plotW + 6, y);
  }

  function drawVolume(
    ctx: CanvasRenderingContext2D,
    pal: Pal,
    candles: Candle[],
    W: number,
    H: number,
  ) {
    const plotW = W - AXIS_W;
    const n = candles.length;
    const cw = plotW / n;
    let vmax = 0;
    for (const c of candles) if (c.v > vmax) vmax = c.v;
    const vh = (H - PAD_T - PAD_B) * 0.14;
    for (let i = 0; i < n; i++) {
      const c = candles[i];
      const h = (c.v / (vmax || 1)) * vh;
      ctx.fillStyle = rgba(c.c >= c.o ? pal.up : pal.down, 0.16);
      ctx.fillRect(i * cw + cw * 0.18, H - PAD_B - h, Math.max(cw * 0.64, 1), h);
    }
  }

  function drawCandles(ctx: CanvasRenderingContext2D, pal: Pal, candles: Candle[], W: number, H: number) {
    const plotW = W - AXIS_W;
    const n = candles.length;
    const cw = plotW / n;
    const [min, max] = range(candles);
    const y = (v: number) => PAD_T + ((max - v) / (max - min)) * (H - PAD_T - PAD_B);
    drawGrid(ctx, pal, W, H, min, max, market.selected.def.dec);
    drawVolume(ctx, pal, candles, W, H);
    for (let i = 0; i < n; i++) {
      const c = candles[i];
      const up = c.c >= c.o;
      const col = up ? pal.up : pal.down;
      const x = i * cw + cw / 2;
      ctx.strokeStyle = rgba(col, 0.85);
      ctx.lineWidth = Math.max(cw * 0.12, 1);
      ctx.beginPath();
      ctx.moveTo(x, y(c.h));
      ctx.lineTo(x, y(c.l));
      ctx.stroke();
      ctx.fillStyle = rgba(col, up ? 0.9 : 0.95);
      const top = y(Math.max(c.o, c.c));
      const bh = Math.max(Math.abs(y(c.o) - y(c.c)), 1);
      ctx.fillRect(i * cw + cw * 0.18, top, Math.max(cw * 0.64, 1), bh);
    }
    drawLastPrice(ctx, pal, W, H, min, max);
    return { min, max };
  }

  function pathLine(ctx: CanvasRenderingContext2D, candles: Candle[], W: number, H: number, min: number, max: number) {
    const plotW = W - AXIS_W;
    const n = candles.length;
    const y = (v: number) => PAD_T + ((max - v) / (max - min)) * (H - PAD_T - PAD_B);
    ctx.beginPath();
    ctx.moveTo(0, y(candles[0].c));
    for (let i = 1; i < n; i++) ctx.lineTo((plotW * i) / (n - 1), y(candles[i].c));
  }

  function drawLineMode(
    ctx: CanvasRenderingContext2D,
    pal: Pal,
    candles: Candle[],
    W: number,
    H: number,
    fill: 'none' | 'area' | 'mountain',
  ) {
    const [min, max] = range(candles);
    drawGrid(ctx, pal, W, H, min, max, market.selected.def.dec);
    const reduced = aesthetics.reduced;
    if (fill === 'area') {
      pathLine(ctx, candles, W, H, min, max);
      const grad = ctx.createLinearGradient(0, PAD_T, 0, H - PAD_B);
      grad.addColorStop(0, rgba(pal.accent, 0.28));
      grad.addColorStop(1, rgba(pal.accent, 0));
      ctx.save();
      ctx.lineTo(W - AXIS_W, H - PAD_B);
      ctx.lineTo(0, H - PAD_B);
      ctx.closePath();
      ctx.fillStyle = grad;
      ctx.fill();
      ctx.restore();
    }
    if (fill === 'mountain') {
      pathLine(ctx, candles, W, H, min, max);
      const grad = ctx.createLinearGradient(0, PAD_T, 0, H - PAD_B);
      grad.addColorStop(0, rgba(pal.accent2, 0.5));
      grad.addColorStop(0.6, rgba(pal.accent2, 0.12));
      grad.addColorStop(1, rgba(pal.accent2, 0));
      ctx.save();
      ctx.lineTo(W - AXIS_W, H - PAD_B);
      ctx.lineTo(0, H - PAD_B);
      ctx.closePath();
      ctx.fillStyle = grad;
      ctx.fill();
      ctx.restore();
    }
    pathLine(ctx, candles, W, H, min, max);
    const col = fill === 'mountain' ? pal.accent2 : pal.accent;
    ctx.strokeStyle = rgba(col, 0.95);
    ctx.lineWidth = fill === 'none' || fill === 'area' ? 1.6 : 2;
    if (!reduced) {
      ctx.shadowColor = rgba(col, 0.7);
      ctx.shadowBlur = fill === 'mountain' ? 14 : 8;
    }
    ctx.stroke();
    ctx.shadowBlur = 0;
    // live tip dot
    if (!reduced) {
      const plotW = W - AXIS_W;
      const n = candles.length;
      const yTip = PAD_T + ((max - candles[n - 1].c) / (max - min)) * (H - PAD_T - PAD_B);
      const pulse = 2.6 + Math.sin(performance.now() / 300) * 1.1;
      ctx.beginPath();
      ctx.arc(plotW, yTip, Math.max(pulse, 1.4), 0, Math.PI * 2);
      ctx.fillStyle = rgba(col, 0.9);
      ctx.shadowColor = rgba(col, 1);
      ctx.shadowBlur = 12;
      ctx.fill();
      ctx.shadowBlur = 0;
    }
    drawLastPrice(ctx, pal, W, H, min, max);
    return { min, max };
  }

  function drawHeatmap(ctx: CanvasRenderingContext2D, pal: Pal, W: number, H: number) {
    const items = market.instruments;
    const cols = W > H * 1.6 ? 6 : 5;
    const rows = Math.ceil(items.length / cols);
    const gap = 6;
    const tw = (W - gap * (cols + 1)) / cols;
    const th = (H - gap * (rows + 1)) / rows;
    const mid = mix(pal.up, pal.down, 0.5);
    ctx.textBaseline = 'alphabetic';
    items.forEach((inst, i) => {
      const cx = gap + (i % cols) * (tw + gap);
      const cy = gap + Math.floor(i / cols) * (th + gap);
      const t = Math.min(Math.abs(inst.changePct) / 2.5, 1);
      const base = inst.up ? pal.up : pal.down;
      const col = mix(mix(pal.up, pal.down, 0.5), base, 0.25 + t * 0.75);
      ctx.fillStyle = rgba(col, 0.16 + t * 0.5);
      ctx.beginPath();
      ctx.roundRect(cx, cy, tw, th, 6);
      ctx.fill();
      if (market.selected === inst) {
        ctx.strokeStyle = rgba(pal.accent, 0.9);
        ctx.lineWidth = 1.5;
        ctx.stroke();
      }
      ctx.fillStyle = rgba(mix(col, hexToRgb(aesthetics.vars['--p-text']), 0.75), 1);
      ctx.font = `600 ${Math.min(th * 0.24, 15)}px ${MONO}`;
      ctx.fillText(inst.def.symbol, cx + 10, cy + th * 0.38);
      ctx.font = `${Math.min(th * 0.2, 12)}px ${MONO}`;
      ctx.fillStyle = rgba(base, 0.95);
      ctx.fillText(fmtPct(inst.changePct), cx + 10, cy + th * 0.66);
      ctx.fillStyle = rgba(mix(mid, base, 0.4), 0.8);
      ctx.font = `${Math.min(th * 0.16, 10)}px ${MONO}`;
      ctx.fillText(fmtPrice(inst), cx + 10, cy + th * 0.87);
    });
  }

  function drawDepth(ctx: CanvasRenderingContext2D, pal: Pal, W: number, H: number) {
    const { bids, asks } = market;
    const inst = market.selected;
    const plotW = W - AXIS_W;
    const midX = plotW / 2;
    const allPrices = [...bids.map((b) => b.price), ...asks.map((a) => a.price)];
    if (allPrices.length === 0) return;
    const min = Math.min(...allPrices);
    const max = Math.max(...allPrices);
    const maxDepth = Math.max(bids[bids.length - 1]?.depth ?? 1, asks[asks.length - 1]?.depth ?? 1);
    const y = (d: number) => PAD_T + (1 - d / maxDepth) * (H - PAD_T - PAD_B);
    drawGrid(ctx, pal, W, H, min, max, inst.def.dec);
    // bids (left, from mid toward left)
    const side = (
      levels: { price: number; depth: number }[],
      col: RGB,
      dir: 1 | -1,
    ) => {
      const lo = Math.min(...levels.map((l) => l.price));
      const hi = Math.max(...levels.map((l) => l.price));
      const x = (p: number) => midX + ((p - (dir === 1 ? hi : lo)) / (hi - lo || 1)) * (midX - 8) * -dir;
      ctx.beginPath();
      ctx.moveTo(midX, y(0));
      // stair-step depth curve from mid outward
      const sorted = [...levels].sort((a, b) => (dir === 1 ? b.price - a.price : a.price - b.price));
      ctx.lineTo(midX, y(sorted[0].depth));
      for (const l of sorted) {
        const lx = dir === 1 ? x(l.price) : x(l.price);
        ctx.lineTo(lx, y(l.depth));
      }
      ctx.lineTo(dir === 1 ? x(lo) : x(hi), y(0));
      ctx.closePath();
      ctx.fillStyle = rgba(col, 0.18);
      ctx.fill();
      ctx.strokeStyle = rgba(col, 0.85);
      ctx.lineWidth = 1.4;
      ctx.stroke();
    };
    side(bids, pal.up, 1);
    side(asks, pal.down, -1);
    // mid line
    ctx.setLineDash([3, 4]);
    ctx.strokeStyle = rgba(pal.accent, 0.5);
    ctx.beginPath();
    ctx.moveTo(midX, PAD_T);
    ctx.lineTo(midX, H - PAD_B);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.fillStyle = pal.dim;
    ctx.font = `10px ${MONO}`;
    ctx.textAlign = 'center';
    ctx.fillText(fmtPrice(inst), midX, H - PAD_B + 1);
    ctx.textAlign = 'left';
  }

  function drawCrosshair(
    ctx: CanvasRenderingContext2D,
    pal: Pal,
    candles: Candle[],
    W: number,
    H: number,
    min: number,
    max: number,
  ) {
    if (mouseX < 0 || mouseX > W - AXIS_W || mouseY < 0) return;
    const plotW = W - AXIS_W;
    const n = candles.length;
    const idx = Math.min(Math.max(Math.floor((mouseX / plotW) * n), 0), n - 1);
    const c = candles[idx];
    const up = c.c >= c.o;
    ctx.setLineDash([3, 3]);
    ctx.strokeStyle = rgba(pal.accent, 0.4);
    ctx.lineWidth = 1;
    const cx = ((idx + 0.5) / n) * plotW;
    ctx.beginPath();
    ctx.moveTo(cx, PAD_T);
    ctx.lineTo(cx, H - PAD_B);
    ctx.moveTo(0, mouseY);
    ctx.lineTo(plotW, mouseY);
    ctx.stroke();
    ctx.setLineDash([]);
    // price label on axis at mouse Y
    const price = max - ((mouseY - PAD_T) / (H - PAD_T - PAD_B)) * (max - min);
    ctx.font = `10px ${MONO}`;
    const label = fmtNum(price, market.selected.def.dec);
    ctx.fillStyle = rgba(pal.accent, 0.9);
    const tw = ctx.measureText(label).width + 10;
    ctx.fillRect(plotW + 1, mouseY - 8, tw, 16);
    ctx.fillStyle = pal.bg;
    ctx.textBaseline = 'middle';
    ctx.fillText(label, plotW + 6, mouseY);
    // OHLC readout box
    const dec = market.selected.def.dec;
    const lines = [
      `O ${fmtNum(c.o, dec)}  H ${fmtNum(c.h, dec)}`,
      `L ${fmtNum(c.l, dec)}  C ${fmtNum(c.c, dec)}`,
      `${fmtPct(((c.c - c.o) / c.o) * 100)}  VOL ${fmtCompact(c.v)}`,
    ];
    ctx.font = `10px ${MONO}`;
    const bw = Math.max(...lines.map((l) => ctx.measureText(l).width)) + 18;
    const bx = 10;
    const by = PAD_T + 6;
    ctx.fillStyle = rgba(hexToRgb(aesthetics.vars['--p-surface2']), 0.92);
    ctx.strokeStyle = rgba(pal.accent, 0.35);
    ctx.beginPath();
    ctx.roundRect(bx, by, bw, 3 * 15 + 8, 5);
    ctx.fill();
    ctx.stroke();
    lines.forEach((l, i) => {
      ctx.fillStyle = i === 2 ? rgba(up ? pal.up : pal.down, 1) : pal.text;
      ctx.fillText(l, bx + 9, by + 14 + i * 15);
    });
  }

  onMount(() => {
    const ctx = canvas.getContext('2d')!;
    let raf = 0;
    let W = 0;
    let H = 0;
    let dpr = 1;

    const resize = () => {
      dpr = window.devicePixelRatio || 1;
      W = host.clientWidth;
      H = host.clientHeight;
      canvas.width = Math.max(W * dpr, 1);
      canvas.height = Math.max(H * dpr, 1);
    };
    const ro = new ResizeObserver(resize);
    ro.observe(host);
    resize();

    const loop = () => {
      const pal = palette();
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, W, H);
      ctx.textBaseline = 'middle';
      ctx.textAlign = 'left';

      const mode = market.viz;
      const candles = market.getCandles();
      const reduced = aesthetics.reduced;

      // transition ease on instrument/timeframe/mode change
      let alpha = 1;
      if (!reduced) {
        const t = Math.min((performance.now() - animStart) / 320, 1);
        alpha = t * t * (3 - 2 * t);
      }
      ctx.globalAlpha = alpha;

      let bounds: { min: number; max: number } | null = null;
      if (mode === 'candles') bounds = drawCandles(ctx, pal, candles, W, H);
      else if (mode === 'line') bounds = drawLineMode(ctx, pal, candles, W, H, 'none');
      else if (mode === 'area') bounds = drawLineMode(ctx, pal, candles, W, H, 'area');
      else if (mode === 'mountain') bounds = drawLineMode(ctx, pal, candles, W, H, 'mountain');
      else if (mode === 'heatmap') drawHeatmap(ctx, pal, W, H);
      else if (mode === 'depth') drawDepth(ctx, pal, W, H);

      ctx.globalAlpha = 1;
      // Draw indicator overlays on the price pane.
      if (bounds && mode !== 'heatmap' && mode !== 'depth') {
        drawPriceOverlays(ctx, candles, W, H, bounds.min, bounds.max);
      }
      // Draw oscillator sub-pane (RSI/MACD/Stoch/ATR).
      if (mode !== 'heatmap' && mode !== 'depth') {
        drawSubPane(ctx, pal, candles, W, H);
      }
      if (bounds && mode !== 'heatmap' && mode !== 'depth') {
        drawCrosshair(ctx, pal, candles, W, H, bounds.min, bounds.max);
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
    };
  });
</script>

<section class="pk-panel pk-chart-wrap">
  <div class="pk-chart-head">
    <div class="pk-chart-title">
      <span class="pk-chart-sym">{market.selected.def.symbol}</span>
      <span
        class="pk-chart-price"
        class:flash-up={market.selected.flash === 'up'}
        class:flash-down={market.selected.flash === 'down'}
        class:pk-up={market.selected.up}
        class:pk-down={!market.selected.up}>{fmtPrice(market.selected)}</span
      >
      <span class="pk-mono" class:pk-up={market.selected.up} class:pk-down={!market.selected.up}
        >{fmtPct(market.selected.changePct)}</span
      >
      <span class="pk-chart-name"
        >{market.selected.def.name} · {market.selected.def.market}</span
      >
    </div>
    <div class="pk-switcher" role="tablist" aria-label="Visualization mode">
      {#each VIZ_MODES as mode (mode.id)}
        <button
          class="pk-btn"
          class:active={market.viz === mode.id}
          onclick={() => (market.viz = mode.id)}>{mode.label}</button
        >
      {/each}
    </div>
    <div class="pk-switcher" role="tablist" aria-label="Timeframe">
      {#each TIMEFRAMES as tf (tf)}
        <button
          class="pk-btn"
          class:active={market.timeframe === tf}
          onclick={() => market.setTimeframe(tf)}>{tf}</button
        >
      {/each}
    </div>
    <div class="pk-indicator-bar">
      <button class="pk-btn pk-indicator-trigger" onclick={() => (showIndicatorMenu = !showIndicatorMenu)}>
        ƒ Indicators
      </button>
      {#if indicators.filter((i) => i.active).length > 0}
        <div class="pk-indicator-chips">
          {#each indicators.filter((i) => i.active) as ind (ind.label)}
            <span class="pk-indicator-chip" style="--ind-color: {INDICATOR_COLORS[ind.kind] ?? '#4dc8ff'}">
              {ind.label}
              <button class="pk-indicator-remove" onclick={() => (ind.active = false)}>×</button>
            </span>
          {/each}
        </div>
      {/if}
      {#if showIndicatorMenu}
        <div class="pk-indicator-menu">
          <div class="pk-indicator-menu-head">
            <span>Technical indicators</span>
            <button onclick={() => (showIndicatorMenu = false)}>×</button>
          </div>
          <div class="pk-indicator-list">
            {#each indicators as ind (ind.label)}
              <label class="pk-indicator-option">
                <input type="checkbox" bind:checked={ind.active} />
                <span class="pk-indicator-color" style="background: {INDICATOR_COLORS[ind.kind] ?? '#4dc8ff'}"></span>
                <span>{ind.label}</span>
              </label>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>
  <div
    class="pk-canvas-host"
    bind:this={host}
    onmousemove={(e) => {
      const r = host.getBoundingClientRect();
      mouseX = e.clientX - r.left;
      mouseY = e.clientY - r.top;
    }}
    onmouseleave={() => {
      mouseX = -1;
      mouseY = -1;
    }}
    role="presentation"
  >
    <canvas bind:this={canvas}></canvas>
  </div>
</section>

<style>
  .pk-indicator-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    position: relative;
  }
  .pk-indicator-trigger {
    font-family: var(--font-mono);
    font-size: 0.6875rem;
    opacity: 0.7;
  }
  .pk-indicator-trigger:hover {
    opacity: 1;
  }
  .pk-indicator-chips {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .pk-indicator-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--ind-color, var(--p-accent)) 15%, transparent);
    border: 1px solid color-mix(in srgb, var(--ind-color, var(--p-accent)) 30%, transparent);
    color: var(--ind-color, var(--p-text));
    font-family: var(--font-mono);
    font-size: 0.625rem;
    letter-spacing: 0.04em;
  }
  .pk-indicator-remove {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: 0.875rem;
    line-height: 1;
    opacity: 0.6;
    padding: 0 0 0 2px;
  }
  .pk-indicator-remove:hover {
    opacity: 1;
  }
  .pk-indicator-menu {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 6px;
    background: var(--p-surface1);
    border: 1px solid var(--p-border);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    z-index: 50;
    min-width: 200px;
  }
  .pk-indicator-menu-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    border-bottom: 1px solid var(--p-border);
    font-family: var(--font-mono);
    font-size: 0.625rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--p-text-dim);
  }
  .pk-indicator-menu-head button {
    background: none;
    border: none;
    color: var(--p-text-dim);
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
  }
  .pk-indicator-list {
    display: flex;
    flex-direction: column;
    padding: 6px;
    max-height: 300px;
    overflow-y: auto;
  }
  .pk-indicator-option {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.75rem;
    color: var(--p-text);
  }
  .pk-indicator-option:hover {
    background: var(--p-surface2);
  }
  .pk-indicator-option input {
    accent-color: var(--p-accent);
  }
  .pk-indicator-color {
    width: 10px;
    height: 2px;
    border-radius: 1px;
  }
</style>
