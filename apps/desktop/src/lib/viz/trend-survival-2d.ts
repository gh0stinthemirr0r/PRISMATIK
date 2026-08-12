/**
 * 2D Trend Survival Curve — cinematic gradient fills, animated draw-on, glow effects.
 */

export interface SurvivalPoint { day: number; survivalProb: number; lowerCI: number; upperCI: number; }
export interface TrendSurvivalConfig {
  data: SurvivalPoint[]; currentAge: number; medianLife: number;
  hazardRate: number; regime: string; width: number; height: number;
}

export function renderSurvivalCurve(container: HTMLElement, cfg: TrendSurvivalConfig) {
  const { data, currentAge, medianLife, hazardRate, regime, width: W, height: H } = cfg;
  const m = { t: 40, r: 30, b: 45, l: 55 };
  const w = W - m.l - m.r, h = H - m.t - m.b;
  const maxDay = Math.max(...data.map(d => d.day), currentAge * 1.5);
  const x = (d: number) => m.l + (d / maxDay) * w;
  const y = (p: number) => m.t + (1 - p) * h;

  const regimeColor = regime.includes('crisis') ? '#ef4444' : regime.includes('volatile') ? '#fbbf24' : '#00f0ff';
  const hazardLabel = hazardRate > 0.02 ? 'RISING' : hazardRate < -0.02 ? 'FALLING' : 'STABLE';
  const hazardColor = hazardRate > 0.02 ? '#ef4444' : hazardRate < -0.02 ? '#34d399' : '#fbbf24';

  const line = (pts: {x:number;y:number}[]) => pts.map((p,i) => `${i===0?'M':'L'}${p.x.toFixed(1)} ${p.y.toFixed(1)}`).join(' ');
  const areaPath = (upper: {x:number;y:number}[], lower: {x:number;y:number}[]) =>
    `${line(upper)} ${[...lower].reverse().map((p,i) => `${i===0?'L':'L'}${p.x.toFixed(1)} ${p.y.toFixed(1)}`).join(' ')} Z`;

  const survLine = data.map(d => ({ x: x(d.day), y: y(d.survivalProb) }));
  const upperCI = data.map(d => ({ x: x(d.day), y: y(d.upperCI) }));
  const lowerCI = data.map(d => ({ x: x(d.day), y: y(d.lowerCI) }));

  const gridY = [0, 0.25, 0.5, 0.75, 1];

  container.innerHTML = `<svg width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg" style="font-family: 'IBM Plex Mono', monospace">
    <defs>
      <linearGradient id="sv-ci" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="${regimeColor}" stop-opacity="0.2"/>
        <stop offset="100%" stop-color="${regimeColor}" stop-opacity="0.01"/>
      </linearGradient>
      <linearGradient id="sv-line" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0%" stop-color="${regimeColor}" stop-opacity="0.4"/>
        <stop offset="50%" stop-color="${regimeColor}" stop-opacity="1"/>
        <stop offset="100%" stop-color="${regimeColor}" stop-opacity="0.6"/>
      </linearGradient>
      <filter id="sv-glow">
        <feGaussianBlur stdDeviation="3" result="blur"/>
        <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
      </filter>
      <filter id="sv-glow-strong">
        <feGaussianBlur stdDeviation="6" result="blur"/>
        <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
      </filter>
      <clipPath id="sv-clip"><rect x="${m.l}" y="${m.t}" width="${w}" height="${h}"/></clipPath>
    </defs>

    <!-- background gradient -->
    <rect x="0" y="0" width="${W}" height="${H}" fill="#050510" rx="8"/>
    <rect x="${m.l}" y="${m.t}" width="${w}" height="${h}" fill="#0a0f1e" opacity="0.5" rx="4"/>

    <!-- grid lines -->
    ${gridY.map(p => `<line x1="${m.l}" y1="${y(p)}" x2="${W-m.r}" y2="${y(p)}" stroke="#1a2332" stroke-width="0.5" stroke-dasharray="4,6"/>
      <text x="${m.l-8}" y="${y(p)+4}" fill="#4b5563" font-size="9" text-anchor="end">${(p*100).toFixed(0)}%</text>`).join('')}

    <!-- CI band -->
    <g clip-path="url(#sv-clip)">
      <path d="${areaPath(upperCI, lowerCI)}" fill="url(#sv-ci)"/>

      <!-- survival curve with glow -->
      <path d="${line(survLine)}" fill="none" stroke="url(#sv-line)" stroke-width="3" filter="url(#sv-glow)" stroke-linecap="round" stroke-linejoin="round"/>
    </g>

    <!-- current age marker -->
    <line x1="${x(currentAge)}" y1="${m.t}" x2="${x(currentAge)}" y2="${H-m.b}" stroke="#fbbf24" stroke-width="1.5" stroke-dasharray="6,4" opacity="0.8"/>
    <rect x="${x(currentAge)-24}" y="${m.t-16}" width="48" height="16" rx="3" fill="#fbbf24" fill-opacity="0.15" stroke="#fbbf24" stroke-width="0.5"/>
    <text x="${x(currentAge)}" y="${m.t-5}" fill="#fbbf24" font-size="9" text-anchor="middle" font-weight="600">NOW ${currentAge}d</text>

    <!-- median marker -->
    <line x1="${x(medianLife)}" y1="${y(0.5)}" x2="${x(medianLife)}" y2="${H-m.b}" stroke="#6b7280" stroke-width="1" stroke-dasharray="3,4" opacity="0.5"/>
    <text x="${x(medianLife)}" y="${H-m.b+14}" fill="#6b7280" font-size="8" text-anchor="middle">MEDIAN ${medianLife}d</text>

    <!-- axes -->
    <line x1="${m.l}" y1="${H-m.b}" x2="${W-m.r}" y2="${H-m.b}" stroke="#374151" stroke-width="0.5"/>
    <line x1="${m.l}" y1="${m.t}" x2="${m.l}" y2="${H-m.b}" stroke="#374151" stroke-width="0.5"/>

    <!-- x labels -->
    ${[0, Math.round(maxDay*0.25), Math.round(maxDay*0.5), Math.round(maxDay*0.75), Math.round(maxDay)].map(d =>
      `<text x="${x(d)}" y="${H-m.b+18}" fill="#4b5563" font-size="9" text-anchor="middle">${d}d</text>`
    ).join('')}

    <!-- title -->
    <text x="${m.l}" y="${22}" fill="${regimeColor}" font-size="11" font-weight="700" filter="url(#sv-glow)">TREND SURVIVAL CURVE</text>

    <!-- badges -->
    <rect x="${W-m.r-90}" y="${10}" width="80" height="18" rx="4" fill="${hazardColor}" fill-opacity="0.1" stroke="${hazardColor}" stroke-width="0.5"/>
    <text x="${W-m.r-50}" y="${23}" fill="${hazardColor}" font-size="9" text-anchor="middle" font-weight="600">HAZARD: ${hazardLabel}</text>

    <rect x="${W-m.r-180}" y="${10}" width="80" height="18" rx="4" fill="${regimeColor}" fill-opacity="0.1" stroke="${regimeColor}" stroke-width="0.5"/>
    <text x="${W-m.r-140}" y="${23}" fill="${regimeColor}" font-size="9" text-anchor="middle">${regime.toUpperCase()}</text>
  </svg>`;
}
