// Prismatik API client, pinned to /api/v1 · Author: Aaron Stovall · 0.1.0
// Timeouts on every request; job polling with a hard client-side deadline.
import type { RunResult, SessionStatus, StrategySpec } from './types';

declare global {
  interface Window { PRISMATIK: { token: string; version: string } }
}

const API = '/api/v1';
const REQUEST_TIMEOUT_MS = 20_000;
const POLL_INTERVAL_MS = 1_500;
const POLL_MAX_MS = 10 * 60 * 1000;

function token(): string {
  return window.PRISMATIK?.token ?? '';
}

async function call<T>(path: string, init: RequestInit = {}): Promise<T> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const resp = await fetch(API + path, {
      ...init,
      signal: controller.signal,
      headers: {
        Authorization: `Bearer ${token()}`,
        'Content-Type': 'application/json',
        ...(init.headers ?? {})
      }
    });
    const body = await resp.json().catch(() => ({}));
    if (!resp.ok) {
      const detail = typeof body.detail === 'string' ? body.detail : JSON.stringify(body);
      throw new Error(`${resp.status}: ${detail}`);
    }
    return body as T;
  } finally {
    clearTimeout(timer);
  }
}

export interface RunRequest {
  symbol: string;
  granularity_s: number;
  days: number;
  initial_equity_usd: number;
  strategy: StrategySpec;
  folds: number;
}

export async function startJob(kind: 'backtest' | 'walkforward', req: RunRequest): Promise<string> {
  const { job_id } = await call<{ job_id: string }>(`/jobs/${kind}`, {
    method: 'POST',
    body: JSON.stringify(req)
  });
  return job_id;
}

export async function pollJob(jobId: string): Promise<RunResult> {
  const started = Date.now();
  for (;;) {
    if (Date.now() - started > POLL_MAX_MS) throw new Error('job timed out client-side');
    const job = await call<{ status: string; result?: RunResult; error?: string }>(`/jobs/${jobId}`);
    if (job.status === 'done' && job.result) return job.result;
    if (job.status === 'error') throw new Error(job.error ?? 'job failed');
    await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS));
  }
}

export interface SessionStartRequest {
  symbol: string;
  granularity_s: number;
  strategy: StrategySpec;
  initial_equity_usd: number;
  window_bars: number;
  execution: 'paper' | 'alpaca';
  live: boolean;
  live_confirm: string;
}

export async function startSession(req: SessionStartRequest): Promise<{ session_id: string; mode: string }> {
  return call('/sessions/start', { method: 'POST', body: JSON.stringify(req) });
}
export async function stopSession(id: string): Promise<void> {
  await call(`/sessions/${id}/stop`, { method: 'POST' });
}
export async function sessionStatus(id: string): Promise<SessionStatus> {
  return call(`/sessions/${id}`);
}
export function sessionSocket(id: string): WebSocket {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  return new WebSocket(
    `${proto}://${location.host}${API}/sessions/${id}/stream?token=${encodeURIComponent(token())}`
  );
}

export const fmtPct = (x: number | null | undefined, dp = 2): string =>
  x == null || Number.isNaN(x) ? '—' : (x * 100).toFixed(dp) + '%';
export const fmtNum = (x: number | null | undefined, dp = 3): string =>
  x == null || Number.isNaN(x) ? '—' : x.toFixed(dp);
export const fmtUsd = (x: number | null | undefined): string =>
  x == null || Number.isNaN(x)
    ? '—'
    : x.toLocaleString('en-US', { style: 'currency', currency: 'USD' });
export const signClass = (x: number): string => (x > 0 ? 'pos' : x < 0 ? 'neg' : '');
