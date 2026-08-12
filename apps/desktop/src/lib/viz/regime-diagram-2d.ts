/**
 * 2D Regime Transition Diagram — animated particle flow on glowing edges.
 * SVG with CSS animations for particle dots along transition paths.
 */

export interface RegimeNode { id: string; label: string; active: boolean; duration: number; color: string; }
export interface RegimeTransition { from: string; to: string; probability: number; recent: boolean; }
export interface RegimeConfig { nodes: RegimeNode[]; transitions: RegimeTransition[]; width: number; height: number; }

export function renderRegimeDiagram(container: HTMLElement, cfg: RegimeConfig) {
  const { nodes, transitions, width: W, height: H } = cfg;
  const cx = W / 2, cy = H / 2, radius = Math.min(W, H) * 0.33;

  const pos: Record<string, {x:number;y:number}> = {};
  nodes.forEach((n, i) => {
    const a = (i / nodes.length) * Math.PI * 2 - Math.PI / 2;
    pos[n.id] = { x: cx + Math.cos(a) * radius, y: cy + Math.sin(a) * radius };
  });

  const edges = transitions.filter(t => t.probability > 0.05).map(t => {
    const f = pos[t.from], to = pos[t.to];
    if (!f || !to) return '';
    const dx = to.x - f.x, dy = to.y - f.y;
    const len = Math.sqrt(dx*dx + dy*dy);
    const nx = dx/len, ny = dy/len;
    const r = 32;
    const sx = f.x + nx*r, sy = f.y + ny*r;
    const ex = to.x - nx*r, ey = to.y - ny*r;
    const mx = (sx+ex)/2 - ny*18, my = (sy+ey)/2 + nx*18;
    const opacity = 0.15 + t.probability * 0.5;
    const sw = 1 + t.probability * 3.5;
    const color = t.recent ? '#00f0ff' : '#4b5563';
    const id = `edge-${t.from}-${t.to}`;

    return `
      <path id="${id}" d="M${sx.toFixed(1)} ${sy.toFixed(1)} Q${mx.toFixed(1)} ${my.toFixed(1)} ${ex.toFixed(1)} ${ey.toFixed(1)}"
            fill="none" stroke="${color}" stroke-width="${sw}" opacity="${opacity}" stroke-linecap="round"
            ${t.recent ? 'filter="url(#rg-glow)"' : ''}/>
      <text fill="${color}" font-size="9" opacity="${opacity}" text-anchor="middle">
        <textPath href="#${id}" startOffset="50%">${(t.probability*100).toFixed(0)}%</textPath>
      </text>
      ${t.recent ? `<circle r="3" fill="#00f0ff" opacity="0.8" filter="url(#rg-glow)">
        <animateMotion dur="${2 + Math.random()}s" repeatCount="indefinite" path="M${sx.toFixed(1)} ${sy.toFixed(1)} Q${mx.toFixed(1)} ${my.toFixed(1)} ${ex.toFixed(1)} ${ey.toFixed(1)}"/>
      </circle>` : ''}`;
  }).join('');

  const nodeElements = nodes.map(n => {
    const p = pos[n.id];
    const r = n.active ? 32 : 26;
    const abbrev = n.label.split('_').map(w => w[0].toUpperCase()).join('');
    return `
      <circle cx="${p.x}" cy="${p.y}" r="${r}" fill="${n.color}" fill-opacity="${n.active?0.15:0.06}"
              stroke="${n.color}" stroke-width="${n.active?2:0.8}" ${n.active?'filter="url(#rg-glow)"':''}/>
      <text x="${p.x}" y="${p.y-5}" fill="${n.color}" font-size="12" text-anchor="middle" font-weight="700">${abbrev}</text>
      ${n.active ? `
        <text x="${p.x}" y="${p.y+10}" fill="${n.color}" font-size="8" text-anchor="middle" opacity="0.7">${n.duration}d</text>
        <circle cx="${p.x}" cy="${p.y}" r="${r+2}" fill="none" stroke="${n.color}" stroke-width="0.5" opacity="0.3">
          <animate attributeName="r" from="${r}" to="${r+20}" dur="2.5s" repeatCount="indefinite"/>
          <animate attributeName="opacity" from="0.3" to="0" dur="2.5s" repeatCount="indefinite"/>
        </circle>
      ` : ''}`;
  }).join('');

  container.innerHTML = `<svg width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg" style="font-family: 'IBM Plex Mono', monospace">
    <defs>
      <filter id="rg-glow"><feGaussianBlur stdDeviation="4" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
      <filter id="rg-glow-soft"><feGaussianBlur stdDeviation="2" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
    </defs>
    <rect width="${W}" height="${H}" fill="#050510" rx="8"/>
    <text x="20" y="24" fill="#00f0ff" font-size="11" font-weight="700" filter="url(#rg-glow)">REGIME TRANSITIONS</text>
    ${edges}
    ${nodeElements}
  </svg>`;
}
