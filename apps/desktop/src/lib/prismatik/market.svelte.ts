/**
 * PRISMATIK — live market store.
 *
 * Every value here is an observation or it is `null`. There is no random walk,
 * no seeded candle series, no synthesized order book and no scenario headlines.
 * When a provider cannot answer, the field stays null and `feedMessage` says
 * why, so a panel renders an explicit gap instead of plausible noise.
 *
 * The symbol universe is the user's tracked list, owned by the backend
 * (`tracking.rs`). Nothing is quoted that the user has not chosen to track.
 */

export type InstrumentKind = 'equity' | 'crypto';
export type Timeframe = '1m' | '5m' | '15m' | '1H' | '1D';
/**
 * `heatmap` grids the tracked universe by observed change — it needs quotes but
 * no history. There is no `depth` mode: no connected provider serves a book.
 */
export type VizMode = 'candles' | 'line' | 'area' | 'mountain' | 'heatmap';

/** `empty` = nothing tracked. `unavailable` = tracked, but nothing quotable. */
export type FeedMode = 'live' | 'degraded' | 'unavailable' | 'empty';

export interface TrackedInstrument {
  kind: InstrumentKind;
  symbol: string;
  providerId: string;
  name: string;
  market: string;
  decimals: number;
  addedAt: string;
}

export interface InstrumentSearchHit {
  kind: InstrumentKind;
  symbol: string;
  providerId: string;
  name: string;
  market: string;
  decimals: number;
  tracked: boolean;
}

export interface InstrumentSearchResult {
  hits: InstrumentSearchHit[];
  searchedProviders: string[];
  message: string;
}

export interface TerminalFeedQuote {
  symbol: string;
  price: number;
  changePct?: number | null;
  volume?: number | null;
  provider: string;
  observedAt: string;
}

export interface TerminalFeedSnapshot {
  mode: FeedMode;
  providers: string[];
  quotes: TerminalFeedQuote[];
  missing: string[];
  trackedCount: number;
  retrievedAt: string;
  message: string;
}

export interface Candle {
  /** bar start, epoch ms */
  t: number;
  o: number;
  h: number;
  l: number;
  c: number;
  v: number;
}

export const TIMEFRAMES: Timeframe[] = ['1m', '5m', '15m', '1H', '1D'];

export const VIZ_MODES: { id: VizMode; label: string }[] = [
  { id: 'candles', label: 'Candles' },
  { id: 'line', label: 'Line' },
  { id: 'area', label: 'Area' },
  { id: 'mountain', label: 'Mountain' },
  { id: 'heatmap', label: 'Heatmap' },
];

/* ------------------------------------------------------------------ */
/* formatting                                                          */
/* ------------------------------------------------------------------ */

/** Em-dash placeholder for every unobserved value, used everywhere. */
export const NO_VALUE = '—';

export function fmtPrice(inst: Instrument, v: number | null = inst.price): string {
  if (v === null) return NO_VALUE;
  return v.toLocaleString('en-US', {
    minimumFractionDigits: inst.decimals,
    maximumFractionDigits: inst.decimals,
  });
}

export function fmtNum(v: number | null, dec = 2): string {
  if (v === null) return NO_VALUE;
  return v.toLocaleString('en-US', { minimumFractionDigits: dec, maximumFractionDigits: dec });
}

export function fmtCompact(v: number | null): string {
  if (v === null) return NO_VALUE;
  if (v >= 1e9) return (v / 1e9).toFixed(2) + 'B';
  if (v >= 1e6) return (v / 1e6).toFixed(2) + 'M';
  if (v >= 1e3) return (v / 1e3).toFixed(1) + 'K';
  return v.toFixed(0);
}

export function fmtPct(v: number | null): string {
  if (v === null) return NO_VALUE;
  return (v >= 0 ? '+' : '') + v.toFixed(2) + '%';
}

export function fmtSigned(v: number | null, dec = 2): string {
  if (v === null) return NO_VALUE;
  return (v >= 0 ? '+' : '−') + fmtNum(Math.abs(v), dec);
}

