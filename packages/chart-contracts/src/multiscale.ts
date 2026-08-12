// Multi-Scale Trend Decomposition
// P1-EX-05 / Architecture Evolution §46

import type { CandlestickPoint } from './index.js';

/**
 * Wavelet-based multi-scale decomposition for OHLCV series.
 * Separates price action into trend, volatility regimes, and noise components.
 */
export interface ScaleDecomposition {
  scale: string;
  name: string;
  values: number[];
  interpretation: 'trend' | 'regime_shift' | 'noise';
}

/** Multi-scale decomposition using moving average profiles */
export class MultiScaleTrendDecomposer {
  private scales: { [key: string]: (data: number[]) => number[] } = {
    // Trend components
    'trend-1d': data => this.movingAverage(data, 20),
    'trend-5d': data => this.movingAverage(data, 60),
    
    // Volatility regimes (thresholds)
    'vol-high': data => this.volatilityRegime(data, 1.5),
    'vol-low': data => this.volatilityRegime(data, 0.7),

    // Seasonal components
    'intraday': data => this.seasonalComponent(data, 5), // 5-minute bars
  };

  constructor() {}

  movingAverage(data: number[], window: number): number[] {
    const result: number[] = [];
    let sum = 0;

    for (let i = 0; i < data.length; i++) {
      if (i < window - 1) {
        result.push(data[i]);
      } else {
        sum += data[i] - data[i - window];
        result.push(sum / window);
      }
    }

    return result;
  }

  volatilityRegime(prices: number[], threshold: number): number[] {
    const returns = prices.slice(1).map((p, i) => p / prices[i] - 1);
    const rollingVol = this.movingAverage(returns.map(r => r ** 2), 20);

    return rollingVol.map(v => v > threshold ? 1 : 0);
  }

  seasonalComponent(data: number[], period: number): number[] {
    // Simple sinusoidal approximation of daily/weekly seasonality
    const n = data.length;
    if (n < period * 2) return data.map(() => 0);

    const result = new Array(n).fill(0);
    
    for (let i = 0; i < n - period + 1; i++) {
      const sum = data.slice(i, i + period).reduce((a, b) => a + b, 0);
      result[i] = sum / period;
    }

    // Pad remaining with last value
    while (result.length < n) {
      result.push(result[result.length - 1]);
    }

    return result;
  }

  /** Decompose OHLCV series into multi-scale components */
  decompose(candles: CandlestickPoint[]): ScaleDecomposition[] {
    const prices = candles.map((c) => c.close);
    
    return [
      {
        scale: 'trend-1d',
        name: 'Trend (20MA)',
        values: this.movingAverage(prices, 20),
        interpretation: 'trend' as const,
      },
      {
        scale: 'vol-high',
        name: 'High Volatility Regime',
        values: this.volatilityRegime(prices, 1.5),
        interpretation: 'regime_shift' as const,
      },
      {
        scale: 'intraday',
        name: 'Intraday Pattern (Weekly)',
        values: this.seasonalComponent(prices, 7), // Assuming 1-day for simplicity
        interpretation: 'trend' as const,
      },
    ];
  }
}

export default MultiScaleTrendDecomposer;