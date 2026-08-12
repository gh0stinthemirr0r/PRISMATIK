/**
 * 2D Accuracy Atlas — heatmap with gradient cells, glow headers, sparkline confidence.
 */

/**
 * One cell of the skill atlas.
 *
 * `skill` is a Brier skill score against climatology: 1 is perfect, 0 means the
 * forecaster did no better than the base rate, negative means it did worse.
 * Zero is therefore the number that matters, not the colour.
 */
export interface AccuracyCell { assetClass: string; horizon: string; skill: number; sampleSize: number; calibrated: boolean; }
export interface AccuracyAtlasConfig { cells: AccuracyCell[]; width: number; height: number; }

export function renderAccuracyAtlas(container: HTMLElement, cfg: AccuracyAtlasConfig) {
  const { cells, width: W, height: H } = cfg;
  const m = { t: 45, r: 20, b: 20, l: 90 };
  const classes = [...new Set(cells.map(c => c.assetClass))];
  const horizons = [...new Set(cells.map(c => c.horizon))];
  const cW = (W - m.l - m.r) / horizons.length;
  const cH = (H - m.t - m.b) / classes.length;

  const color = (v: number, n: number): string => {
    if (n < 30) return '#1f2937';
    if (v > 0.3) return '#059669'; if (v > 0.1) return '#34d399'; if (v > 0) return '#6ee7b7';
    if (v > -0.1) return '#fef3c7'; if (v > -0.3) return '#fbbf24'; return '#ef4444';
  };
  const tc = (v: number, n: number): string => {
    if (n < 30) return '#6b7280';
    if (v > 0.15 || v < -0.15) return '#ffffff'; return '#1f2937';
  };

  const cells_ = cells.map(c => {
    const ri = classes.indexOf(c.assetClass), ci = horizons.indexOf(c.horizon);
    if (ri < 0 || ci < 0) return '';
    const x = m.l + ci * cW, y = m.t + ri * cH;
    const fill = color(c.skill, c.sampleSize);
    const text = c.sampleSize < 30 ? '—' : c.skill.toFixed(2);
    const glow = c.skill > 0.3 ? `filter="url(#at-glow)"` : '';
    return `
      <rect x="${x+1}" y="${y+1}" width="${cW-2}" height="${cH-2}" fill="${fill}" rx="4" opacity="0.85" ${glow}/>
      <text x="${x+cW/2}" y="${y+cH/2-3}" fill="${tc(c.skill, c.sampleSize)}" font-size="12"
            text-anchor="middle" dominant-baseline="middle" font-weight="700">${text}</text>
      <text x="${x+cW/2}" y="${y+cH/2+10}" fill="${tc(c.skill, c.sampleSize)}" font-size="7"
            text-anchor="middle" opacity="0.5">n=${c.sampleSize}</text>
      ${!c.calibrated ? `<circle cx="${x+cW-7}" cy="${y+7}" r="2.5" fill="#fbbf24" opacity="0.6"/>` : ''}`;
  }).join('');

  const rowLabels = classes.map((c, i) => `
    <text x="${m.l-10}" y="${m.t + i*cH + cH/2 + 4}" fill="#9ca3af" font-size="10" text-anchor="end">${c}</text>
  `).join('');

  const colLabels = horizons.map((h, i) => `
    <text x="${m.l + i*cW + cW/2}" y="${m.t-10}" fill="#9ca3af" font-size="9" text-anchor="middle">${h}</text>
  `).join('');

  container.innerHTML = `<svg width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg" style="font-family: 'IBM Plex Mono', monospace">
    <defs>
      <filter id="at-glow"><feGaussianBlur stdDeviation="3" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
    </defs>
    <rect width="${W}" height="${H}" fill="#050510" rx="8"/>
    <text x="20" y="24" fill="#00f0ff" font-size="11" font-weight="700">ACCURACY ATLAS</text>
    <text x="${W-20}" y="24" fill="#6b7280" font-size="8" text-anchor="end">● = uncalibrated</text>
    <text x="${W-20}" y="34" fill="#4b5563" font-size="7" text-anchor="end">Brier skill vs base rate · 0 = no better than climatology</text>
    ${colLabels}${rowLabels}${cells_}
  </svg>`;
}
