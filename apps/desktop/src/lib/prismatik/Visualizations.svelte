<script lang="ts">
  /**
   * Visualization surface.
   *
   * Every view declares the real input it needs. If that input is not available
   * the view says which provider or computation is missing instead of rendering
   * shaped noise — a chart of `Math.random()` is indistinguishable from a chart
   * of a signal, which makes the whole surface untrustworthy.
   */
  import { onMount, untrack } from 'svelte';
  import { invoke, isTauri } from '@tauri-apps/api/core';
  import { market, type Candle } from './market.svelte';
  import NoData from './NoData.svelte';
  import { createGravityField, type AssetNode } from '../viz/gravity-field-3d';
  import { createYieldCurveSurface } from '../viz/yield-curve-3d';
  import { renderAccuracyAtlas, type AccuracyCell } from '../viz/accuracy-atlas-2d';
  import { renderPortfolioSunburst, type ExposureNode } from '../viz/portfolio-sunburst-2d';
  import { renderRegimeDiagram, type RegimeConfig } from '../viz/regime-diagram-2d';
  import { renderSurvivalCurve, type TrendSurvivalConfig } from '../viz/trend-survival-2d';
  import { createVolMountain, type SurfacePoint } from '../viz/vol-mountain-3d';
  import { createScenarioRiver, type SimPath } from '../viz/scenario-river-3d';

  interface VizDef {
    id: string;
    label: string;
    dim: '2d' | '3d';
    /** What this view is made of, shown when it cannot be drawn. */
    requires: string;
  }

  interface VizGroup {
    label: string;
    items: VizDef[];
  }

  /**
   * Only views with a real data path are listed. Still held back: options flow
   * and contagion (need an options chain and cross-venue stress), sector
   * rotation (needs a sector classification source), sentiment terrain, entity
   * network and constellation (need a news NLP pipeline), and crowding radar
   * (needs positioning data such as COT or short interest).
   */
  const VIZ_GROUPS: VizGroup[] = [
    {
      label: 'MARKET',
      items: [
        {
          id: 'gravity',
          label: 'Correlation Field',
          dim: '3d',
          requires: 'Two or more tracked instruments with overlapping price history',
        },
        {
          id: 'regime',
          label: 'Regime Map',
          dim: '2d',
          requires: 'Daily history long enough to classify and observe regime changes',
        },
        {
          id: 'survival',
          label: 'Regime Survival',
          dim: '2d',
          requires: 'At least one completed regime run in the selected instrument',
        },
        {
          id: 'vol',
          label: 'Realized Vol Surface',
          dim: '3d',
          requires: 'Daily history covering several lookback and horizon windows',
        },
        {
          id: 'scenario',
          label: 'Scenario River',
          dim: '3d',
          requires: 'A classified regime with enough return blocks to bootstrap',
        },
      ],
    },
    {
      label: 'PORTFOLIO',
      items: [
        {
          id: 'portfolio',
          label: 'Exposure Sunburst',
          dim: '2d',
          requires: 'Paper OMS positions with citable marks',
        },
      ],
    },
    {
      label: 'MACRO',
      items: [
        { id: 'yield', label: 'Yield Curve', dim: '3d', requires: 'FRED treasury series' },
      ],
    },
    {
      label: 'FORECAST',
      items: [
        {
          id: 'atlas',
          label: 'Accuracy Atlas',
          dim: '2d',
          requires: 'Resolved forecast candidates with scored cohorts',
        },
      ],
    },
  ];

  let {
    /**
     * Restrict this instance to specific view ids. Each workspace embeds the
     * views that belong to it, so there is no separate "visuals" destination to
     * navigate to and then navigate back from.
     */
    only = null,
  }: { only?: string[] | null } = $props();

  const ALL_VIZ = VIZ_GROUPS.flatMap((g) => g.items);
  const groups = $derived(
    only === null
      ? VIZ_GROUPS
      : VIZ_GROUPS.map((g) => ({ ...g, items: g.items.filter((i) => only.includes(i.id)) })).filter(
          (g) => g.items.length > 0,
        ),
  );
  /** A single embedded view needs no picker. */
  const showNav = $derived(groups.reduce((n, g) => n + g.items.length, 0) > 1);

  let vizContainer: HTMLDivElement;
  // Deliberately the initial value only: `only` is fixed per embed site, and
  // `activeViz` becomes user-owned state as soon as the picker is used.
  let activeViz = $state<string>(untrack(() => only?.[0]) ?? 'gravity');
  let currentViz: { destroy(): void } | null = null;
  let unavailable = $state<string | null>(null);
  let busy = $state(false);

  const activeDef = $derived(ALL_VIZ.find((v) => v.id === activeViz) ?? ALL_VIZ[0]);

  /* ---------------------------------------------------------------- */
  /* real-input builders                                              */
  /* ---------------------------------------------------------------- */

  /** Pearson correlation of log returns; null when the overlap is too short. */
  function correlate(a: number[], b: number[]): number | null {
    const n = Math.min(a.length, b.length);
    if (n < 20) return null;
    const ra: number[] = [];
    const rb: number[] = [];
    for (let i = 1; i < n; i++) {
      ra.push(Math.log(a[i] / a[i - 1]));
      rb.push(Math.log(b[i] / b[i - 1]));
    }
    const mean = (xs: number[]) => xs.reduce((s, x) => s + x, 0) / xs.length;
    const ma = mean(ra);
    const mb = mean(rb);
    let num = 0;
    let da = 0;
    let db = 0;
    for (let i = 0; i < ra.length; i++) {
      const xa = ra[i] - ma;
      const xb = rb[i] - mb;
      num += xa * xb;
      da += xa * xa;
      db += xb * xb;
    }
    const den = Math.sqrt(da * db);
    return den === 0 ? null : num / den;
  }

  interface RegimeSnapshotRow {
    symbol: string;
    regime: string | null;
    runLength: number;
    realizedVol: number;
    volPercentile: number;
    barCount: number;
  }

  async function buildGravity(): Promise<AssetNode[] | string> {
    const quoted = market.instruments.filter((i) => i.quoted);
    if (quoted.length < 2) return 'Track at least two instruments and wait for quotes.';

    // Real regime labels drive node colour. A failed sweep leaves them
    // unclassified rather than assigning a regime at random.
    let regimes = new Map<string, RegimeSnapshotRow>();
    if (isTauri()) {
      try {
        const rows = await invoke<RegimeSnapshotRow[]>('regime_snapshot');
        regimes = new Map(rows.map((r) => [r.symbol, r]));
      } catch {
        /* colouring degrades to unclassified; the field still renders */
      }
    }

    // Daily closes are what the correlation is computed from, so force 1D.
    await Promise.all(quoted.map((i) => market.loadCandles(i, '1D')));
    const closes = new Map<string, number[]>();
    for (const inst of quoted) {
      const series = market.getCandles(inst, '1D');
      if (Array.isArray(series) && series.length >= 21) {
        closes.set(inst.symbol, series.map((c: Candle) => c.c));
      }
    }
    if (closes.size < 2) {
      return 'Not enough overlapping daily history yet — at least 21 bars are needed per instrument.';
    }

    const nodes: AssetNode[] = [];
    for (const inst of quoted) {
      const own = closes.get(inst.symbol);
      if (!own) continue;
      const correlation: Record<string, number> = {};
      for (const other of quoted) {
        if (other.symbol === inst.symbol) continue;
        const theirs = closes.get(other.symbol);
        if (!theirs) continue;
        const r = correlate(own, theirs);
        if (r !== null) correlation[other.symbol] = r;
      }
      nodes.push({
        symbol: inst.symbol,
        name: inst.name,
        price: inst.price ?? 0,
        changePct: inst.changePct ?? 0,
        volume: inst.volume ?? 0,
        correlation,
        regime: regimes.get(inst.symbol)?.regime ?? 'unclassified',
        // Volatility percentile stands in for prominence: the instruments
        // moving most relative to their own history glow brightest.
        narrativeMomentum: regimes.get(inst.symbol)?.volPercentile ?? 0,
        sector: inst.kind === 'crypto' ? 'crypto' : 'tech',
      });
    }
    return nodes.length >= 2 ? nodes : 'Not enough instruments with usable history.';
  }

  interface PaperPositionView {
    symbol: string;
    quantity: string;
    marketValueMicros: string | null;
    unrealizedPnlMicros: string | null;
  }
  interface PaperOmsView {
    positions: PaperPositionView[];
    netMarketValueMicros: string;
    message: string;
  }

  async function buildPortfolio(): Promise<ExposureNode | string> {
    if (!isTauri()) return 'Desktop runtime required.';
    const oms = await invoke<PaperOmsView>('get_paper_oms');
    const marked = oms.positions.filter((p) => p.marketValueMicros !== null);
    if (marked.length === 0) return oms.message || 'No marked paper positions to chart.';

    const value = (p: PaperPositionView) => Math.abs(Number(p.marketValueMicros ?? 0));
    const total = marked.reduce((s, p) => s + value(p), 0);
    if (total === 0) return 'Marked positions have zero market value.';

    // Group by the tracked instrument's kind so the ring means something.
    const byKind = new Map<string, PaperPositionView[]>();
    for (const p of marked) {
      const kind = market.bySymbol.get(p.symbol)?.kind ?? 'other';
      const bucket = byKind.get(kind) ?? [];
      bucket.push(p);
      byKind.set(kind, bucket);
    }

    const pnlRatio = (p: PaperPositionView) => {
      const v = value(p);
      const pnl = Number(p.unrealizedPnlMicros ?? 0);
      return v === 0 ? 0 : pnl / v;
    };

    return {
      name: 'Paper book',
      weight: 1,
      pnl:
        marked.reduce((s, p) => s + Number(p.unrealizedPnlMicros ?? 0), 0) / (total || 1),
      children: [...byKind.entries()].map(([kind, rows]) => ({
        name: kind === 'crypto' ? 'Crypto' : kind === 'equity' ? 'Equity' : 'Other',
        weight: rows.reduce((s, p) => s + value(p), 0) / total,
        pnl: rows.reduce((s, p) => s + pnlRatio(p), 0) / rows.length,
        children: rows.map((p) => ({
          name: p.symbol,
          weight: value(p) / total,
          pnl: pnlRatio(p),
        })),
      })),
    };
  }

  interface YieldCurvePoint {
    maturity: string;
    maturityYears: number;
    yieldPct: number;
    date: string;
  }
  interface YieldCurveResult {
    date: string;
    points: YieldCurvePoint[];
    curveShape: string;
    source: string;
  }

  async function buildYield(): Promise<{ curves: YieldPointT[][]; labels: string[] } | string> {
    if (!isTauri()) return 'Desktop runtime required.';
    const result = await invoke<YieldCurveResult>('get_yield_curve');
    if (result.points.length === 0) return 'FRED returned no treasury observations.';
    // One observed curve — the surface renders a single ridge rather than
    // padding the time axis with days that were never fetched.
    const curve = result.points
      .slice()
      .sort((a, b) => a.maturityYears - b.maturityYears)
      .map((p) => ({ maturity: p.maturityYears, yield: p.yieldPct, date: 0 }));
    return { curves: [curve], labels: [result.date] };
  }

  type YieldPointT = { maturity: number; yield: number; date: number };

  interface CalibrationHealth {
    cohortId: string;
    model: string;
    target: string;
    horizonMinutes: number;
    sampleCount: number;
    overallBrierPpm: number | null;
    baselineBrierPpm: number | null;
    climatologyBrierPpm: number | null;
    skillPpm: number | null;
    skillSampleCount: number;
    state: string;
  }

  function horizonLabel(minutes: number): string {
    if (minutes < 60) return `${minutes}m`;
    if (minutes < 1440) return `${Math.round(minutes / 60)}h`;
    return `${Math.round(minutes / 1440)}d`;
  }

  /**
   * Skill against climatology — not against an earlier window of the same
   * model. A forecaster that has been consistently useless has zero drift and
   * would have scored perfectly well on the old baseline; measured against the
   * base rate it correctly reads as zero skill.
   */
  async function buildAtlas(): Promise<AccuracyCell[] | string> {
    if (!isTauri()) return 'Desktop runtime required.';
    const rows = await invoke<CalibrationHealth[]>('forecast_calibration_health');
    const scored = rows.filter((r) => r.skillPpm !== null && r.skillSampleCount > 0);
    if (scored.length === 0) {
      const awaiting = rows.filter((r) => r.sampleCount > 0).length;
      return awaiting > 0
        ? `${awaiting} cohort(s) have resolved forecasts but none recorded a base rate to score against. Only the statistical forecaster files one.`
        : 'No resolved forecasts yet. Cells appear once candidates are filed and their horizons mature.';
    }
    return scored.map((r) => ({
      assetClass: r.target,
      horizon: horizonLabel(r.horizonMinutes),
      skill: (r.skillPpm as number) / 1_000_000,
      sampleSize: r.skillSampleCount,
      calibrated: r.state === 'stable',
    }));
  }


  /* ---------------------------------------------------------------- */
  /* regime analytics                                                 */
  /* ---------------------------------------------------------------- */

  interface RegimeTransitionRow {
    from: string;
    to: string;
    count: number;
    probability: number;
    recent: boolean;
  }
  interface SurvivalPointRow {
    age: number;
    survival: number;
    lower: number;
    upper: number;
    atRisk: number;
    ended: number;
  }
  interface VolCellRow {
    lookback: number;
    horizon: number;
    realizedVol: number;
    forwardVol: number;
    ratio: number;
    sampleSize: number;
  }
  interface ScenarioPointRow {
    t: number;
    value: number;
  }
  interface InstrumentAnalytics {
    symbol: string;
    barCount: number;
    source: string;
    currentRegime: string | null;
    currentRunLength: number;
    realizedVol: number;
    volPercentile: number;
    transitions: {
      transitions: RegimeTransitionRow[];
      occupancy: [string, number][];
      totalChanges: number;
      current: string | null;
    };
    survival: {
      points: SurvivalPointRow[];
      medianLife: number | null;
      completedRuns: number;
      currentAge: number;
      nextBarSurvival: number | null;
    };
    volSurface: { cells: VolCellRow[]; spotVol: number };
    scenarios: {
      paths: { id: string; regime: string; points: ScenarioPointRow[]; terminal: number }[];
      realized: ScenarioPointRow[];
      regime: string | null;
      blockPool: number;
      p05Terminal: number;
      p50Terminal: number;
      p95Terminal: number;
    };
    message: string;
  }

  /** Analytics are expensive to fetch; cache per instrument for this instance. */
  let analyticsCache = new Map<string, InstrumentAnalytics>();

  async function loadAnalytics(): Promise<InstrumentAnalytics | string> {
    if (!isTauri()) return 'Desktop runtime required.';
    const inst = market.selected;
    if (!inst) return 'Select a tracked instrument first.';
    const key = `${inst.kind}:${inst.providerId}`;
    const cached = analyticsCache.get(key);
    if (cached) return cached;
    const result = await invoke<InstrumentAnalytics>('analyze_instrument', {
      kind: inst.kind,
      providerId: inst.providerId,
      symbol: inst.symbol,
    });
    analyticsCache.set(key, result);
    return result;
  }

  const REGIME_LABELS: Record<string, string> = {
    calm_trending: 'Calm trending',
    calm_mean_revert: 'Calm mean-revert',
    volatile_trending: 'Volatile trending',
    volatile_mean_revert: 'Volatile mean-revert',
    crisis: 'Crisis',
  };
  const REGIME_COLORS: Record<string, string> = {
    calm_trending: '#34d399',
    calm_mean_revert: '#00f0ff',
    volatile_trending: '#fbbf24',
    volatile_mean_revert: '#f87171',
    crisis: '#ef4444',
  };

  function buildRegimeDiagram(a: InstrumentAnalytics, W: number, H: number): RegimeConfig | string {
    if (a.transitions.totalChanges === 0) {
      return a.currentRegime
        ? `${a.symbol} has stayed in ${REGIME_LABELS[a.currentRegime] ?? a.currentRegime} for its whole ${a.barCount}-bar history — no transitions observed yet.`
        : a.message;
    }
    const occupancy = new Map(a.transitions.occupancy);
    return {
      nodes: Object.keys(REGIME_LABELS).map((id) => ({
        id,
        label: REGIME_LABELS[id],
        active: a.currentRegime === id,
        duration: a.currentRegime === id ? a.currentRunLength : (occupancy.get(id) ?? 0),
        color: REGIME_COLORS[id],
      })),
      transitions: a.transitions.transitions.map((row) => ({
        from: row.from,
        to: row.to,
        probability: row.probability,
        recent: row.recent,
      })),
      width: W,
      height: H,
    };
  }

  function buildSurvival(
    a: InstrumentAnalytics,
    W: number,
    H: number,
  ): TrendSurvivalConfig | string {
    if (a.survival.points.length === 0) {
      return `No completed regime runs for ${a.symbol} yet — the current run has lasted ${a.survival.currentAge} bars and has not ended, so there is nothing to estimate from.`;
    }
    // Hazard is the complement of the one-step conditional survival.
    const hazard = a.survival.nextBarSurvival === null ? 0 : 1 - a.survival.nextBarSurvival;
    return {
      data: a.survival.points.map((p) => ({
        day: p.age,
        survivalProb: p.survival,
        lowerCI: p.lower,
        upperCI: p.upper,
      })),
      currentAge: a.survival.currentAge,
      medianLife: a.survival.medianLife ?? 0,
      hazardRate: hazard,
      regime: a.currentRegime ?? 'unclassified',
      width: W,
      height: H,
    };
  }

  function buildVolSurface(a: InstrumentAnalytics): SurfacePoint[] | string {
    if (a.volSurface.cells.length < 4) {
      return `${a.symbol} has ${a.barCount} daily bars — not enough to fill a lookback x horizon surface.`;
    }
    return a.volSurface.cells.map((cell) => ({
      x: cell.lookback,
      z: cell.horizon,
      height: cell.realizedVol,
    }));
  }

  function buildScenarios(a: InstrumentAnalytics): {
    paths: SimPath[];
    realizedPath: ScenarioPointRow[];
  } | string {
    if (a.scenarios.paths.length === 0) {
      return `Not enough return history for ${a.symbol} to bootstrap forward paths.`;
    }
    // The renderer colours by a coarse regime family, not the full label.
    const family = (regime: string): string => {
      if (regime === 'crisis') return 'crisis';
      if (regime.startsWith('volatile')) return 'volatile';
      return 'calm';
    };
    return {
      paths: a.scenarios.paths.map((path) => ({
        id: path.id,
        regime: family(path.regime),
        points: path.points.map((point) => ({
          t: point.t,
          value: point.value,
          // Terminal rank stands in for path likelihood: central paths are the
          // dense part of the fan and are drawn more prominently.
          probability:
            1 -
            Math.min(
              Math.abs(path.terminal - a.scenarios.p50Terminal) /
                Math.max(Math.abs(a.scenarios.p95Terminal - a.scenarios.p05Terminal), 1e-9),
              1,
            ),
        })),
      })),
      realizedPath: a.scenarios.realized,
    };
  }

  /* ---------------------------------------------------------------- */
  /* lifecycle                                                        */
  /* ---------------------------------------------------------------- */

  async function initViz(): Promise<void> {
    if (!vizContainer || !vizContainer.clientWidth) return;
    if (currentViz) {
      currentViz.destroy();
      currentViz = null;
    }
    vizContainer.innerHTML = '';
    unavailable = null;
    busy = true;

    const W = vizContainer.clientWidth;
    const H = vizContainer.clientHeight;
    if (W < 10 || H < 10) {
      busy = false;
      return;
    }

    try {
      switch (activeViz) {
        case 'gravity': {
          const nodes = await buildGravity();
          if (typeof nodes === 'string') {
            unavailable = nodes;
            break;
          }
          currentViz = createGravityField(vizContainer, {
            nodes,
            onNodeClick: (symbol: string) => market.select(symbol),
          });
          break;
        }
        case 'regime': {
          const a = await loadAnalytics();
          if (typeof a === 'string') {
            unavailable = a;
            break;
          }
          const config = buildRegimeDiagram(a, W, H);
          if (typeof config === 'string') {
            unavailable = config;
            break;
          }
          renderRegimeDiagram(vizContainer, config);
          break;
        }
        case 'survival': {
          const a = await loadAnalytics();
          if (typeof a === 'string') {
            unavailable = a;
            break;
          }
          const config = buildSurvival(a, W, H);
          if (typeof config === 'string') {
            unavailable = config;
            break;
          }
          renderSurvivalCurve(vizContainer, config);
          break;
        }
        case 'vol': {
          const a = await loadAnalytics();
          if (typeof a === 'string') {
            unavailable = a;
            break;
          }
          const surface = buildVolSurface(a);
          if (typeof surface === 'string') {
            unavailable = surface;
            break;
          }
          currentViz = createVolMountain(vizContainer, { surface });
          break;
        }
        case 'scenario': {
          const a = await loadAnalytics();
          if (typeof a === 'string') {
            unavailable = a;
            break;
          }
          const built = buildScenarios(a);
          if (typeof built === 'string') {
            unavailable = built;
            break;
          }
          currentViz = createScenarioRiver(vizContainer, built);
          break;
        }
        case 'portfolio': {
          const root = await buildPortfolio();
          if (typeof root === 'string') {
            unavailable = root;
            break;
          }
          renderPortfolioSunburst(vizContainer, { root, width: W, height: H });
          break;
        }
        case 'yield': {
          const data = await buildYield();
          if (typeof data === 'string') {
            unavailable = data;
            break;
          }
          currentViz = createYieldCurveSurface(vizContainer, data);
          break;
        }
        case 'atlas': {
          const cells = await buildAtlas();
          if (typeof cells === 'string') {
            unavailable = cells;
            break;
          }
          renderAccuracyAtlas(vizContainer, { cells, width: W, height: H });
          break;
        }
      }
    } catch (error) {
      unavailable = error instanceof Error ? error.message : String(error);
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    void initViz();
    const observer = new ResizeObserver(() => void initViz());
    if (vizContainer) observer.observe(vizContainer);
    return () => {
      observer.disconnect();
      currentViz?.destroy();
      currentViz = null;
    };
  });

  $effect(() => {
    void activeViz;
    void initViz();
  });