export function fmtClock(t: number): string {
  const d = new Date(t);
  const p = (n: number) => String(n).padStart(2, '0');
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

/** Relative age of an observation — a quote's staleness is itself information. */
export function fmtAge(iso: string | null): string {
  if (!iso) return NO_VALUE;
  const ms = Date.now() - new Date(iso).getTime();
  if (!Number.isFinite(ms) || ms < 0) return NO_VALUE;
  if (ms < 60_000) return `${Math.floor(ms / 1000)}s ago`;
  if (ms < 3_600_000) return `${Math.floor(ms / 60_000)}m ago`;
  if (ms < 86_400_000) return `${Math.floor(ms / 3_600_000)}h ago`;
  return `${Math.floor(ms / 86_400_000)}d ago`;
}

/* ------------------------------------------------------------------ */
/* instrument                                                          */
/* ------------------------------------------------------------------ */

/**
 * A tracked instrument and whatever has actually been observed about it.
 *
 * `price === null` is the normal state before the first successful poll and
 * after a provider drops — it is never backfilled with a guess.
 */
export class Instrument {
  readonly kind: InstrumentKind;
  readonly symbol: string;
  readonly providerId: string;
  readonly name: string;
  readonly market: string;
  readonly decimals: number;

  price = $state<number | null>(null);
  prevPrice = $state<number | null>(null);
  changePct = $state<number | null>(null);
  volume = $state<number | null>(null);
  provider = $state<string | null>(null);
  observedAt = $state<string | null>(null);
  flash = $state<'up' | 'down' | null>(null);

  /** bump counter — fine-grained invalidation for canvas consumers */
  tick = $state(0);
  /** observed closes only, in arrival order; never pre-seeded */
  history: number[] = [];

  private flashTimer: ReturnType<typeof setTimeout> | undefined;

  /** true once at least one real quote has landed */
  quoted = $derived(this.price !== null);
  up = $derived(this.changePct !== null && this.changePct >= 0);

  constructor(tracked: TrackedInstrument) {
    this.kind = tracked.kind;
    this.symbol = tracked.symbol;
    this.providerId = tracked.providerId;
    this.name = tracked.name;
    this.market = tracked.market;
    this.decimals = tracked.decimals;
  }

  applyQuote(quote: TerminalFeedQuote): void {
    this.prevPrice = this.price;
    this.price = quote.price;
    this.changePct = typeof quote.changePct === 'number' ? quote.changePct : null;
    this.volume = typeof quote.volume === 'number' ? quote.volume : null;
    this.provider = quote.provider;
    this.observedAt = quote.observedAt;
    this.history.push(quote.price);
    if (this.history.length > 240) this.history.shift();
    this.tick++;
    if (this.prevPrice !== null && this.prevPrice !== quote.price) {
      this.flash = quote.price >= this.prevPrice ? 'up' : 'down';
      clearTimeout(this.flashTimer);
      this.flashTimer = setTimeout(() => (this.flash = null), 480);
    }
  }

  /** Provider stopped answering for this symbol — drop the value, keep the row. */
  markUnquoted(): void {
    this.price = null;
    this.prevPrice = null;
    this.changePct = null;
    this.volume = null;
    this.provider = null;
    this.tick++;
  }
}

/* ------------------------------------------------------------------ */
/* engine                                                              */
/* ------------------------------------------------------------------ */

async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke, isTauri } = await import('@tauri-apps/api/core');
  if (!isTauri()) throw new Error('desktop runtime required');
  return invoke<T>(command, args);
}

class MarketStore {
  tracked = $state<TrackedInstrument[]>([]);
  instruments = $state<Instrument[]>([]);
  bySymbol = new Map<string, Instrument>();

  selectedSymbol = $state<string | null>(null);
  timeframe = $state<Timeframe>('1D');
  viz = $state<VizMode>('candles');

  feedMode = $state<FeedMode>('empty');
  feedProviders = $state<string[]>([]);
  feedMessage = $state('Not started');
  feedRetrievedAt = $state<string | null>(null);
  feedMissing = $state<string[]>([]);
  /** true while a poll is in flight — lets surfaces distinguish empty from loading */
  loading = $state(false);
  /**
   * Round-trip of the last quote poll, in milliseconds. `null` until one has
   * completed. This is the measured cost of the IPC call plus whatever the
   * provider took — not a synthetic figure, and not a claim about exchange
   * latency, which nothing here can observe.
   */
  lastPollMs = $state<number | null>(null);
  ready = $state(false);

  selected = $derived(
    this.selectedSymbol ? (this.bySymbol.get(this.selectedSymbol) ?? null) : null,
  );
  hasTracked = $derived(this.tracked.length > 0);
  quotedCount = $derived(this.instruments.filter((i) => i.quoted).length);

  private candles = $state<Record<string, Candle[] | null>>({});
  private candleErrors = $state<Record<string, string>>({});
  private pending = new Set<string>();
  private pollTimer: ReturnType<typeof setInterval> | undefined;

  byKind(kind: InstrumentKind): Instrument[] {
    return this.instruments.filter((i) => i.kind === kind);
  }

  select(symbol: string): void {
    if (this.bySymbol.has(symbol)) this.selectedSymbol = symbol;
  }

