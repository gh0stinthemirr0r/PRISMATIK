// Prismatik API types · Author: Aaron Stovall · 0.1.0
export interface Metrics {
  total_return: number;
  cagr: number;
  ann_volatility: number;
  sharpe: number;
  sortino: number;
  max_drawdown: number;
  calmar: number;
  n_periods: number;
}
export interface EquityPoint { time: string; value: number }
export interface BootstrapReport {
  n_sims: number;
  block_bars: number;
  total_return_p05: number;
  total_return_p50: number;
  total_return_p95: number;
  prob_loss: number;
}
export interface BacktestResult {
  kind: 'backtest';
  symbol: string;
  strategy: string;
  bars: number;
  metrics: Metrics;
  benchmark_metrics: Metrics;
  n_trades: number;
  cost_drag: number;
  beats_benchmark: boolean;
  equity: EquityPoint[];
  benchmark_equity: EquityPoint[];
  verdict: string;
}
export interface WalkforwardResult {
  kind: 'walkforward';
  symbol: string;
  strategy: string;
  bars: number;
  oos_metrics: Metrics;
  oos_equity: EquityPoint[];
  chosen_params: Record<string, number>[];
  param_stability: Record<string, number>;
  stable: boolean;
  n_folds: number;
  configs_searched: number;
  caution: string;
  bootstrap: BootstrapReport | null;
  verdict: string;
}
export type RunResult = BacktestResult | WalkforwardResult;
export interface SessionStatus {
  session_id: string;
  running: boolean;
  halted: boolean;
  live: boolean;
  symbol: string;
  strategy: string;
  last_price: number | null;
  equity: number;
  cash_usd: number;
  position_units: number;
  events: JournalEvent[];
}
export interface JournalEvent {
  ts?: string;
  event: string;
  [key: string]: unknown;
}
export interface StrategySpec {
  kind: string;
  params: Record<string, number>;
  vol_target: number | null;
}