</script>

<section class="pk-viz" class:solo={!showNav}>
  {#if showNav}
  <nav class="pk-viz-nav" aria-label="Visualizations">
    {#each groups as group (group.label)}
      <div class="pk-viz-group">
        <span class="pk-viz-group-label">{group.label}</span>
        {#each group.items as item (item.id)}
          <button
            class="pk-viz-item"
            class:active={activeViz === item.id}
            onclick={() => (activeViz = item.id)}
          >
            {item.label}<small>{item.dim}</small>
          </button>
        {/each}
      </div>
    {/each}
  </nav>
  {/if}

  <div class="pk-viz-stage">
    <div class="pk-viz-canvas" bind:this={vizContainer}></div>
    {#if busy}
      <div class="pk-viz-overlay"><NoData title="Building {activeDef.label}" compact /></div>
    {:else if unavailable}
      <div class="pk-viz-overlay">
        <NoData title="{activeDef.label} unavailable" detail={unavailable}>
          <span class="pk-viz-req">Needs: {activeDef.requires}</span>
        </NoData>
      </div>
    {/if}
  </div>
</section>

<style>
  .pk-viz {
    display: grid;
    grid-template-columns: 170px minmax(0, 1fr);
    min-height: 0;
    height: 100%;
  }
  .pk-viz.solo {
    grid-template-columns: minmax(0, 1fr);
  }
  .pk-viz-nav {
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
    padding: 10px 8px;
    border-right: 1px solid var(--p-border);
  }
  .pk-viz-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .pk-viz-group-label {
    padding: 0 6px 4px;
    color: var(--p-dim);
    font-size: var(--fz-sm);
    letter-spacing: 0.14em;
    opacity: 0.65;
  }
  .pk-viz-item {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 6px;
    padding: 5px 8px;
    border: none;
    border-left: 2px solid transparent;
    border-radius: 4px;
    background: transparent;
    color: var(--p-text);
    cursor: pointer;
    font-family: var(--font-ui);
    font-size: var(--fz-sm);
    text-align: left;
  }
  .pk-viz-item:hover {
    background: var(--p-surface2);
  }
  .pk-viz-item.active {
    border-left-color: var(--p-accent);
    background: var(--p-surface2);
    color: var(--p-accent);
  }
  .pk-viz-item small {
    color: var(--p-dim);
    font-size: 9px;
    text-transform: uppercase;
  }
  .pk-viz-stage {
    position: relative;
    min-width: 0;
    min-height: 0;
  }
  .pk-viz-canvas {
    width: 100%;
    height: 100%;
  }
  .pk-viz-overlay {
    position: absolute;
    inset: 0;
    display: grid;
    background: color-mix(in srgb, var(--p-bg) 70%, transparent);
    place-items: center;
  }
  .pk-viz-req {
    color: var(--p-dim);
    font-size: var(--fz-sm);
    opacity: 0.8;
  }
</style>