  setTimeframe(tf: Timeframe): void {
    this.timeframe = tf;
    const inst = this.selected;
    if (inst) void this.loadCandles(inst, tf);
  }

  /* -------------------------------------------------------------- */
  /* tracked list                                                    */
  /* -------------------------------------------------------------- */

  async loadTracked(): Promise<void> {
    try {
      const rows = await tauriInvoke<TrackedInstrument[]>('get_tracked_instruments');
      this.applyTracked(rows);
    } catch (error) {
      this.feedMode = 'unavailable';
      this.feedMessage = error instanceof Error ? error.message : String(error);
    } finally {
      this.ready = true;
    }
  }

  private applyTracked(rows: TrackedInstrument[]): void {
    const next: Instrument[] = [];
    const nextBy = new Map<string, Instrument>();
    for (const row of rows) {
      // Reuse the existing instrument so observed history survives a reorder.
      const existing = this.bySymbol.get(row.symbol);
      const inst =
        existing && existing.providerId === row.providerId ? existing : new Instrument(row);
      next.push(inst);
      nextBy.set(inst.symbol, inst);
    }
    this.tracked = rows;
    this.instruments = next;
    this.bySymbol = nextBy;
    if (this.selectedSymbol && !nextBy.has(this.selectedSymbol)) this.selectedSymbol = null;
    if (!this.selectedSymbol && next.length > 0) this.selectedSymbol = next[0].symbol;
  }

  async track(hit: InstrumentSearchHit): Promise<void> {
    const rows = await tauriInvoke<TrackedInstrument[]>('add_tracked_instrument', {
      instrument: {
        kind: hit.kind,
        symbol: hit.symbol,
        providerId: hit.providerId,
        name: hit.name,
        market: hit.market,
        decimals: hit.decimals,
        addedAt: '',
      },
    });
    this.applyTracked(rows);
    await this.refresh();
  }

  async untrack(inst: Instrument): Promise<void> {
    const rows = await tauriInvoke<TrackedInstrument[]>('remove_tracked_instrument', {
      kind: inst.kind,
      providerId: inst.providerId,
    });
    this.applyTracked(rows);
  }

  async reorder(symbols: string[]): Promise<void> {
    const ids = symbols
      .map((symbol) => this.bySymbol.get(symbol)?.providerId)
      .filter((id): id is string => typeof id === 'string');
    const rows = await tauriInvoke<TrackedInstrument[]>('reorder_tracked_instruments', {
      providerIds: ids,
    });
    this.applyTracked(rows);
  }

  async search(query: string): Promise<InstrumentSearchResult> {
    return tauriInvoke<InstrumentSearchResult>('search_instruments', { query });
  }

  /* -------------------------------------------------------------- */
  /* quotes                                                          */
  /* -------------------------------------------------------------- */

  async refresh(): Promise<void> {
    this.loading = true;
    const startedAt = performance.now();
    try {
      const snapshot = await tauriInvoke<TerminalFeedSnapshot>('get_terminal_feed');
      this.applyFeed(snapshot);
      this.lastPollMs = Math.round(performance.now() - startedAt);
    } catch (error) {
      this.feedMode = 'unavailable';
      this.feedProviders = [];
      this.feedMessage = error instanceof Error ? error.message : String(error);
      // A failed poll has no meaningful latency to report.
      this.lastPollMs = null;
      for (const inst of this.instruments) inst.markUnquoted();
    } finally {
      this.loading = false;
      this.ready = true;
    }
  }

  applyFeed(snapshot: TerminalFeedSnapshot): void {
    this.feedMode = snapshot.mode;
    this.feedProviders = snapshot.providers;
    this.feedMessage = snapshot.message;
    this.feedRetrievedAt = snapshot.retrievedAt;
    this.feedMissing = snapshot.missing;

    const quoted = new Set<string>();
    for (const quote of snapshot.quotes) {
      const inst = this.bySymbol.get(quote.symbol.toUpperCase());
      if (!inst) continue;
      inst.applyQuote(quote);
      quoted.add(inst.symbol);
    }
    for (const inst of this.instruments) {
      if (!quoted.has(inst.symbol)) inst.markUnquoted();
    }
  }

  /* -------------------------------------------------------------- */
  /* candles                                                         */
  /* -------------------------------------------------------------- */

  candleKey(inst: Instrument, tf: Timeframe = this.timeframe): string {
    return `${inst.kind}:${inst.providerId}:${tf}`;
  }

  /** `undefined` = never requested, `null` = requested and unavailable. */
  getCandles(inst: Instrument | null, tf: Timeframe = this.timeframe): Candle[] | null | undefined {
    if (!inst) return null;
    return this.candles[this.candleKey(inst, tf)];
  }

