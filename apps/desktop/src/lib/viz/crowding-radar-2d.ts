/**
 * 2D Crowding Radar — polar chart with heat gradient, danger glow, animated scan sweep.
 */

export interface CrowdingPoint { symbol: string; saturation: number; positioning: number; dangerous: boolean; sector: string; }
export interface CrowdingConfig { points: CrowdingPoint[]; width: number; height: number; }

export function renderCrowdingRadar(container: HTMLElement, cfg: CrowdingConfig) {
  const { points, width: W, height: H } = cfg;
  const cx = W/2, cy = H/2, maxR = Math.min(W,H) * 0.36;

  const toXY = (sat: number, pos: number) => {
    const a = sat * Math.PI * 2 - Math.PI / 2;
    return { x: cx + Math.cos(a) * pos * maxR, y: cy + Math.sin(a) * pos * maxR };
  };

  const circles = [0.25, 0.5, 0.75, 1].map(r => `
    <circle cx="${cx}" cy="${cy}" r="${r*maxR}" fill="none" stroke="#1a2332" stroke-width="0.5" ${r<1?'stroke-dasharray="3,5"':''}/>
    <text x="${cx+4}" y="${cy-r*maxR+4}" fill="#374151" font-size="7" opacity="0.6">${(r*100).toFixed(0)}%</text>
  `).join('');

  const labels = ['EMERGING','BUILDING','PEAK','FADING'];
  const radials = [0, 0.25, 0.5, 0.75].map((s,i) => {
    const a = s * Math.PI * 2 - Math.PI / 2;
    const ex = cx + Math.cos(a)*maxR, ey = cy + Math.sin(a)*maxR;
    const lx = cx + Math.cos(a)*(maxR+18), ly = cy + Math.sin(a)*(maxR+18);
    return `<line x1="${cx}" y1="${cy}" x2="${ex}" y2="${ey}" stroke="#1a2332" stroke-width="0.4"/>
      <text x="${lx}" y="${ly}" fill="#4b5563" font-size="7" text-anchor="middle" font-family="monospace">${labels[i]}</text>`;
  }).join('');

  // danger zone — radial gradient
  const dangerZone = `
    <defs>
      <radialGradient id="cr-danger" cx="70%" cy="30%" r="40%">
        <stop offset="0%" stop-color="#ef4444" stop-opacity="0.15"/>
        <stop offset="100%" stop-color="#ef4444" stop-opacity="0"/>
      </radialGradient>
    </defs>
    <path d="M${cx} ${cy} L${cx+maxR*0.7} ${cy-maxR*0.15} A${maxR} ${maxR} 0 0 1 ${cx+maxR*0.15} ${cy-maxR*0.7} Z"
          fill="url(#cr-danger)" stroke="#ef4444" stroke-width="0.5" stroke-dasharray="4,4" opacity="0.6"/>
    <text x="${cx+maxR*0.52}" y="${cy-maxR*0.42}" fill="#ef4444" font-size="7" font-weight="600" opacity="0.5">DANGER ZONE</text>`;

  // animated scan sweep
  const scanSweep = `<line x1="${cx}" y1="${cy}" x2="${cx}" y2="${cy-maxR}" stroke="#00f0ff" stroke-width="1" opacity="0.15">
    <animateTransform attributeName="transform" type="rotate" from="0 ${cx} ${cy}" to="360 ${cx} ${cy}" dur="8s" repeatCount="indefinite"/>
  </line>`;

  const dots = points.map(p => {
    const pt = toXY(p.saturation, p.positioning);
    const color = p.dangerous ? '#ef4444' : p.positioning > 0.7 ? '#fbbf24' : '#00f0ff';
    const size = p.dangerous ? 5 : 3.5;
    const glowR = p.dangerous ? 12 : 6;
    return `
      <circle cx="${pt.x}" cy="${pt.y}" r="${glowR}" fill="${color}" opacity="${p.dangerous?0.15:0.08}" filter="url(#cr-blur)"/>
      <circle cx="${pt.x}" cy="${pt.y}" r="${size}" fill="${color}" opacity="${p.dangerous?0.9:0.7}"/>
      <text x="${pt.x}" y="${pt.y-8}" fill="${color}" font-size="7" text-anchor="middle" font-weight="600">${p.symbol}</text>`;
  }).join('');

  container.innerHTML = `<svg width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg" style="font-family: 'IBM Plex Mono', monospace">
    <defs><filter id="cr-blur"><feGaussianBlur stdDeviation="4"/></filter></defs>
    <rect width="${W}" height="${H}" fill="#050510" rx="8"/>
    <text x="20" y="24" fill="#00f0ff" font-size="11" font-weight="700">CROWDING RADAR</text>
    ${circles}${radials}${dangerZone}${scanSweep}${dots}
  </svg>`;
}
