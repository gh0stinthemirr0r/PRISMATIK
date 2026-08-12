/**
 * Prismatik Desktop - API Client
 * Interfaces with backend server at http://127.0.0.1:8787
 */

const API_BASE = 'http://127.0.0.1:8787/api/v1';
let apiToken: string | null = null;

/**
 * Initialize API client with bearer token
 */
export function initializeAPI(token: string) {
  apiToken = token;
  localStorage.setItem('api_token', token);
}

/**
 * Get stored API token
 */
export function getAPIToken(): string | null {
  return apiToken || localStorage.getItem('api_token');
}

/**
 * Generic fetch wrapper with bearer token
 */
async function apiCall<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const token = getAPIToken();
  const headers = {
    'Content-Type': 'application/json',
    ...(token && { 'Authorization': `Bearer ${token}` }),
    ...options.headers,
  };

  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers,
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`API Error (${response.status}): ${error}`);
  }

  return response.json();
}

// ============================================================================
// HEALTH & STATUS
// ============================================================================

export interface HealthResponse {
  status: 'ok' | 'degraded';
}

export async function getHealth(): Promise<HealthResponse> {
  return fetch(`${API_BASE}/health`).then(r => r.json());
}

export interface DetailedHealthResponse {
  status: 'healthy' | 'degraded';
  coinbase_reachable: boolean;
  alpaca_configured: boolean;
  uptime_seconds: number;
  capabilities: string[];
}

export async function getDetailedHealth(): Promise<DetailedHealthResponse> {
  return apiCall('/health/detailed');
}

export interface MetricsResponse {
  jobs_complete: number;
  jobs_pending: number;
  sessions_running: number;
  sessions_stopped: number;
  uptime_seconds: number;
  data_sources: string[];
  brokers_enabled: string[];
  capabilities: string[];
}

export async function getMetrics(): Promise<MetricsResponse> {
  return apiCall('/metrics');
}

// ============================================================================
// CONFIGURATION
// ============================================================================

export interface ConfigResponse {
  max_jobs_per_batch: number;
  max_job_id_length: number;
  max_event_tail: number;
  max_search_results: number;
  supported_granularities: string[];
  supported_backtesting_engines: string[];
}

export async function getConfig(): Promise<ConfigResponse> {
  return apiCall('/config');
}

export interface MetaResponse {
  strategies: string[];
  granularities: string[];
  symbols: string[];
}

export async function getMeta(): Promise<MetaResponse> {
  return apiCall('/meta');
}

// ============================================================================
// VALIDATION
// ============================================================================

export interface ValidationRequest {
  job_type: 'backtest' | 'walkforward';
  symbol: string;
  strategy: string;
  initial_capital: number;
  start_date: string;
  end_date: string;
  granularity: string;
  stop_loss_pct?: number;
  take_profit_pct?: number;
}

export interface ValidationError {
  field: string;
  message: string;
}

export interface ValidationResponse {
  valid: boolean;
  errors: ValidationError[];
}