  candleError(inst: Instrument | null, tf: Timeframe = this.timeframe): string | null {
    if (!inst) return null;
    return this.candleErrors[this.candleKey(inst, tf)] ?? null;
  }

  /**
   * Fetch real OHLC history. Crypto goes to CoinGecko market-chart, equities to
   * the historical bars command. Nothing is synthesized on failure.
   */
  async loadCandles(inst: Instrument, tf: Timeframe = this.timeframe): Promise<void> {
    const key = this.candleKey(inst, tf);
    if (this.pending.has(key) || this.candles[key]) return;
    this.pending.add(key);
    try {
      const rows =
        inst.kind === 'crypto'
          ? await tauriInvoke<HistoricalResult>('get_crypto_historical', {
              coinId: inst.providerId,
              vsCurrency: 'usd',
              days: daysForTimeframe(tf),
            })
          : await tauriInvoke<HistoricalResult>('get_historical_ohlcv', {
              symbol: inst.providerId,
              interval: intervalForTimeframe(tf),
              period: periodForTimeframe(tf),
            });
      const candles = normalizeCandles(rows);
      this.candles = { ...this.candles, [key]: candles.length > 0 ? candles : null };
      if (candles.length === 0) {
        this.candleErrors = { ...this.candleErrors, [key]: 'Provider returned no bars' };
      }
    } catch (error) {
      this.candles = { ...this.candles, [key]: null };
      this.candleErrors = {
        ...this.candleErrors,
        [key]: error instanceof Error ? error.message : String(error),
      };
    } finally {
      this.pending.delete(key);
    }
  }

  /* -------------------------------------------------------------- */
  /* lifecycle                                                       */
  /* -------------------------------------------------------------- */

  async start(intervalMs = 30_000): Promise<void> {
    await this.loadTracked();
    await this.refresh();
    const inst = this.selected;
    if (inst) void this.loadCandles(inst);
    if (this.pollTimer) return;
    this.pollTimer = setInterval(() => void this.refresh(), intervalMs);
  }

  stop(): void {
    clearInterval(this.pollTimer);
    this.pollTimer = undefined;
  }
}

/* ------------------------------------------------------------------ */
/* candle normalization                                                */
/* ------------------------------------------------------------------ */

interface HistoricalRow {
  timestamp?: number | string;
  time?: number | string;
  open?: number | string;
  high?: number | string;
  low?: number | string;
  close?: number | string;
  volume?: number | string;
}

interface HistoricalResult {
  bars?: HistoricalRow[];
  rows?: HistoricalRow[];
  candles?: HistoricalRow[];
}

function num(v: number | string | undefined): number | null {
  if (v === undefined) return null;
  const n = typeof v === 'number' ? v : Number.parseFloat(v);
  return Number.isFinite(n) ? n : null;
}

/**
 * The two history commands return slightly different envelopes; normalize both
 * and drop any row that is missing a close, rather than interpolating one.
 */
function normalizeCandles(result: HistoricalResult): Candle[] {
  const rows = result.bars ?? result.rows ?? result.candles ?? [];
  const out: Candle[] = [];
  for (const row of rows) {
    const close = num(row.close);
    if (close === null) continue;
    const open = num(row.open) ?? close;
    const raw = row.timestamp ?? row.time;
    const t = typeof raw === 'number' ? raw : raw ? Date.parse(raw) : Number.NaN;
    if (!Number.isFinite(t)) continue;
    out.push({
      t: t < 1e12 ? t * 1000 : t,
      o: open,
      h: num(row.high) ?? Math.max(open, close),
      l: num(row.low) ?? Math.min(open, close),
      c: close,
      v: num(row.volume) ?? 0,
    });
  }
  out.sort((a, b) => a.t - b.t);
  return out;
}

function daysForTimeframe(tf: Timeframe): number {
  switch (tf) {
    case '1m':
    case '5m':
      return 1;
    case '15m':
      return 7;
    case '1H':
      return 30;
    case '1D':
      return 365;
  }
}

function intervalForTimeframe(tf: Timeframe): string {
  switch (tf) {
    case '1m':
      return '1m';
    case '5m':
      return '5m';
    case '15m':
      return '15m';
    case '1H':
      return '60m';
    case '1D':
      return '1d';
  }
}

function periodForTimeframe(tf: Timeframe): string {
  switch (tf) {
    case '1m':
    case '5m':
      return '1d';
    case '15m':
      return '5d';
    case '1H':
      return '1mo';
    case '1D':
      return '1y';
  }
}

export const market = new MarketStore();
