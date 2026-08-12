/**
 * Prismatik Desktop - Svelte Stores
 * Manages global state and real-time data
 */

import { writable, derived, readable } from 'svelte/store';
import type {
  MetricsResponse,
  DetailedHealthResponse,
  JobMetadata,
  SessionMetadata,
  ConfigResponse,
} from './api';
import {
  getMetrics,
  getDetailedHealth,
  listJobs,
  listSessions,
  getConfig,
} from './api';

export { initializeAPI } from './api';

// ============================================================================
// STATE STORES
// ============================================================================

export const apiToken = writable<string | null>(null);
export const isConnected = writable(false);
export const connectionError = writable<string | null>(null);

// Metrics
export const metrics = writable<MetricsResponse | null>(null);
export const metricsError = writable<string | null>(null);

// Health
export const health = writable<DetailedHealthResponse | null>(null);
export const healthError = writable<string | null>(null);

// Jobs
export const jobs = writable<JobMetadata[]>([]);
export const jobsLoading = writable(false);
export const jobsError = writable<string | null>(null);

// Sessions
export const sessions = writable<SessionMetadata[]>([]);
export const sessionsLoading = writable(false);
export const sessionsError = writable<string | null>(null);

// Config
export const config = writable<ConfigResponse | null>(null);
export const configError = writable<string | null>(null);

// UI State
export const currentView = writable<
  'dashboard' | 'jobs' | 'sessions' | 'validation' | 'settings'
>('dashboard');
export const selectedJobId = writable<string | null>(null);
export const selectedSessionId = writable<string | null>(null);

// ============================================================================
// DERIVED STORES
// ============================================================================

export const isHealthy = derived(health, ($health) => {
  return $health?.status === 'healthy';
});

export const activeSessions = derived(sessions, ($sessions) => {
  return $sessions.filter((s) => s.status === 'running');
});

export const pendingJobs = derived(jobs, ($jobs) => {
  return $jobs.filter((j) => j.status === 'pending');
});

export const completedJobs = derived(jobs, ($jobs) => {
  return $jobs.filter((j) => j.status === 'complete');
});

// ============================================================================
// SYNC FUNCTIONS
// ============================================================================

export async function syncMetrics() {
  try {
    metricsError.set(null);
    const data = await getMetrics();
    metrics.set(data);
    isConnected.set(true);
    connectionError.set(null);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Unknown error';
    metricsError.set(message);
    connectionError.set(message);
    isConnected.set(false);
  }
}

export async function syncHealth() {
  try {
    healthError.set(null);
    const data = await getDetailedHealth();
    health.set(data);
    isConnected.set(true);
    connectionError.set(null);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Unknown error';
    healthError.set(message);
    connectionError.set(message);
    isConnected.set(false);
  }
}

export async function syncJobs() {
  try {
    jobsLoading.set(true);
    jobsError.set(null);
    const response = await listJobs(100);
    jobs.set(response.jobs);
    isConnected.set(true);
    connectionError.set(null);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Unknown error';
    jobsError.set(message);
    connectionError.set(message);
    isConnected.set(false);
  } finally {
    jobsLoading.set(false);
  }
}

export async function syncSessions() {
  try {
    sessionsLoading.set(true);
    sessionsError.set(null);
    const response = await listSessions();
    sessions.set(response.sessions);
    isConnected.set(true);
    connectionError.set(null);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Unknown error';
    sessionsError.set(message);
    connectionError.set(message);
    isConnected.set(false);
  } finally {
    sessionsLoading.set(false);
  }
}

export async function syncConfig() {
  try {
    configError.set(null);
    const data = await getConfig();
    config.set(data);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Unknown error';
    configError.set(message);
  }
}

/**
 * Perform full sync of all data
 */
export async function syncAll() {
  await Promise.all([syncMetrics(), syncHealth(), syncJobs(), syncSessions(), syncConfig()]);
}
