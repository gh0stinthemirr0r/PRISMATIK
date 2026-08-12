/**
 * 2D Portfolio Exposure Sunburst — nested rings showing allocation.
 * Center = portfolio, Ring 1 = asset class, Ring 2 = sector, Ring 3 = individual.
 * Color = P&L, width = weight. Interactive drill-down.
 */
export interface ExposureNode {
  name: string;
  weight: number;
  pnl: number;
  children?: ExposureNode[];
}

export interface PortfolioExposureConfig {
  root: ExposureNode;
  width: number;
  height: number;
}

export function renderPortfolioSunburst(container: HTMLElement, cfg: PortfolioExposureConfig) {
  const { root, width: W, height: H } = cfg;
  const cx = W / 2, cy = H / 2;
  const maxR = Math.min(W, H) * 0.38;

  const pnlColor = (pnl: number): string => {
    if (pnl > 0.05) return '#059669';
    if (pnl > 0.01) return '#34d399';
    if (pnl > -0.01) return '#6b7280';
    if (pnl > -0.05) return '#fbbf24';
    return '#ef4444';
  };

  let paths = '';
  let labels = '';
  let startAngle = -Math.PI / 2;

  function drawRing(nodes: ExposureNode[], innerR: number, outerR: number, parentStart: number) {
    let angle = parentStart;
    nodes.forEach(node => {
      const sweep = node.weight * Math.PI * 2;
      if (sweep < 0.02) { angle += sweep; return; }

      const x1 = cx + Math.cos(angle) * innerR;
      const y1 = cy + Math.sin(angle) * innerR;
      const x2 = cx + Math.cos(angle) * outerR;
      const y2 = cy + Math.sin(angle) * outerR;
      const x3 = cx + Math.cos(angle + sweep) * outerR;
      const y3 = cy + Math.sin(angle + sweep) * outerR;
      const x4 = cx + Math.cos(angle + sweep) * innerR;
      const y4 = cy + Math.sin(angle + sweep) * innerR;
      const large = sweep > Math.PI ? 1 : 0;

      paths += `<path d="M${x1} ${y1} L${x2} ${y2} A${outerR} ${outerR} 0 ${large} 1 ${x3} ${y3} L${x4} ${y4} A${innerR} ${innerR} 0 ${large} 0 ${x1} ${y1} Z"
        fill="${pnlColor(node.pnl)}" fill-opacity="0.6" stroke="#050510" stroke-width="1"/>`;

      // label
      if (sweep > 0.15) {
        const midAngle = angle + sweep / 2;
        const midR = (innerR + outerR) / 2;
        const lx = cx + Math.cos(midAngle) * midR;
        const ly = cy + Math.sin(midAngle) * midR;
        const fontSize = Math.min(10, sweep * 12);
        labels += `<text x="${lx}" y="${ly}" fill="#ffffff" font-size="${fontSize}" text-anchor="middle"
          dominant-baseline="middle" font-family="monospace" opacity="0.8">${node.name}</text>`;
      }

      if (node.children) {
        drawRing(node.children, outerR, outerR + (maxR - outerR) * 0.6, angle);
      }
      angle += sweep;
    });
  }

  const ring1R = maxR * 0.3;
  drawRing(root.children ?? [root], ring1R * 0.4, ring1R, startAngle);
  if (root.children) {
    drawRing(root.children.flatMap(c => c.children ?? []), ring1R, maxR * 0.65, startAngle);
  }

  container.innerHTML = `<svg width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg" style="font-family: 'IBM Plex Mono', monospace">
    <defs>
      <filter id="sb-glow"><feGaussianBlur stdDeviation="2" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
    </defs>
    <rect width="${W}" height="${H}" fill="#050510" rx="8"/>
    <text x="20" y="24" fill="#00f0ff" font-size="11" font-weight="700">PORTFOLIO EXPOSURE</text>
    ${paths}
    ${labels}
    <!-- center -->
    <circle cx="${cx}" cy="${cy}" r="${ring1R * 0.35}" fill="#0a0f1e" stroke="#1a2332" stroke-width="1"/>
    <text x="${cx}" y="${cy - 6}" fill="#ffffff" font-size="11" text-anchor="middle" font-weight="700">${root.name}</text>
    <text x="${cx}" y="${cy + 8}" fill="${pnlColor(root.pnl)}" font-size="9" text-anchor="middle">${(root.pnl * 100).toFixed(1)}%</text>
  </svg>`;
}
