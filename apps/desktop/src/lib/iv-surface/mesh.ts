/** Implied-vol surface mesh helpers (Canvas / WebGPU shared). */

export type IvQuote = {
  expiration: string;
  strike: number;
  optionType: string;
  impliedVolatility: number;
};

export type IvSurfaceMesh = {
  terms: string[];
  strikes: number[];
  /** `values[termIndex][strikeIndex]` — null when the cell is missing. */
  values: (number | null)[][];
  minIv: number;
  maxIv: number;
};

/**
 * Build a dense term × strike IV grid.
 * Prefers call quotes when both rights exist for the same cell.
 */
export function buildIvSurfaceMesh(quotes: IvQuote[]): IvSurfaceMesh {
  const terms = [...new Set(quotes.map((q) => q.expiration))].sort();
  const strikes = [...new Set(quotes.map((q) => q.strike))].sort((a, b) => a - b);
  const values: (number | null)[][] = terms.map(() => strikes.map(() => null));

  const preference = (right: string) => (right.toLowerCase() === "call" ? 2 : 1);
  const ranked = new Map<string, { iv: number; rank: number }>();

  for (const quote of quotes) {
    const key = `${quote.expiration}|${quote.strike}`;
    const rank = preference(quote.optionType);
    const prior = ranked.get(key);
    if (!prior || rank >= prior.rank) {
      ranked.set(key, { iv: quote.impliedVolatility, rank });
    }
  }

  let minIv = Number.POSITIVE_INFINITY;
  let maxIv = Number.NEGATIVE_INFINITY;
  for (const [key, { iv }] of ranked) {
    const [term, strikeRaw] = key.split("|");
    const yi = terms.indexOf(term);
    const xi = strikes.indexOf(Number(strikeRaw));
    if (yi < 0 || xi < 0) continue;
    values[yi][xi] = iv;
    minIv = Math.min(minIv, iv);
    maxIv = Math.max(maxIv, iv);
  }

  if (!Number.isFinite(minIv)) {
    minIv = 0;
    maxIv = 0;
  }

  return { terms, strikes, values, minIv, maxIv };
}

export function sampleMeshCell(mesh: IvSurfaceMesh, termIndex: number, strikeIndex: number): number | null {
  return mesh.values[termIndex]?.[strikeIndex] ?? null;
}

/** Cool brand-blue → warm accent (no purple/glow palette). */
export function ivColorRgba(t: number, minIv: number, maxIv: number): [number, number, number, number] {
  const span = Math.max(1e-9, maxIv - minIv);
  const u = Math.min(1, Math.max(0, (t - minIv) / span));
  const r = Math.round(47 + u * (224 - 47));
  const g = Math.round(111 + u * (122 - 111));
  const b = Math.round(237 + u * (61 - 237));
  return [r, g, b, 255];
}

export function ivColorCss(t: number, minIv: number, maxIv: number): string {
  const [r, g, b, a] = ivColorRgba(t, minIv, maxIv);
  return `rgba(${r}, ${g}, ${b}, ${(a / 255).toFixed(3)})`;
}
