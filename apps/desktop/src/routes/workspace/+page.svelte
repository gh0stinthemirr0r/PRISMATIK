<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import {
    WorkspaceShell,
    CommandPalette,
    RateBudgetMeter,
    Metric,
    EvidenceChip,
    CoverageBadge,
    StaleDataMarker,
    PriceChart,
    VirtualList,
    GlassPanel,
    TelemetryBadge,
    ViewSwitcher,
    Sparkline,
    CorrelationMatrix,
    type CommandItem,
  } from "@prismatik/ui";
  import type { CandlestickPoint } from "@prismatik/chart-contracts";
  import ExperiencesNav from "$lib/ExperiencesNav.svelte";

  type PanelId = "overview" | "watchlist" | "scanner" | "providers" | "alerts";

  type ScannerFilter = { id: string; name: string; query: string };
  type AlertRule = {
    id: string;
    symbol: string;
    change24hAbove: number;
    enabled: boolean;
  };

  type UiCryptoQuote = {
    coingeckoId: string;
    symbol: string;
    name: string;
    price: string;
    change24hPct: string | null;
    volume24h: string | null;
    marketCap: string | null;
    provider: string;
    eventTime: string;
    retrievedAt: string;
    quality: number;
  };

  type UiCryptoGlobal = {
    btcDominance: string | null;
    ethDominance: string | null;
    totalMarketCapUsd: string | null;
    totalVolumeUsd: string | null;
    provider: string;
    retrievedAt: string;
  };

  type UiTrendingCoin = {
    coingeckoId: string;
    symbol: string;
    name: string;
    marketCapRank: number | null;
  };

  type UiCryptoChart = {
    coingeckoId: string;
    provider: string;
    retrievedAt: string;
    interval: string;
    candles: CandlestickPoint[];
  };

  type UiCoinDetail = {
    coingeckoId: string;
    symbol: string;
    name: string;
    marketCapRank: number | null;
    homepage: string | null;
    description: string | null;
    categories: string[];
    provider: string;
    retrievedAt: string;
  };

  type UiCryptoCategory = {
    id: string;
    name: string;
    marketCap: string | null;
    change24hPct: string | null;
    top3Coins: string[];
  };

  type UiCryptoExchange = {
    id: string;
    name: string;
    trustScore: number | null;
    tradeVolume24hBtc: string | null;
    country: string | null;
  };

  type WatchRow = {
    coingeckoId: string;
    symbol: string;
    name: string;
    price: string;
    change: string;
    volume: string;
    provider: string;
    asOf: string;
    evidence: "confirmed" | "uncertain" | "contradicted";
  };

  const PREVIEW_AS_OF = "2026-07-27T09:30:00Z";
  const PREVIEW_MARKETS: UiCryptoQuote[] = [
    {
      coingeckoId: "bitcoin",
      symbol: "btc",
      name: "Bitcoin",
      price: "118742.16",
      change24hPct: "2.84",
      volume24h: "48394281722",
      marketCap: "2363124000000",
      provider: "coingecko-cassette",
      eventTime: PREVIEW_AS_OF,
      retrievedAt: PREVIEW_AS_OF,
      quality: 0.99,
    },
    {
      coingeckoId: "ethereum",
      symbol: "eth",
      name: "Ethereum",
      price: "3826.41",
      change24hPct: "4.18",
      volume24h: "28715492018",
      marketCap: "461729000000",
      provider: "coingecko-cassette",
      eventTime: PREVIEW_AS_OF,
      retrievedAt: PREVIEW_AS_OF,
      quality: 0.98,
    },
    {
      coingeckoId: "solana",
      symbol: "sol",
      name: "Solana",
      price: "192.84",
      change24hPct: "6.72",
      volume24h: "6738164200",
      marketCap: "101338000000",
      provider: "coingecko-cassette",
      eventTime: PREVIEW_AS_OF,
      retrievedAt: PREVIEW_AS_OF,
      quality: 0.96,
    },
    {
      coingeckoId: "chainlink",
      symbol: "link",
      name: "Chainlink",
      price: "19.642",
      change24hPct: "-1.26",
      volume24h: "936482100",
      marketCap: "13397000000",
      provider: "coingecko-cassette",
      eventTime: PREVIEW_AS_OF,
      retrievedAt: PREVIEW_AS_OF,
      quality: 0.94,
    },
    {
      coingeckoId: "avalanche-2",
      symbol: "avax",
      name: "Avalanche",
      price: "28.174",
      change24hPct: "1.93",
      volume24h: "582917400",
      marketCap: "11934000000",
      provider: "coingecko-cassette",
      eventTime: PREVIEW_AS_OF,
      retrievedAt: PREVIEW_AS_OF,
      quality: 0.93,
    },
  ];
  const PREVIEW_GLOBAL: UiCryptoGlobal = {
    btcDominance: "61.7",
    ethDominance: "12.1",
    totalMarketCapUsd: "3836000000000",
    totalVolumeUsd: "178420000000",
    provider: "coingecko-cassette",
    retrievedAt: PREVIEW_AS_OF,
  };
  const PREVIEW_TRENDING: UiTrendingCoin[] = [
    { coingeckoId: "bitcoin", symbol: "btc", name: "Bitcoin", marketCapRank: 1 },
    { coingeckoId: "ethereum", symbol: "eth", name: "Ethereum", marketCapRank: 2 },
    { coingeckoId: "solana", symbol: "sol", name: "Solana", marketCapRank: 6 },
  ];
  const PREVIEW_CATEGORIES: UiCryptoCategory[] = [
    {
      id: "smart-contract-platform",
      name: "Smart Contract Platform",
      marketCap: "1084200000000",
      change24hPct: "3.48",
      top3Coins: ["ethereum", "solana", "avalanche-2"],
    },
    {
      id: "decentralized-finance-defi",
      name: "Decentralized Finance",
      marketCap: "164920000000",
      change24hPct: "2.17",
      top3Coins: ["chainlink", "aave", "uniswap"],
    },
    {
      id: "artificial-intelligence",
      name: "AI & Agent Networks",
      marketCap: "49810000000",
      change24hPct: "5.62",
      top3Coins: ["bittensor", "render-token", "near"],
    },
  ];
  const PREVIEW_EXCHANGES: UiCryptoExchange[] = [
    { id: "coinbase", name: "Coinbase Exchange", trustScore: 10, tradeVolume24hBtc: "48291", country: "United States" },
    { id: "kraken", name: "Kraken", trustScore: 10, tradeVolume24hBtc: "21842", country: "United States" },
    { id: "bitstamp", name: "Bitstamp", trustScore: 9, tradeVolume24hBtc: "8934", country: "Luxembourg" },
  ];

  function isTauriRuntime(): boolean {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  function previewChart(coingeckoId: string): UiCryptoChart {
    const seed = PREVIEW_MARKETS.findIndex((quote) => quote.coingeckoId === coingeckoId);
    const base = Number(PREVIEW_MARKETS[Math.max(0, seed)]?.price ?? 100);
    const shape = [
      0.934, 0.947, 0.941, 0.958, 0.965, 0.953, 0.971, 0.984, 0.978, 0.996,
      1.008, 1.001, 1.019, 1.028, 1.015, 1.036, 1.044, 1.031, 1.052, 1.061,
      1.049, 1.068, 1.079, 1.072,
    ];
    const candles = shape.map((factor, index) => {
      const close = base * factor;
      const open = base * (index === 0 ? factor - 0.006 : shape[index - 1]);
      return {
        time: 1_751_155_200 + index * 86_400,
        open,
        high: Math.max(open, close) * (1.006 + (index % 3) * 0.001),
        low: Math.min(open, close) * (0.994 - (index % 2) * 0.001),
        close,
      };
    });
    return {
      coingeckoId,
      provider: "coingecko-cassette",
      retrievedAt: PREVIEW_AS_OF,
      interval: "1d · 24 bars",
      candles,
    };
  }

  function previewDetail(coingeckoId: string): UiCoinDetail {
    const quote = PREVIEW_MARKETS.find((item) => item.coingeckoId === coingeckoId) ?? PREVIEW_MARKETS[0];
    const rank = PREVIEW_MARKETS.findIndex((item) => item.coingeckoId === quote.coingeckoId) + 1;
    return {
      coingeckoId: quote.coingeckoId,
      symbol: quote.symbol,
      name: quote.name,
      marketCapRank: rank,
      homepage: null,
      description: `${quote.name} market telemetry with deterministic cassette provenance and retrieval-time evidence.`,
      categories: quote.symbol === "btc" ? ["Digital store of value", "Proof of Work"] : ["Smart Contract Platform", "Layer 1"],
      provider: "coingecko-cassette",
      retrievedAt: PREVIEW_AS_OF,
    };
  }

  let paletteOpen = $state(false);
  let remainingRatio = $state(0.74);
  let selected = $state("BTC");
  let notice = $state<string | null>(null);
  let loadError = $state<string | null>(null);
  let loading = $state(true);
  let watchlist = $state<WatchRow[]>([]);
  let global = $state<UiCryptoGlobal | null>(null);
  let trending = $state<UiTrendingCoin[]>([]);
  let categories = $state<UiCryptoCategory[]>([]);
  let exchanges = $state<UiCryptoExchange[]>([]);
  let coinDetail = $state<UiCoinDetail | null>(null);
  let sourceLabel = $state("cassette");
  let chartCandles = $state<CandlestickPoint[]>([]);
  let chartMeta = $state<string | null>(null);
  let chartError = $state<string | null>(null);
  let searchHits = $state<CommandItem[]>([]);
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let nextPermitAt = $state<string | null>(null);
  let budgetNow = $state(Date.now());
  let savedWatchlistIds = $state<string[] | null>(null);
  let scannerQuery = $state("");
  let filterName = $state("");
  let savedFilters = $state<ScannerFilter[]>([]);
  let alertThreshold = $state(5);
  let alertRules = $state<AlertRule[]>([]);
  let dataMode = $state("cassette");
  let analyticsView = $state("correlation");

  function formatPrice(raw: string): string {
    const n = Number(raw);
    if (!Number.isFinite(n)) return raw;
    return n.toLocaleString(undefined, {
      maximumFractionDigits: n >= 100 ? 2 : n >= 1 ? 4 : 6,
    });
  }

  function formatChange(raw: string | null | undefined): string {
    if (raw == null || raw === "") return "—";
    const n = Number(raw);
    if (!Number.isFinite(n)) return raw;
    const sign = n > 0 ? "+" : "";
    return `${sign}${n.toFixed(2)}%`;
  }

  function formatCompact(raw: string | null | undefined): string {
    if (raw == null || raw === "") return "—";
    const n = Number(raw);
    if (!Number.isFinite(n)) return raw;
    if (n >= 1e12) return `${(n / 1e12).toFixed(2)}T`;
    if (n >= 1e9) return `${(n / 1e9).toFixed(1)}B`;
    if (n >= 1e6) return `${(n / 1e6).toFixed(0)}M`;
    return n.toLocaleString();
  }

  function formatDom(raw: string | null | undefined): string {
    if (raw == null) return "—";
    const n = Number(raw);
    if (!Number.isFinite(n)) return raw;
    return n.toFixed(1);
  }

  function toWatchRow(q: UiCryptoQuote): WatchRow {
    return {
      coingeckoId: q.coingeckoId,
      symbol: q.symbol.toUpperCase(),
      name: q.name,
      price: formatPrice(q.price),
      change: formatChange(q.change24hPct),
      volume: formatCompact(q.volume24h),
      provider: q.provider,
      asOf: q.retrievedAt,
      evidence: q.quality >= 0.9 ? "confirmed" : "uncertain",
    };
  }

  async function loadChart(coingeckoId: string) {
    chartError = null;
    if (!isTauriRuntime()) {
      const chart = previewChart(coingeckoId);
      chartCandles = chart.candles;
      chartMeta = `${chart.interval} · ${chart.provider}`;
      return;
    }
    try {
      const chart = await invoke<UiCryptoChart>("get_crypto_ohlc", {
        coingeckoId,
        days: 30,
      });
      chartCandles = chart.candles;
      chartMeta = `${chart.interval} · ${chart.provider}`;
    } catch (err) {
      chartCandles = [];
      chartMeta = null;
      chartError = err instanceof Error ? err.message : String(err);
    }
  }

  async function loadDetail(coingeckoId: string) {
    if (!isTauriRuntime()) {
      coinDetail = previewDetail(coingeckoId);
      return;
    }
    try {
      coinDetail = await invoke<UiCoinDetail>("get_coin_detail", {
        coingeckoId,
      });
    } catch {
      coinDetail = null;
    }
  }

  async function refreshBudget() {
    if (!isTauriRuntime()) return;
    try {
      const b = await invoke<{
        remainingRatio: number;
        nextPermitAt: string | null;
        ready: boolean;
      }>("get_rate_budget_state");
      remainingRatio = b.remainingRatio;
      nextPermitAt = b.nextPermitAt;
    } catch {
      /* browser-only vite: keep local meter */
    }
  }

  async function spendBudget() {
    if (!isTauriRuntime()) {
      remainingRatio = Math.max(0.08, remainingRatio - 0.06);
      flash("Preview request admitted · deterministic cassette");
      return;
    }
    try {
      const r = await invoke<{
        kind: string;
        remainingRatio: number;
        nextPermitAt: string | null;
        message: string;
      }>("spend_rate_budget");
      remainingRatio = r.remainingRatio;
      nextPermitAt = r.nextPermitAt;
      flash(r.message);
    } catch (err) {
      flash(err instanceof Error ? err.message : String(err));
    }
  }

  function onPaletteQuery(q: string) {
    if (searchTimer) clearTimeout(searchTimer);
    const trimmed = q.trim();
    if (trimmed.length < 2) {
      searchHits = [];
      return;
    }
    searchTimer = setTimeout(async () => {
      if (!isTauriRuntime()) {
        searchHits = PREVIEW_MARKETS
          .filter((item) =>
            `${item.symbol} ${item.name} ${item.coingeckoId}`.toLowerCase().includes(trimmed.toLowerCase()),
          )
          .map((item) => ({
            id: `search-${item.coingeckoId}`,
            label: `${item.symbol.toUpperCase()} · ${item.name}`,
            hint: item.coingeckoId,
            group: "Search",
          }));
        return;
      }
      try {
        const hits = await invoke<UiTrendingCoin[]>("search_crypto", { query: trimmed });
        searchHits = hits.map((h) => ({
          id: `search-${h.coingeckoId}`,
          label: `${h.symbol.toUpperCase()} · ${h.name}`,
          hint: h.coingeckoId,
          group: "Search",
        }));
      } catch {
        searchHits = [];
      }
    }, 180);
  }

  async function loadDesktopPreferences() {
    if (!isTauriRuntime()) {
      savedWatchlistIds = PREVIEW_MARKETS.map((item) => item.coingeckoId);
      dataMode = "browser preview";
      sourceLabel = "deterministic cassette";
      return;
    }
    try {
      const [ids, filters, rules, mode] = await Promise.all([
        invoke<string[] | null>("get_watchlist"),
        invoke<ScannerFilter[]>("get_scanner_filters"),
        invoke<AlertRule[]>("list_alert_rules"),
        invoke<string>("get_data_mode"),
      ]);
      savedWatchlistIds = ids;
      savedFilters = filters;
      alertRules = rules;
      dataMode = mode;
      sourceLabel = mode;
    } catch {
      /* Browser preview keeps ephemeral defaults. */
    }
  }

  async function persistWatchlist(ids: string[]) {
    savedWatchlistIds = ids;
    if (!isTauriRuntime()) {
      flash("Watchlist updated for this preview session");
      return;
    }
    try {
      await invoke("save_watchlist", { coingeckoIds: ids });
    } catch (err) {
      flash(`Watchlist save failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function removeFromWatchlist(coingeckoId: string) {
    await persistWatchlist(
      (savedWatchlistIds ?? watchlist.map((row) => row.coingeckoId)).filter(
        (id) => id !== coingeckoId,
      ),
    );
    watchlist = watchlist.filter((row) => row.coingeckoId !== coingeckoId);
  }

  async function addToWatchlist(coingeckoId: string) {
    const ids = savedWatchlistIds ?? watchlist.map((row) => row.coingeckoId);
    if (!ids.includes(coingeckoId)) await persistWatchlist([...ids, coingeckoId]);
    flash(`${coingeckoId} added to watchlist`);
    await loadMarkets();
  }

  async function saveScannerFilter() {
    const name = filterName.trim();
    if (!name) return;
    const next = [
      ...savedFilters.filter((filter) => filter.name.toLowerCase() !== name.toLowerCase()),
      { id: crypto.randomUUID(), name, query: scannerQuery.trim() },
    ];
    if (!isTauriRuntime()) {
      savedFilters = next;
      filterName = "";
      flash(`Saved preview filter ${name}`);
      return;
    }
    try {
      await invoke("save_scanner_filters", { filters: next });
      savedFilters = next;
      filterName = "";
      flash(`Saved filter ${name}`);
    } catch (err) {
      flash(err instanceof Error ? err.message : String(err));
    }
  }

  async function addAlertRule() {
    const rule: AlertRule = {
      id: `btc-change-above-${alertThreshold}`,
      symbol: "BTC",
      change24hAbove: alertThreshold,
      enabled: true,
    };
    if (!isTauriRuntime()) {
      alertRules = [...alertRules.filter((item) => item.id !== rule.id), rule];
      flash("BTC preview alert saved");
      return;
    }
    try {
      alertRules = await invoke<AlertRule[]>("upsert_alert_rule", { rule });
      flash("BTC alert saved");
    } catch (err) {
      flash(err instanceof Error ? err.message : String(err));
    }
  }

  async function loadMarkets() {
    loading = watchlist.length === 0;
    loadError = null;
    try {
      const [markets, g, t, cats, xs] = isTauriRuntime()
        ? await Promise.all([
            invoke<UiCryptoQuote[]>("get_crypto_markets"),
            invoke<UiCryptoGlobal>("get_crypto_global"),
            invoke<UiTrendingCoin[]>("get_crypto_trending"),
            invoke<UiCryptoCategory[]>("get_crypto_categories"),
            invoke<UiCryptoExchange[]>("get_crypto_exchanges"),
          ])
        : [
            PREVIEW_MARKETS,
            PREVIEW_GLOBAL,
            PREVIEW_TRENDING,
            PREVIEW_CATEGORIES,
            PREVIEW_EXCHANGES,
          ];
      const allRows = markets.map(toWatchRow);
      if (savedWatchlistIds === null) {
        const defaults = allRows.map((row) => row.coingeckoId);
        await persistWatchlist(defaults);
      }
      watchlist = allRows.filter((row) => savedWatchlistIds?.includes(row.coingeckoId));
      global = g;
      trending = t;
      categories = cats;
      exchanges = xs;
      sourceLabel = isTauriRuntime() ? `coingecko ${dataMode}` : "preview · coingecko cassette";
      if (watchlist.length && !watchlist.some((r) => r.symbol === selected)) {
        selected = watchlist[0].symbol;
      }
      const focus = watchlist.find((r) => r.symbol === selected) ?? watchlist[0];
      if (focus) {
        await Promise.all([loadChart(focus.coingeckoId), loadDetail(focus.coingeckoId)]);
      }
      await refreshBudget();
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
      flash("Market load failed — is the Tauri shell running?");
    } finally {
      loading = false;
    }
  }

  async function selectSymbol(symbol: string) {
    selected = symbol;
    const row = watchlist.find((r) => r.symbol === symbol);
    if (row) {
      await Promise.all([loadChart(row.coingeckoId), loadDetail(row.coingeckoId)]);
    }
  }

  async function selectByCoingeckoId(coingeckoId: string) {
    const row = watchlist.find((r) => r.coingeckoId === coingeckoId);
    if (row) {
      await selectSymbol(row.symbol);
      setPanel("watchlist");
      return;
    }
    selected = coingeckoId.toUpperCase();
    await Promise.all([loadChart(coingeckoId), loadDetail(coingeckoId)]);
    setPanel("watchlist");
    flash(`Focused ${coingeckoId} (not on watchlist)`);
  }

  onMount(() => {
    let refreshTimer: ReturnType<typeof setInterval>;
    const budgetTimer = setInterval(() => (budgetNow = Date.now()), 1000);
    void (async () => {
      await loadDesktopPreferences();
      await loadMarkets();
      refreshTimer = setInterval(() => void loadMarkets(), 30_000);
    })();
    return () => {
      clearInterval(budgetTimer);
      if (refreshTimer) clearInterval(refreshTimer);
      if (searchTimer) clearTimeout(searchTimer);
    };
  });

  const panels: { id: PanelId; label: string }[] = [
    { id: "overview", label: "Overview" },
    { id: "watchlist", label: "Watchlist" },
    { id: "scanner", label: "Scanner" },
    { id: "providers", label: "Providers" },
    { id: "alerts", label: "Alerts" },
  ];

  const analyticsViews = [
    { id: "correlation", label: "Correlation" },
    { id: "trends", label: "Trends" },
    { id: "anomaly", label: "Anomaly matrix" },
    { id: "log", label: "Real-time log" },
  ];

  const telemetryTrend = [
    64210, 64582, 64191, 64844, 65120, 64982, 65446, 65902, 65631, 66208,
    66740, 66518, 67102, 67420, 67266, 68044, 68412, 68108, 68792, 69240,
    68944, 69518, 70184, 69966, 70642, 71208, 70974, 71630, 72184, 72418,
  ];

  const telemetryLabels = [
    "06:00", "06:30", "07:00", "07:30", "08:00", "08:30", "09:00", "09:30",
    "10:00", "10:30", "11:00", "11:30", "12:00", "12:30", "13:00", "13:30",
    "14:00", "14:30", "15:00", "15:30", "16:00", "16:30", "17:00", "17:30",
    "18:00", "18:30", "19:00", "19:30", "20:00", "20:30",
  ];

  const correlationNodes = [
    { id: "btc", label: "BTC", value: "+1.00", x: 49, y: 48, tone: "cyan" as const },
    { id: "eth", label: "ETH", value: "+0.84", x: 72, y: 28, tone: "violet" as const },
    { id: "sol", label: "SOL", value: "+0.72", x: 77, y: 70, tone: "violet" as const },
    { id: "dxy", label: "DXY", value: "−0.61", x: 22, y: 29, tone: "amber" as const },
    { id: "gold", label: "GOLD", value: "+0.33", x: 18, y: 72, tone: "emerald" as const },
    { id: "vix", label: "VIX", value: "−0.48", x: 48, y: 83, tone: "amber" as const },
    { id: "liq", label: "LIQ", value: "+0.67", x: 50, y: 15, tone: "emerald" as const },
  ];

  const correlationEdges = [
    { from: "btc", to: "eth", strength: 0.84 },
    { from: "btc", to: "sol", strength: 0.72 },
    { from: "btc", to: "dxy", strength: 0.61 },
    { from: "btc", to: "gold", strength: 0.33 },
    { from: "btc", to: "vix", strength: 0.48 },
    { from: "btc", to: "liq", strength: 0.67 },
    { from: "eth", to: "sol", strength: 0.79 },
    { from: "dxy", to: "gold", strength: 0.44 },
    { from: "vix", to: "sol", strength: 0.41 },
    { from: "liq", to: "eth", strength: 0.58 },
  ];

  const anomalySignals = [
    { factor: "Perp funding", asset: "BTC", score: "+2.41σ", intensity: 88, tone: "amber" },
    { factor: "Options skew", asset: "ETH", score: "−1.83σ", intensity: 69, tone: "violet" },
    { factor: "Spot volume", asset: "SOL", score: "+1.62σ", intensity: 61, tone: "cyan" },
    { factor: "Exchange flow", asset: "BTC", score: "−1.28σ", intensity: 48, tone: "emerald" },
    { factor: "Basis spread", asset: "ETH", score: "+0.91σ", intensity: 34, tone: "cyan" },
    { factor: "Realized vol", asset: "BTC", score: "+0.74σ", intensity: 28, tone: "violet" },
  ];

  const telemetryLog = [
    { time: "20:31:08.412", source: "MARKET", event: "BTC/USD quote normalized", meta: "72,418.21 · q=0.997" },
    { time: "20:31:08.391", source: "SIGNAL", event: "Liquidity impulse crossed threshold", meta: "+2.18σ · confirmed" },
    { time: "20:31:08.204", source: "CHAIN", event: "CoinGecko cassette evidence sealed", meta: "sha256:9e34…b71c" },
    { time: "20:31:07.988", source: "MODEL", event: "Regime posterior refreshed", meta: "expansion 0.64" },
    { time: "20:31:07.644", source: "RISK", event: "Portfolio stress surface recomputed", meta: "p95 −3.42%" },
    { time: "20:31:07.119", source: "SYSTEM", event: "Deterministic clock advanced", meta: "epoch +30s" },
  ];

  const activePanel = $derived(
    (["overview", "watchlist", "scanner", "providers", "alerts"].includes(
      $page.url.searchParams.get("panel") ?? "",
    )
      ? ($page.url.searchParams.get("panel") as PanelId)
      : "overview"),
  );

  const activeRow = $derived(
    watchlist.find((r) => r.symbol === selected) ?? watchlist[0],
  );

  const btcRow = $derived(watchlist.find((r) => r.symbol === "BTC"));
  const ethRow = $derived(watchlist.find((r) => r.symbol === "ETH"));
  const filteredTrending = $derived(
    trending.filter((hit) => {
      const query = scannerQuery.trim().toLowerCase();
      return (
        !query ||
        hit.symbol.toLowerCase().includes(query) ||
        hit.name.toLowerCase().includes(query) ||
        hit.coingeckoId.toLowerCase().includes(query)
      );
    }),
  );

  const commands = $derived<CommandItem[]>([
    { id: "overview", label: "Overview", hint: "g o", group: "Navigate" },
    { id: "watchlist", label: "Watchlist", hint: "g w", group: "Navigate" },
    { id: "scanner", label: "Scanner", hint: "g s", group: "Navigate" },
    { id: "providers", label: "Providers", hint: "g p", group: "Navigate" },
    { id: "alerts", label: "Alerts", hint: "g a", group: "Navigate" },
    ...watchlist.slice(0, 6).map((r) => ({
      id: `select-${r.symbol.toLowerCase()}`,
      label: `Focus ${r.symbol}`,
      hint: "asset",
      group: "Assets",
    })),
    { id: "reload", label: "Reload markets", hint: "r", group: "System" },
    { id: "home", label: "Leave workspace", hint: "esc", group: "System" },
    { id: "simulate-spend", label: "Spend rate budget", hint: "debug", group: "System" },
  ]);

  function setPanel(id: PanelId) {
    goto(`/workspace?panel=${id}`);
  }

  function flash(message: string) {
    notice = message;
    setTimeout(() => (notice = null), 2200);
  }

  function onSelect(command: CommandItem) {
    if (command.id === "home") {
      goto("/");
      return;
    }
    if (command.id === "reload") {
      void loadMarkets();
      return;
    }
    if (command.id === "simulate-spend") {
      void spendBudget();
      return;
    }
    if (command.id.startsWith("search-")) {
      void selectByCoingeckoId(command.id.replace("search-", ""));
      return;
    }
    if (command.id.startsWith("select-")) {
      void selectSymbol(command.id.replace("select-", "").toUpperCase());
      setPanel("watchlist");
      return;
    }
    if (panels.some((p) => p.id === command.id)) {
      setPanel(command.id as PanelId);
    }
  }

  function onGlobalKey(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      paletteOpen = true;
    }
  }

  const nextPermitLabel = $derived(
    nextPermitAt
      ? `${Math.max(0, Math.ceil((new Date(nextPermitAt).getTime() - budgetNow) / 1000))}s`
      : remainingRatio < 0.3
        ? `low ${(remainingRatio * 100).toFixed(0)}%`
        : "ready",
  );
</script>

<svelte:window onkeydown={onGlobalKey} />

<svelte:head>
  <title>Workspace · PRISMATIK</title>
</svelte:head>

<WorkspaceShell title="PRISMATIK" onOpenPalette={() => (paletteOpen = true)}>
  {#snippet sidebar()}
    <div class="rail-label">Experiences</div>
    <ExperiencesNav active="crypto" />
    <div class="rail-label rail-label--gap">Crypto panels</div>
    <nav class="nav" aria-label="Workspace">
      {#each panels as panel}
        <button
          type="button"
          class="nav__item"
          class:active={activePanel === panel.id}
          onclick={() => setPanel(panel.id)}
        >
          {panel.label}
        </button>
      {/each}
    </nav>
    <div class="rail-label rail-label--gap">Focus</div>
    <div class="focus-list">
      {#each watchlist.slice(0, 4) as row}
        <button
          type="button"
          class="focus"
          class:active={selected === row.symbol}
          onclick={() => {
            void selectSymbol(row.symbol);
            setPanel("watchlist");
          }}
        >
          <span>{row.symbol}</span>
          <span class="focus__chg" data-tone={row.change.startsWith("-") ? "down" : "up"}>
            {row.change}
          </span>
        </button>
      {/each}
    </div>
  {/snippet}

  {#snippet status()}
    <RateBudgetMeter
      label="CG"
      remainingRatio={remainingRatio}
      nextPermitLabel={nextPermitLabel}
    />
    <span class="sep">{sourceLabel}</span>
  {/snippet}

  <div class="canvas motion-fade-in">
    {#if loadError || dataMode === "chaos"}
      <div class="degraded" role="status">
        <strong>Degraded data mode</strong>
        <span>
          {loadError ?? "Chaos mode is active; results may include intentional failures."}
        </span>
      </div>
    {/if}
    {#if loading}
      <header class="page-head">
        <div>
          <h1>Loading markets</h1>
          <p>Replaying the CoinGecko demo cassette through the trusted core.</p>
        </div>
      </header>
      <div class="state-block" aria-busy="true">
        <div class="state-block__rule"></div>
        <p>Fetching markets · global · trending · categories · exchanges</p>
      </div>
    {:else if loadError && watchlist.length === 0}
      <header class="page-head">
        <div>
          <h1>Markets unavailable</h1>
          <p>{loadError}</p>
        </div>
        <button type="button" class="text-action" onclick={() => loadMarkets()}>Retry</button>
      </header>
      <div class="state-block state-block--error">
        <p>Run the Tauri shell so IPC can reach the cassette-backed adapter.</p>
      </div>
    {:else if activePanel === "overview"}
      <header class="page-head">
        <div>
          <div class="page-head__kicker">Digital asset intelligence / 20:31 UTC</div>
          <h1>Market pulse</h1>
          <p>
            A living read of price, liquidity, correlation, and evidence integrity.
          </p>
        </div>
        <div class="page-head__meta">
          <TelemetryBadge label="Streaming" tone="cyan" pulse />
          <EvidenceChip status="confirmed" label={`${sourceLabel} · searched`} />
          <CoverageBadge status="good" label={`n=${watchlist.length}`} />
          <button type="button" class="text-action" onclick={() => void loadMarkets()}>
            Refresh now
          </button>
        </div>
      </header>

      <section class="metric-strip" aria-label="Global metrics">
        <Metric
          label="BTC"
          value={btcRow ? btcRow.price.split(".")[0] : "—"}
          delta={btcRow?.change}
          series={[62, 65, 63, 68, 71, 70, 74, 78, 76, 82]}
          tone="cyan"
        />
        <Metric
          label="ETH"
          value={ethRow ? ethRow.price.split(".")[0] : "—"}
          delta={ethRow?.change}
          series={[51, 54, 57, 55, 59, 63, 61, 66, 68, 71]}
          tone="violet"
        />
        <Metric
          label="Dominance"
          value={formatDom(global?.btcDominance)}
          unit="%"
          delta={global ? `ETH ${formatDom(global.ethDominance)}%` : undefined}
          series={[52, 53, 52, 54, 55, 54, 56, 57, 56, 58]}
          tone="emerald"
        />
        <Metric
          label="Total cap"
          value={formatCompact(global?.totalMarketCapUsd)}
          delta={global ? `vol ${formatCompact(global.totalVolumeUsd)}` : undefined}
          series={[38, 42, 40, 46, 51, 49, 57, 61, 60, 66]}
          tone="amber"
        />
      </section>

      <GlassPanel eyebrow="Relationship engine" title="Cross-market signal topology" tone="violet" interactive={false}>
        {#snippet actions()}
          <ViewSwitcher bind:value={analyticsView} options={analyticsViews} ariaLabel="Analytics view" />
        {/snippet}

        <div class="analytics-frame">
          {#if analyticsView === "correlation"}
            <CorrelationMatrix nodes={correlationNodes} edges={correlationEdges} />
          {:else if analyticsView === "trends"}
            <div class="trend-view">
              <div class="trend-view__head">
                <div>
                  <span>BTC composite reference</span>
                  <strong>$72,418.21</strong>
                </div>
                <div class="trend-view__delta">+12.78% <span>30D</span></div>
              </div>
              <Sparkline
                values={telemetryTrend}
                labels={telemetryLabels}
                height={230}
                showAxis
                valuePrefix="$"
              />
              <div class="trend-axis"><span>06:00 UTC</span><span>13:00</span><span>20:30 LIVE</span></div>
            </div>
          {:else if analyticsView === "anomaly"}
            <div class="anomaly-grid">
              {#each anomalySignals as signal, index}
                <article class="anomaly" data-tone={signal.tone} style={`--intensity:${signal.intensity / 100};--index:${index}`}>
                  <div class="anomaly__top">
                    <span>{signal.asset}</span>
                    <strong>{signal.score}</strong>
                  </div>
                  <div class="anomaly__factor">{signal.factor}</div>
                  <div class="anomaly__meter"><i style={`width:${signal.intensity}%`}></i></div>
                  <div class="anomaly__meta">confidence {signal.intensity}%</div>
                </article>
              {/each}
            </div>
          {:else}
            <div class="live-log">
              <div class="live-log__head">
                <span>Time</span><span>Plane</span><span>Event</span><span>Evidence</span>
              </div>
              {#each telemetryLog as entry, index}
                <div class="live-log__row" style={`--index:${index}`}>
                  <time>{entry.time}</time>
                  <span class="live-log__source">{entry.source}</span>
                  <strong>{entry.event}</strong>
                  <code>{entry.meta}</code>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </GlassPanel>

      <section class="split">
        <GlassPanel eyebrow="Intelligence brief" title="Session doctrine" tone="cyan">
          <ul class="notes">
            <li><strong>Primary fabric</strong><span>CoinGecko entitlement through deterministic cassette transport.</span></li>
            <li><strong>Adaptive failover</strong><span>Market and OHLCV requests route to CCXT on empty or 429.</span></li>
            <li><strong>Identity invariant</strong><span>Asset identity halts on divergence—no silent substitution.</span></li>
          </ul>
          <div class="session-coordinates">
            <span>REGIME <strong>EXPANSION</strong></span>
            <span>CONF <strong>94.8%</strong></span>
            <span>LATENCY <strong>18ms</strong></span>
          </div>
        </GlassPanel>
        <GlassPanel eyebrow="Focused asset" title={activeRow ? `${activeRow.symbol} · ${activeRow.name}` : "No asset"} tone="emerald">
          {#if activeRow}
            <div class="focus-card">
              <div class="focus-card__price">{activeRow.price}</div>
              <div class="focus-card__row">
                <EvidenceChip status={activeRow.evidence} label={activeRow.provider} />
                <StaleDataMarker eventTime={activeRow.asOf} maxAge={60_000} />
              </div>
            </div>
            <div class="chart-block">
              <div class="pane__title">
                OHLC{#if chartMeta} · {chartMeta}{/if}
              </div>
              {#if chartCandles.length}
                <PriceChart candles={chartCandles} height={240} />
              {:else}
                <p class="notes">
                  {chartError ?? "No OHLC cassette for this asset yet."}
                </p>
              {/if}
              {#if coinDetail && coinDetail.coingeckoId === activeRow.coingeckoId}
                <div class="detail-blurb">
                  {#if coinDetail.marketCapRank != null}
                    <span class="fresh">rank {coinDetail.marketCapRank}</span>
                  {/if}
                  {#each coinDetail.categories.slice(0, 3) as cat}
                    <EvidenceChip status="confirmed" label={cat} />
                  {/each}
                  {#if coinDetail.description}
                    <p class="notes">{coinDetail.description}</p>
                  {/if}
                </div>
              {/if}
            </div>
          {:else}
            <p class="notes">No quotes loaded.</p>
          {/if}
        </GlassPanel>
      </section>
    {:else if activePanel === "watchlist"}
      <header class="page-head">
        <div>
          <h1>Watchlist</h1>
          <p>Select a row to pin focus. Staleness and evidence travel with every quote.</p>
        </div>
      </header>

      <div class="watch">
        {#if watchlist.length > 100}
          <VirtualList items={watchlist} rowHeight={58} height={580}>
            {#snippet children(row)}
              <div class="virtual-watch-row">
                <a href={`/workspace/asset/${row.coingeckoId}`}>
                  <strong>{row.symbol}</strong> · {row.name}
                </a>
                <span class="num">{row.price}</span>
                <span class="num" data-tone={row.change.startsWith("-") ? "down" : "up"}>
                  {row.change}
                </span>
                <EvidenceChip status={row.evidence} label={row.provider} />
              </div>
            {/snippet}
          </VirtualList>
        {:else}
          <table>
          <thead>
            <tr>
              <th>Asset</th>
              <th class="num">Last</th>
              <th class="num">24h</th>
              <th class="num">Vol</th>
              <th>Provenance</th>
              <th>Freshness</th>
            </tr>
          </thead>
          <tbody>
            {#each watchlist as row}
              <tr
                class:selected={selected === row.symbol}
                tabindex="0"
                onclick={() => void selectSymbol(row.symbol)}
                onkeydown={(e) => e.key === "Enter" && void selectSymbol(row.symbol)}
              >
                <td>
                  <div class="asset">
                    <strong>{row.symbol}</strong>
                    <span>{row.name}</span>
                    <a
                      class="asset-link"
                      href={`/workspace/asset/${row.coingeckoId}`}
                      onclick={(event) => event.stopPropagation()}
                    >Open asset</a>
                  </div>
                </td>
                <td class="num">{row.price}</td>
                <td class="num" data-tone={row.change.startsWith("-") ? "down" : "up"}>
                  {row.change}
                </td>
                <td class="num">{row.volume}</td>
                <td><EvidenceChip status={row.evidence} label={row.provider} /></td>
                <td>
                  <StaleDataMarker eventTime={row.asOf} maxAge={60_000} label="Stale" />
                  {#if Date.now() - new Date(row.asOf).getTime() <= 60_000}
                    <span class="fresh">fresh</span>
                  {/if}
                  <button
                    class="remove"
                    type="button"
                    onclick={(event) => {
                      event.stopPropagation();
                      void removeFromWatchlist(row.coingeckoId);
                    }}
                  >Remove</button>
                </td>
              </tr>
            {/each}
          </tbody>
          </table>
        {/if}

        {#if activeRow}
          <aside class="detail" aria-label="Selected asset">
            <div class="detail__kicker">Selected</div>
            <h2>{activeRow.symbol}</h2>
            <p>{activeRow.name}</p>
            <Metric label="Last" value={activeRow.price} delta={activeRow.change} />
            <div class="detail__block">
              <div class="detail__label">Evidence</div>
              <EvidenceChip status={activeRow.evidence} label={`${activeRow.provider} · searched`} />
            </div>
            <div class="detail__block">
              <div class="detail__label">As-of</div>
              <code>{new Date(activeRow.asOf).toLocaleTimeString()}</code>
            </div>
            <div class="detail__block">
              <div class="detail__label">Chart</div>
              {#if chartCandles.length}
                <PriceChart candles={chartCandles} height={180} />
              {:else}
                <p class="notes">{chartError ?? "No OHLC cassette for this asset."}</p>
              {/if}
            </div>
          </aside>
        {/if}
      </div>
    {:else if activePanel === "scanner"}
      <header class="page-head">
        <div>
          <h1>Scanner</h1>
          <p>Trending + category map from the CoinGecko adapter — every row carries provenance.</p>
        </div>
      </header>

      <div class="scanner-tools">
        <label>
          Filter trending
          <input bind:value={scannerQuery} placeholder="symbol, name, or id" />
        </label>
        <label>
          Save current filter
          <input bind:value={filterName} placeholder="Filter name" />
        </label>
        <button type="button" class="text-action" onclick={() => void saveScannerFilter()}>
          Save filter
        </button>
      </div>
      {#if savedFilters.length}
        <div class="saved-filters" aria-label="Saved scanner filters">
          {#each savedFilters as filter}
            <button type="button" onclick={() => (scannerQuery = filter.query)}>
              {filter.name} · {filter.query || "all"}
            </button>
          {/each}
        </div>
      {/if}

      <div class="pane__title">Trending</div>
      <ol class="hits">
        {#each filteredTrending as hit, i}
          <li class="hit">
            <span class="hit__rank">{String(i + 1).padStart(2, "0")}</span>
            <div>
              <div class="hit__name">{hit.symbol.toUpperCase()} · {hit.name}</div>
              <div class="hit__detail">
                {hit.coingeckoId}{#if hit.marketCapRank != null} · rank {hit.marketCapRank}{/if}
              </div>
            </div>
            <EvidenceChip status="confirmed" label="coingecko" />
            <button
              type="button"
              class="text-action"
              onclick={() => void addToWatchlist(hit.coingeckoId)}
            >Watch</button>
          </li>
        {/each}
      </ol>

      <div class="pane__title pane__title--gap">Categories</div>
      <ol class="hits">
        {#each categories as cat, i}
          <li class="hit">
            <span class="hit__rank">{String(i + 1).padStart(2, "0")}</span>
            <div>
              <div class="hit__name">{cat.name}</div>
              <div class="hit__detail">
                cap {formatCompact(cat.marketCap)} · {formatChange(cat.change24hPct)} ·
                {cat.top3Coins.slice(0, 3).join(", ")}
              </div>
            </div>
            <EvidenceChip status="confirmed" label="coingecko" />
          </li>
        {/each}
      </ol>
    {:else if activePanel === "providers"}
      <header class="page-head">
        <div>
          <h1>Providers</h1>
          <p>GCRA admission with exact next-permit times — never a spinner.</p>
        </div>
      </header>
      <div class="providers">
        <div class="provider">
          <div class="provider__name">CoinGecko</div>
          <div class="provider__role">Primary · Pro cassette · {sourceLabel}</div>
          <RateBudgetMeter
            label="quota"
            remainingRatio={remainingRatio}
            nextPermitLabel={nextPermitLabel}
          />
          <button
            type="button"
            class="text-action"
            onclick={() => void spendBudget()}
          >
            Spend budget
          </button>
          <button type="button" class="text-action" onclick={() => loadMarkets()}>
            Reload cassette
          </button>
        </div>
        <div class="provider">
          <div class="provider__name">CCXT</div>
          <div class="provider__role">Fallback · markets / OHLCV</div>
          <RateBudgetMeter label="quota" remainingRatio={1} nextPermitLabel="ready" />
        </div>
        <div class="provider">
          <div class="provider__name">Identity</div>
          <div class="provider__role">CoinGecko only · halt on divergence</div>
          <EvidenceChip status="confirmed" label="no failover" />
        </div>
      </div>

      <div class="pane__title pane__title--gap">Exchanges</div>
      <ol class="hits">
        {#each exchanges as x, i}
          <li class="hit">
            <span class="hit__rank">{String(i + 1).padStart(2, "0")}</span>
            <div>
              <div class="hit__name">{x.name}</div>
              <div class="hit__detail">
                {x.id}
                {#if x.country} · {x.country}{/if}
                {#if x.tradeVolume24hBtc} · vol {formatCompact(x.tradeVolume24hBtc)} BTC{/if}
              </div>
            </div>
            {#if x.trustScore != null}
              <EvidenceChip status="confirmed" label={`trust ${x.trustScore}`} />
            {:else}
              <EvidenceChip status="uncertain" label="unscored" />
            {/if}
          </li>
        {/each}
      </ol>
    {:else}
      <header class="page-head">
        <div>
          <h1>Alerts</h1>
          <p>Local Wave 1 rules are persisted now; evaluation and notifications arrive later.</p>
        </div>
      </header>
      <section class="alert-builder">
        <span>BTC 24h change above</span>
        <input type="number" min="0" step="0.5" bind:value={alertThreshold} />
        <span>%</span>
        <button type="button" class="text-action" onclick={() => void addAlertRule()}>
          Add rule
        </button>
      </section>
      <ol class="hits">
        {#each alertRules as rule}
          <li class="hit">
            <span class="hit__rank">{rule.enabled ? "ON" : "OFF"}</span>
            <div>
              <div class="hit__name">{rule.symbol} 24h change above {rule.change24hAbove}%</div>
              <div class="hit__detail">{rule.id}</div>
            </div>
            <EvidenceChip status="confirmed" label="local rule" />
          </li>
        {/each}
      </ol>
    {/if}
  </div>
</WorkspaceShell>

<CommandPalette
  bind:open={paletteOpen}
  {commands}
  extraCommands={searchHits}
  {onSelect}
  onQueryChange={onPaletteQuery}
/>

{#if notice}
  <div class="notice" role="status">{notice}</div>
{/if}

<style>
  .rail-label {
    margin: 0 0 var(--space-2) var(--space-3);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .rail-label--gap { margin-top: var(--space-6); }
  .nav { display: grid; gap: 2px; }
  .nav__item {
    padding: 8px 12px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text-secondary);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    text-align: left;
  }
  .nav__item.active,
  .nav__item:hover {
    background: var(--color-surface-2);
    color: var(--color-text-primary);
  }
  .focus-list { display: grid; gap: 2px; }
  .focus {
    display: flex;
    justify-content: space-between;
    padding: 8px 12px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text-primary);
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }
  .focus.active,
  .focus:hover { background: var(--color-surface-2); }
  .focus__chg[data-tone="up"] { color: var(--color-up); }
  .focus__chg[data-tone="down"] { color: var(--color-down); }

  .canvas { padding: var(--space-5) var(--space-6) var(--space-7); }
  .degraded {
    display: flex;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--color-warning);
    background: color-mix(in oklab, var(--color-surface-1) 88%, var(--color-warning));
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
  }
  .degraded strong { color: var(--color-warning); }
  .page-head {
    display: flex;
    justify-content: space-between;
    gap: var(--space-5);
    align-items: flex-end;
    margin-bottom: var(--space-5);
  }
  .page-head h1 {
    margin: 0;
    font-size: var(--font-size-2xl);
    font-weight: var(--font-weight-semibold);
    letter-spacing: -0.03em;
  }
  .page-head p {
    max-width: 40rem;
    margin: var(--space-2) 0 0;
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
  }
  .page-head__meta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .metric-strip {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-4);
    padding: var(--space-4) 0;
    margin-bottom: var(--space-5);
    border-top: 1px solid var(--color-border-default);
    border-bottom: 1px solid var(--color-border-default);
  }

  .split {
    display: grid;
    grid-template-columns: 1.4fr 1fr;
    gap: var(--space-5);
  }
  .pane__title {
    margin-bottom: var(--space-3);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .pane__title--gap { margin-top: var(--space-6); }
  .detail-blurb {
    display: grid;
    gap: var(--space-2);
    margin-top: var(--space-4);
  }
  .notes {
    margin: 0;
    padding-left: 1.1rem;
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
    line-height: var(--line-height-relaxed);
  }
  .focus-card__price {
    margin: var(--space-3) 0;
    font-family: var(--font-mono);
    font-size: var(--font-size-2xl);
    font-weight: var(--font-weight-semibold);
    letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums;
  }
  .focus-card__row { display: flex; gap: var(--space-3); align-items: center; }
  .chart-block { margin-top: var(--space-5); }

  .watch {
    display: grid;
    grid-template-columns: 1fr 280px;
    gap: var(--space-5);
    align-items: start;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-sm);
  }
  th {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border-default);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.05em;
    text-align: left;
    text-transform: uppercase;
  }
  td {
    padding: 12px var(--space-3);
    border-bottom: 1px solid var(--color-border-default);
    vertical-align: middle;
  }
  tr {
    cursor: pointer;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }
  tr:hover,
  tr.selected { background: color-mix(in oklab, var(--color-surface-1) 80%, var(--color-brand-primary) 8%); }
  .num {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  th.num { text-align: right; }
  [data-tone="up"] { color: var(--color-up); }
  [data-tone="down"] { color: var(--color-down); }
  .asset { display: grid; gap: 2px; }
  .asset strong { font-family: var(--font-mono); }
  .asset span { color: var(--color-text-tertiary); font-size: var(--font-size-xs); }
  .asset-link {
    width: fit-content;
    color: var(--color-brand-primary);
    font-size: var(--font-size-xs);
    text-decoration: none;
  }
  .remove {
    display: block;
    margin-top: var(--space-1);
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-text-tertiary);
    cursor: pointer;
    font-size: var(--font-size-xs);
  }
  .virtual-watch-row {
    display: grid;
    grid-template-columns: minmax(180px, 1fr) 120px 80px auto;
    gap: var(--space-4);
    align-items: center;
    height: 100%;
    padding: 0 var(--space-3);
    border-bottom: 1px solid var(--color-border-default);
    font-size: var(--font-size-sm);
  }
  .virtual-watch-row a { color: inherit; text-decoration: none; }
  .fresh {
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    text-transform: uppercase;
  }
  .detail {
    position: sticky;
    top: var(--space-4);
    padding: var(--space-4);
    border-left: 1px solid var(--color-border-default);
  }
  .detail__kicker {
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .detail h2 {
    margin: var(--space-2) 0 0;
    font-family: var(--font-mono);
    font-size: var(--font-size-xl);
  }
  .detail > p {
    margin: 0 0 var(--space-4);
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
  }
  .detail__block { margin-top: var(--space-4); }
  .detail__label {
    margin-bottom: var(--space-2);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  code {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }

  .hits {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .hit {
    display: grid;
    grid-template-columns: 48px 1fr auto auto;
    gap: var(--space-4);
    align-items: center;
    padding: var(--space-4) 0;
    border-bottom: 1px solid var(--color-border-default);
  }
  .hit__rank {
    font-family: var(--font-mono);
    color: var(--color-text-tertiary);
  }
  .hit__name { font-weight: var(--font-weight-semibold); }
  .hit__detail {
    margin-top: 2px;
    color: var(--color-text-secondary);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }
  .scanner-tools,
  .alert-builder {
    display: flex;
    gap: var(--space-3);
    align-items: end;
    margin-bottom: var(--space-4);
  }
  .scanner-tools label {
    display: grid;
    gap: var(--space-1);
    color: var(--color-text-tertiary);
    font-size: var(--font-size-xs);
  }
  .scanner-tools input,
  .alert-builder input {
    min-width: 180px;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-md);
    background: var(--color-surface-1);
    color: var(--color-text-primary);
  }
  .alert-builder input { min-width: 80px; width: 100px; }
  .saved-filters {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-bottom: var(--space-5);
  }
  .saved-filters button {
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-sm);
    background: var(--color-surface-1);
    color: var(--color-text-secondary);
    cursor: pointer;
    font-size: var(--font-size-xs);
  }

  .providers {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-5);
  }
  .provider {
    display: grid;
    gap: var(--space-3);
    padding: var(--space-4) 0;
    border-top: 1px solid var(--color-border-default);
  }
  .provider__name {
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-semibold);
  }
  .provider__role {
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
  }
  .text-action {
    width: fit-content;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-brand-primary);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
  }
  .sep { opacity: 0.7; }
  .state-block {
    padding: var(--space-6) 0;
    color: var(--color-text-secondary);
    font-size: var(--font-size-sm);
  }
  .state-block__rule {
    width: 64px;
    height: 2px;
    margin-bottom: var(--space-4);
    background: var(--color-brand-primary);
    animation: prismatik-fade-in var(--duration-base) var(--ease-decelerate);
  }
  .state-block--error { color: var(--color-danger); }
  .notice {
    position: fixed;
    right: var(--space-5);
    bottom: var(--space-5);
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface-1);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    animation: prismatik-slide-up-in var(--duration-base) var(--ease-decelerate);
  }

  @media (max-width: 1100px) {
    .metric-strip,
    .split,
    .watch,
    .providers {
      grid-template-columns: 1fr;
    }
    .detail { border-left: 0; border-top: 1px solid var(--color-border-default); }
  }

  /* Luminous command-node composition */
  .rail-label { letter-spacing: .13em; }
  .nav { gap: 3px; }
  .nav__item {
    padding: 8px 10px;
    border: 1px solid transparent;
    transition:
      background-color var(--duration-base) var(--ease-standard),
      border-color var(--duration-base) var(--ease-standard),
      color var(--duration-base) var(--ease-standard),
      transform var(--duration-base) var(--ease-standard);
  }
  .nav__item.active,
  .nav__item:hover {
    border-color: rgba(0, 240, 255, .09);
    background: linear-gradient(90deg, rgba(0, 240, 255, .08), transparent);
    transform: translateX(2px);
  }
  .focus.active,
  .focus:hover { background: rgba(168, 85, 247, .075); }
  .canvas {
    width: min(100%, 1560px);
    margin: 0 auto;
    padding: var(--space-6) clamp(20px, 3vw, 46px) var(--space-8);
  }
  .degraded {
    border-color: color-mix(in oklab, var(--color-warning) 34%, transparent);
    border-radius: var(--radius-md);
    background: color-mix(in oklab, rgba(13, 17, 28, .78) 92%, var(--color-warning));
    box-shadow: inset 3px 0 0 var(--color-warning);
  }
  .page-head { margin-bottom: var(--space-6); }
  .page-head__kicker {
    margin-bottom: 10px;
    color: var(--color-brand-primary);
    font-family: var(--font-mono);
    font-size: .625rem;
    font-weight: var(--font-weight-semibold);
    letter-spacing: .14em;
    text-transform: uppercase;
  }
  .page-head h1 {
    font-size: clamp(1.85rem, 3vw, 2.65rem);
    letter-spacing: -.045em;
    line-height: 1;
  }
  .page-head p {
    margin-top: var(--space-3);
    font-size: var(--font-size-base);
  }
  .page-head__meta { align-items: center; gap: var(--space-3); }
  .metric-strip {
    gap: var(--space-3);
    padding: 0;
    border: 0;
  }
  .split { margin-top: var(--space-5); }
  .notes { padding: 0; list-style: none; }
  .notes li {
    display: grid;
    grid-template-columns: minmax(110px, .42fr) 1fr;
    gap: var(--space-4);
    padding: 12px 0;
    border-bottom: 1px solid rgba(255,255,255,.05);
  }
  .notes li:first-child { padding-top: 0; }
  .notes li:last-child { border-bottom: 0; }
  .notes strong {
    color: var(--color-text-primary);
    font-size: .6875rem;
    font-weight: var(--font-weight-semibold);
    letter-spacing: .04em;
    text-transform: uppercase;
  }
  .session-coordinates {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    margin-top: var(--space-5);
    padding-top: var(--space-4);
    border-top: 1px solid var(--color-border-default);
  }
  .session-coordinates span {
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: .5625rem;
    letter-spacing: .08em;
  }
  .session-coordinates strong {
    color: var(--color-success);
    font-weight: var(--font-weight-medium);
  }
  .analytics-frame,
  .trend-view { min-height: 292px; }
  .trend-view__head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: var(--space-4);
    margin-bottom: var(--space-2);
  }
  .trend-view__head > div:first-child { display: grid; gap: 3px; }
  .trend-view__head span {
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: .625rem;
    letter-spacing: .08em;
    text-transform: uppercase;
  }
  .trend-view__head strong {
    font-family: var(--font-mono);
    font-size: var(--font-size-2xl);
    font-weight: var(--font-weight-semibold);
    letter-spacing: -.035em;
  }
  .trend-view__delta {
    color: var(--color-success);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }
  .trend-view__delta span { margin-left: 4px; }
  .trend-axis {
    display: flex;
    justify-content: space-between;
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: .5625rem;
    letter-spacing: .04em;
  }
  .trend-axis span:last-child { color: var(--color-brand-primary); }
  .anomaly-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-3);
    min-height: 292px;
  }
  .anomaly {
    --anomaly-color: var(--color-brand-primary);
    position: relative;
    display: flex;
    min-height: 138px;
    flex-direction: column;
    justify-content: space-between;
    padding: var(--space-4);
    overflow: hidden;
    border: 1px solid color-mix(in oklab, var(--anomaly-color) calc(var(--intensity) * 30%), rgba(255,255,255,.06));
    border-radius: var(--radius-md);
    background:
      radial-gradient(circle at 100% 0, color-mix(in oklab, var(--anomaly-color) calc(var(--intensity) * 15%), transparent), transparent 65%),
      rgba(255,255,255,.018);
    animation: card-enter var(--duration-slow) var(--ease-standard) both;
    animation-delay: calc(var(--index) * 65ms);
  }
  .anomaly[data-tone="violet"] { --anomaly-color: var(--color-brand-accent); }
  .anomaly[data-tone="emerald"] { --anomaly-color: var(--color-success); }
  .anomaly[data-tone="amber"] { --anomaly-color: var(--color-warning); }
  .anomaly__top { display: flex; justify-content: space-between; gap: var(--space-3); }
  .anomaly__top span,
  .anomaly__meta {
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: .5625rem;
    letter-spacing: .08em;
    text-transform: uppercase;
  }
  .anomaly__top strong {
    color: var(--anomaly-color);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }
  .anomaly__factor { font-size: var(--font-size-sm); font-weight: var(--font-weight-medium); }
  .anomaly__meter { height: 2px; overflow: hidden; background: rgba(255,255,255,.06); }
  .anomaly__meter i {
    display: block;
    height: 100%;
    background: var(--anomaly-color);
    box-shadow: 0 0 10px var(--anomaly-color);
  }
  .live-log {
    min-height: 292px;
    overflow: hidden;
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-md);
    background: rgba(3, 6, 11, .44);
  }
  .live-log__head,
  .live-log__row {
    display: grid;
    grid-template-columns: 120px 82px minmax(220px, 1fr) minmax(170px, .7fr);
    gap: var(--space-3);
    align-items: center;
    padding: 10px var(--space-4);
  }
  .live-log__head {
    border-bottom: 1px solid var(--color-border-default);
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
    font-size: .5625rem;
    letter-spacing: .1em;
    text-transform: uppercase;
  }
  .live-log__row {
    border-bottom: 1px solid rgba(255,255,255,.045);
    animation: card-enter var(--duration-slow) var(--ease-standard) both;
    animation-delay: calc(var(--index) * 50ms);
    font-size: var(--font-size-xs);
  }
  .live-log__row time,
  .live-log__row code {
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
  }
  .live-log__source {
    color: var(--color-brand-primary);
    font-family: var(--font-mono);
    font-size: .5625rem;
    letter-spacing: .06em;
  }
  .live-log__row strong { font-weight: var(--font-weight-medium); }
  .focus-card__price { margin-top: 0; font-size: 2rem; }
  .chart-block {
    padding-top: var(--space-4);
    border-top: 1px solid var(--color-border-default);
  }
  tr {
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      box-shadow var(--duration-fast) var(--ease-standard);
  }
  tr:hover,
  tr.selected {
    background: rgba(0,240,255,.04);
    box-shadow: inset 2px 0 0 rgba(0,240,255,.5);
  }
  .detail {
    padding: var(--space-5);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-lg);
    background: var(--color-surface-1);
    box-shadow: var(--shadow-panel);
    backdrop-filter: blur(var(--blur-panel));
  }
  .provider {
    padding: var(--space-5);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-lg);
    background: var(--color-surface-1);
    box-shadow: var(--shadow-panel);
  }
  .text-action {
    padding: 7px 11px;
    border: 1px solid rgba(0,240,255,.12);
    border-radius: 999px;
    background: rgba(0,240,255,.045);
  }
  .notice {
    border-radius: 999px;
    background: rgba(8,12,20,.86);
    box-shadow: var(--shadow-panel), var(--shadow-cyan);
    backdrop-filter: blur(20px);
  }
  @keyframes card-enter {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }
  @media (max-width: 1100px) {
    .anomaly-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .live-log__head,
    .live-log__row { grid-template-columns: 108px 70px 1fr; }
    .live-log__head span:last-child,
    .live-log__row code { display: none; }
  }
  @media (max-width: 720px) {
    .canvas { padding-inline: var(--space-4); }
    .page-head { align-items: flex-start; flex-direction: column; }
    .anomaly-grid { grid-template-columns: 1fr; }
  }
</style>
