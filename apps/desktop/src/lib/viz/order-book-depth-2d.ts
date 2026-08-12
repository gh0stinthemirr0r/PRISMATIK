/**
 * 2D Order Book Depth Heatmap — real-time depth visualization.
 * X = price level, Y = cumulative volume. Bid/ask sides with gradient fill.
 * Animated updates. Trade prints as flash markers.
 */
export interface BookLevel { price: number; size: number; side: 'bid' | 'ask'; }
export interface TradePrint { price: number; size: number; side: 'buy' | 'sell'; time: number; }

export interface OrderBookConfig {
  bids: BookLevel[];
  asks: BookLevel[];
  trades: TradePrint[];
  spotPrice: number;
  symbol: string;
  width: number;
  height: number;
}

export function renderOrderBookDepth(container: HTMLElement, cfg: OrderBookConfig) {
  const { bids, asks, trades, spotPrice, symbol, width: W, height: H } = cfg;
  const m = { t: 40, r: 20, b: 45, l: 60 };
  const w = W - m.l - m.r, h = H - m.t - m.b;

  if (bids.length === 0 && asks.length === 0) {
    container.innerHTML = `<svg width="${W}" height="${H}"><rect width="${W}" height="${H}" fill="#050510" rx="8"/>
      <text x="${W/2}" y="${H/2}" fill="#4b5563" text-anchor="middle" font-family="monospace">No order book data</text></svg>`;
    return;
  }

  // compute cumulative depth
  const cumBids: { price: number; cum: number }[] = [];
  let cum = 0;
  [...bids].sort((a, b) => b.price - a.price).forEach(b => { cum += b.size; cumBids.push({ price: b.price, cum }); });

  const cumAsks: { price: number; cum: number }[] = [];
  cum = 0;
  [...asks].sort((a, b) => a.price - b.price).forEach(a => { cum += a.size; cumAsks.push({ price: a.price, cum }); });

  const allPrices = [...cumBids.map(b => b.price), ...cumAsks.map(a => a.price)];
  const minP = Math.min(...allPrices), maxP = Math.max(...allPrices);
  const maxCum = Math.max(...cumBids.map(b => b.cum), ...cumAsks.map(a => a.cum));

  const x = (p: number) => m.l + ((p - minP) / (maxP - minP)) * w;
  const y = (c: number) => m.t + h - (c / maxCum) * h;

  const bidLine = cumBids.map(b => `${x(b.price).toFixed(1)},${y(b.cum).toFixed(1)}`).join(' ');
  const askLine = cumAsks.map(a => `${x(a.price).toFixed(1)},${y(a.cum).toFixed(1)}`).join(' ');

  const bidArea = `M${x(cumBids[0]?.price ?? 0).toFixed(1)} ${y(0).toFixed(1)} ` +
    cumBids.map(b => `L${x(b.price).toFixed(1)} ${y(b.cum).toFixed(1)}`).join(' ') +
    ` L${x(cumBids[cumBids.length-1]?.price ?? 0).toFixed(1)} ${y(0).toFixed(1)} Z`;

  const askArea = `M${x(cumAsks[0]?.price ?? 0).toFixed(1)} ${y(0).toFixed(1)} ` +
    cumAsks.map(a => `L${x(a.price).toFixed(1)} ${y(a.cum).toFixed(1)}`).join(' ') +
    ` L${x(cumAsks[cumAsks.length-1]?.price ?? 0).toFixed(1)} ${y(0).toFixed(1)} Z`;

  // recent trades
  const recentTrades = trades.slice(-20);
  const tradeMarkers = recentTrades.map(t => {
    const color = t.side === 'buy' ? '#34d399' : '#f87171';
    const opacity = Math.max(0.2, 1 - (Date.now() - t.time) / 60000);
    return `<circle cx="${x(t.price)}" cy="${m.t + h * 0.3}" r="${1 + t.size * 0.001}" fill="${color}" opacity="${opacity}">
      <animate attributeName="cy" from="${m.t + h * 0.3}" to="${m.t + h * 0.1}" dur="2s" fill="freeze"/>
      <animate attributeName="opacity" from="${opacity}" to="0" dur="2s" fill="freeze"/>
    </circle>`;
  }).join('');

  container.innerHTML = `<svg width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg" style="font-family: 'IBM Plex Mono', monospace">
    <defs>
      <linearGradient id="ob-bid" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="#34d399" stop-opacity="0.4"/>
        <stop offset="100%" stop-color="#34d399" stop-opacity="0.02"/>
      </linearGradient>
      <linearGradient id="ob-ask" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="#f87171" stop-opacity="0.4"/>
        <stop offset="100%" stop-color="#f87171" stop-opacity="0.02"/>
      </linearGradient>
      <filter id="ob-glow"><feGaussianBlur stdDeviation="2" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
    </defs>

    <rect width="${W}" height="${H}" fill="#050510" rx="8"/>
    <text x="${m.l}" y="24" fill="#00f0ff" font-size="11" font-weight="700">${symbol} ORDER BOOK</text>
    <text x="${W-m.r}" y="24" fill="#6b7280" font-size="8" text-anchor="end">DEPTH</text>

    <!-- grid -->
    ${[0, 0.25, 0.5, 0.75, 1].map(p => `<line x1="${m.l}" y1="${y(p*maxCum)}" x2="${W-m.r}" y2="${y(p*maxCum)}" stroke="#1a2332" stroke-width="0.5"/>
      <text x="${m.l-6}" y="${y(p*maxCum)+4}" fill="#4b5563" font-size="8" text-anchor="end">${(p*maxCum).toFixed(0)}</text>`).join('')}

    <!-- depth areas -->
    <path d="${bidArea}" fill="url(#ob-bid)" filter="url(#ob-glow)"/>
    <path d="${askArea}" fill="url(#ob-ask)" filter="url(#ob-glow)"/>

    <!-- depth lines -->
    <polyline points="${bidLine}" fill="none" stroke="#34d399" stroke-width="2" filter="url(#ob-glow)"/>
    <polyline points="${askLine}" fill="none" stroke="#f87171" stroke-width="2" filter="url(#ob-glow)"/>

    <!-- spot price -->
    <line x1="${x(spotPrice)}" y1="${m.t}" x2="${x(spotPrice)}" y2="${H-m.b}" stroke="#ffffff" stroke-width="1" stroke-dasharray="4,4" opacity="0.4"/>
    <text x="${x(spotPrice)}" y="${H-m.b+14}" fill="#ffffff" font-size="9" text-anchor="middle">${spotPrice.toFixed(2)}</text>

    <!-- trade prints -->
    ${tradeMarkers}

    <!-- bid/ask labels -->
    <rect x="${m.l}" y="${H-m.b+20}" width="30" height="14" rx="3" fill="#34d399" fill-opacity="0.15"/>
    <text x="${m.l+15}" y="${H-m.b+31}" fill="#34d399" font-size="8" text-anchor="middle">BIDS</text>
    <rect x="${W-m.r-30}" y="${H-m.b+20}" width="30" height="14" rx="3" fill="#f87171" fill-opacity="0.15"/>
    <text x="${W-m.r-15}" y="${H-m.b+31}" fill="#f87171" font-size="8" text-anchor="middle">ASKS</text>
  </svg>`;
}