export async function validateStrategy(
  request: ValidationRequest
): Promise<ValidationResponse> {
  return apiCall('/validate/strategy', {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

// ============================================================================
// JOBS
// ============================================================================

export interface JobMetadata {
  id: string;
  job_type: 'backtest' | 'walkforward';
  symbol: string;
  status: 'pending' | 'complete';
  created_at: string;
  completed_at?: string;
}

export interface JobsListResponse {
  jobs: JobMetadata[];
  total: number;
}

export async function listJobs(limit: number = 50): Promise<JobsListResponse> {
  return apiCall(`/jobs?limit=${limit}`);
}

export interface JobSearchQuery {
  symbol?: string;
  status?: 'pending' | 'complete';
  limit?: number;
}

export async function searchJobs(query: JobSearchQuery): Promise<JobsListResponse> {
  const params = new URLSearchParams();
  if (query.symbol) params.append('symbol', query.symbol);
  if (query.status) params.append('status', query.status);
  if (query.limit) params.append('limit', String(query.limit));

  return apiCall(`/jobs/search?${params.toString()}`);
}

export interface JobResult {
  id: string;
  job_type: string;
  symbol: string;
  verdict?: string;
  metrics?: Record<string, unknown>;
}

export async function getJob(jobId: string): Promise<JobResult> {
  return apiCall(`/jobs/${jobId}`);
}

export interface SubmitJobRequest {
  symbol: string;
  strategy: string;
  initial_capital: number;
  start_date: string;
  end_date: string;
  granularity: string;
  stop_loss_pct?: number;
  take_profit_pct?: number;
}

export interface SubmitJobResponse {
  id: string;
}

export async function submitBacktest(request: SubmitJobRequest): Promise<SubmitJobResponse> {
  return apiCall('/jobs/backtest', {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

export async function submitWalkforward(request: SubmitJobRequest): Promise<SubmitJobResponse> {
  return apiCall('/jobs/walkforward', {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

export interface BatchJobRequest {
  jobs: SubmitJobRequest[];
}

export interface BatchJobResponse {
  ids: string[];
}

export async function submitBacktestBatch(
  request: BatchJobRequest
): Promise<BatchJobResponse> {
  return apiCall('/jobs/batch/backtest', {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

export async function submitWalkforwardBatch(
  request: BatchJobRequest
): Promise<BatchJobResponse> {
  return apiCall('/jobs/batch/walkforward', {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

// ============================================================================
// SESSIONS
// ============================================================================

export interface SessionMetadata {
  id: string;
  status: 'running' | 'stopped';
  mode: 'paper' | 'live';
  started_at: string;
  stopped_at?: string;
}

export interface SessionsListResponse {
  sessions: SessionMetadata[];
  total: number;
}

export async function listSessions(): Promise<SessionsListResponse> {
  return apiCall('/sessions');
}

export interface SessionDetail extends SessionMetadata {
  event_count: number;
}

export async function getSession(sessionId: string): Promise<SessionDetail> {
  return apiCall(`/sessions/${sessionId}`);
}

export interface SessionEvent {
  timestamp: string;
  event_type: string;
  data: Record<string, unknown>;
}

export interface SessionEventsResponse {
  events: SessionEvent[];
  session_id: string;
}

export async function getSessionEvents(
  sessionId: string,
  limit: number = 100
): Promise<SessionEventsResponse> {
  return apiCall(`/sessions/${sessionId}/events?limit=${limit}`);
}

export interface StartSessionRequest {
  mode: 'paper' | 'live';
}

export interface StartSessionResponse {
  id: string;
  status: string;
}

export async function startSession(request: StartSessionRequest): Promise<StartSessionResponse> {
  return apiCall('/sessions/start', {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

export async function stopSession(sessionId: string): Promise<{ status: string }> {
  return apiCall(`/sessions/${sessionId}/stop`, {
    method: 'POST',
  });
}

// ============================================================================
// OVERVIEW
// ============================================================================

export interface OverviewResponse {
  latest_jobs: JobMetadata[];
  latest_sessions: SessionMetadata[];
  metrics: MetricsResponse;
}

export async function getOverview(): Promise<OverviewResponse> {
  return apiCall('/overview');
}

// ============================================================================
// LIVE TICKER (Phase 1 - Simulated ~10 Hz tick generation)
// ============================================================================

export interface TickLive {
  symbol: string;
  price: number;
  bid: number;
  ask: number;
  spread: number;
  timestamp_ms: number;
  source: 'cassette' | 'live';
}

export interface OrderBookLevel {
  price: number;
  volume: number;
  count: number;
}

export interface OrderBookSnapshot {
  symbol: string;
  bids: OrderBookLevel[];
  asks: OrderBookLevel[];
  spread: number;
}

/**
 * Get available symbols for ticker subscriptions (autocomplete).
 */
export async function getTickerSymbols(): Promise<string[]> {
  return apiCall<string[]>('/ticker/available-symbols');
}

/**
 * Subscribe to real-time tick stream for a specific asset.
 * Returns immediately; ticks will be pushed via WebSocket in future phases.
 */
export async function subscribeToTicker(symbol: string): Promise<void> {
  await apiCall('/ticker/subscribe', {
    method: 'POST',
    body: JSON.stringify({ symbol }),
  });
}

/**
 * Unsubscribe from real-time tick stream.
 */
export async function unsubscribeFromTicker(symbol: string): Promise<void> {
  await apiCall('/ticker/unsubscribe', {
    method: 'POST',
    body: JSON.stringify({ symbol }),
  });
}

/**
 * Get latest quote (single-shot, no subscription required).
 */
export async function getQuote(symbol: string): Promise<{
  coingecko_id: string;
  symbol: string;
  name: string;
  price: string;
  change_24h_pct: string | null;
  volume_24h: string | null;
  market_cap: string | null;
  provider: string;
  event_time: string;
  retrieved_at: string;
  quality: number;
}> {
  return apiCall(`/quotes?symbol=${encodeURIComponent(symbol)}`);
}

/**
 * Get all quotes for currently active subscriptions.
 */
export async function getAllQuotes(): Promise<TickLive[]> {
  return apiCall<TickLive[]>('/ticker/all-quotes');
}

/**
 * Get subscriber count (total active ticker subscriptions).
 */
export async function getSubscriberCount(): Promise<number> {
  return apiCall<number>('/ticker/subscriber-count');
}

/**
 * Get latest live tick from the streaming engine.
 */
export async function getLiveQuote(symbol: string): Promise<TickLive> {
  return apiCall<TickLive>(`/ticker/live-quote/${encodeURIComponent(symbol)}`);
}

/**
 * Get order book snapshot from the streaming engine.
 */
export async function getOrderBook(symbol: string): Promise<OrderBookSnapshot> {
  return apiCall<OrderBookSnapshot>(`/ticker/order-book/${encodeURIComponent(symbol)}`);
}

