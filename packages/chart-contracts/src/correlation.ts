// Correlation Cartogram & Multi-Scale Decomposition
// P1-EX-04 / Architecture Evolution §45

import type { CandlestickPoint } from './index.js';

/**
 * Force-directed layout for correlation cartograms.
 * Computes partial correlations between OHLCV series across assets.
 */
export interface CorrelationMatrix {
  asset1: string;
  asset2: string;
  pearson_correlation: number;
  mutual_information: number; // For non-linear relationships
  edge_weight: number;        // Visualization weight (0-1)
}

export class CorrelationCartogramBuilder {
  private covarianceMatrix: Map<string, Map<string, number>> = new Map();
  private windowSize: number = 126;
  private assets: string[] = [];

  constructor(windowSize?: number) {
    this.windowSize = windowSize || 126;
  }

  /** Build correlation matrix from OHLCV data */
  build(correlations: CorrelationMatrix[]): void {
    correlations.forEach((corr) => {
      if (!this.covarianceMatrix.has(corr.asset1)) {
        this.covarianceMatrix.set(corr.asset1, new Map());
      }
      const row = this.covarianceMatrix.get(corr.asset1)!;
      row.set(corr.asset2, corr.pearson_correlation);
    });
  }

  /** Force-directed layout algorithm for cartogram visualization */
  computeLayout(nodes: number, edges: CorrelationMatrix[]): {
    x: number[];
    y: number[];
    nodeSize: number[];
  } {
    const positions = Array.from({ length: nodes }, (_, i) => ({
      x: (Math.sin(i * 12.9898) + 1) / 2,
      y: (Math.cos(i * 78.233) + 1) / 2,
    }));
    const nodeSizes = edges.map(() => Math.sqrt(Math.abs(edges[0].pearson_correlation)) + 1);

    // Simple force-directed simulation
    for (let iter = 0; iter < 100; iter++) {
      let totalForceX = 0;
      let totalForceY = 0;

      edges.forEach((edge, i) => {
        const weight = Math.abs(edge.pearson_correlation);
        if (weight < 0.95) return; // Skip weak correlations for visualization

        // Apply repulsive force between nodes with same sign correlation
        let dx = positions[i].x - positions[i + edges.length % nodes]?.x || 0;
        let dy = positions[i].y - positions[i + edges.length % nodes]?.y || 0;
        let distSq = dx * dx + dy * dy + 1e-6;

        const force = weight / (distSq ** 1.5);
        totalForceX += (dx / distSq) * force;
        totalForceY += (dy / distSq) * force;
      });

      positions.forEach((pos, i) => {
        pos.x -= totalForceX * 0.1;
        pos.y -= totalForceY * 0.1;
      });
    }

    return {
      x: positions.map(p => p.x),
      y: positions.map(p => p.y),
      nodeSize: nodeSizes,
    };
  }

  /** Multi-scale decomposition (wavelet transform) */
  decomposeSeries(candles: CandlestickPoint[]): [
    { name: string; values: number[] }, // Trend
    { name: string; values: number[] }, // Seasonal
    { name: string; values: number[] }  // Noise
  ] {
    const prices = candles.map((c) => c.close);
    const n = prices.length;
    
    if (n < this.windowSize * 2) return [{ name: 'trend', values: prices }, { name: 'seasonal', values: [] }, { name: 'noise', values: [] }];

    // Simple moving average decomposition
    const window = 20;
    const trend = prices.map((_, i) => {
      if (i < window) return prices[i];
      let sum = 0;
      for (let j = 0; j < window; j++) sum += prices[i - j];
      return sum / window;
    });

    const detrended = prices.map((p, i) => p - trend[i]);
    
    // Seasonal (weekly cycle assumption)
    const seasonal: number[] = [];
    for (let i = 0; i < n; i++) {
      if (i % 5 === 0 && i > 0) { // Weekly pattern
        seasonal.push(detrended[i] - seasonal[seasonal.length - 1]);
      } else {
        seasonal.push(0);
      }
    }

    const noise = detrended.map((d, i) => d - seasonal[i]);

    return [
      { name: 'trend', values: trend },
      { name: 'seasonal', values: seasonal },
      { name: 'noise', values: noise },
    ];
  }
}

export default CorrelationCartogramBuilder;
