<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke, isTauri } from '@tauri-apps/api/core';
  import { TrendingUp, TrendingDown, Minus, Loader2, CheckCircle, XCircle, Clock, Volume2 } from 'lucide-svelte';

  interface Prediction {
    id: string;
    entity: string;
    direction: string;
    confidence: number;
    intervalLow: number;
    intervalHigh: number;
    horizon: string;
    regime: string;
    evidence: string[];
    falsifiers: string[];
    model: string;
    provider: string;
    status: string;
    createdAt: number;
  }

  interface TrackRecord {
    total: number;
    correct: number;
    accuracy: number;
    avgConfidence: number;
    calibrationError: number;
  }

  interface PredictionState {
    predictions: Prediction[];
    trackRecord: TrackRecord;
  }

  let predictions = $state<Prediction[]>([]);
  let trackRecord = $state<TrackRecord>({ total: 0, correct: 0, accuracy: 0, avgConfidence: 0, calibrationError: 0 });
  let generating = $state(false);
  let error = $state('');
  let entity = $state('NVDA');
  let question = $state('');
  let horizon = $state('7d');
  let selectedPrediction = $state<Prediction | null>(null);

  const DIRECTION_ICONS: Record<string, typeof TrendingUp> = {
    bullish: TrendingUp,
    bearish: TrendingDown,
    neutral: Minus,
  };

  const DIRECTION_COLORS: Record<string, string> = {
    bullish: '#34d399',
    bearish: '#f87171',
    neutral: '#f59e0b',
  };

  const STATUS_ICONS: Record<string, typeof CheckCircle> = {
    confirmed: CheckCircle,
    invalidated: XCircle,
    pending: Clock,
    expired: Clock,
  };

  async function load() {
    if (!isTauri()) return;
    try {
      const state = await invoke<PredictionState>('get_predictions');
      predictions = state.predictions.sort((a, b) => b.createdAt - a.createdAt);
      trackRecord = state.trackRecord;
    } catch (e) {
      error = String(e);
    }
  }

  async function generate() {
    if (!question.trim() || generating) return;
    generating = true;
    error = '';
    try {
      await invoke('generate_live_prediction', {
        entity,
        question: question.trim(),
        horizon,
      });
      question = '';
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      generating = false;
    }
  }

  async function resolve(id: string, outcome: string) {
    try {
      await invoke('resolve_prediction', { id, outcome });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function speak(text: string) {
    try {
      const result = await invoke<{ audioBase64: string; format: string }>('tts_speak', { text });
      const audio = new Audio(`data:audio/${result.format};base64,${result.audioBase64}`);
      audio.play();
    } catch (e) {
      // TTS not available — silent fail
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); generate(); }
  }

  onMount(() => { void load(); });
</script>

<div class="pk-predictions">
  <div class="pk-pred-header">
    <h3>Live Predictions</h3>
    <div class="pk-pred-stats">
      <span class="stat">{trackRecord.total} predictions</span>
      <span class="stat" style="color: {trackRecord.accuracy > 0.55 ? '#34d399' : trackRecord.accuracy < 0.45 ? '#f87171' : '#f59e0b'}">
        {(trackRecord.accuracy * 100).toFixed(0)}% accuracy
      </span>
      <span class="stat" style="color: #6b7280">cal err: {(trackRecord.calibrationError * 100).toFixed(1)}%</span>
    </div>
  </div>

  <div class="pk-pred-input">
    <select bind:value={entity}>
      <option>NVDA</option><option>AAPL</option><option>TSLA</option><option>BTC</option>
      <option>ETH</option><option>SPY</option><option>QQQ</option><option>MSFT</option>
    </select>
    <input type="text" bind:value={question} {onkeydown} placeholder="Ask a prediction question…" disabled={generating} />
    <select bind:value={horizon}>
      <option value="1d">1d</option><option value="7d">7d</option>
      <option value="30d">30d</option><option value="90d">90d</option>
    </select>
    <button onclick={generate} disabled={generating || !question.trim()}>
      {#if generating}<Loader2 size={14} class="pk-spin" />{:else}Predict{/if}
    </button>
  </div>

  {#if error}
    <div class="pk-pred-error">{error}</div>
  {/if}

  <div class="pk-pred-list">
    {#each predictions as pred}
      <div class="pk-pred-card" class:selected={selectedPrediction?.id === pred.id}
           onclick={() => selectedPrediction = selectedPrediction?.id === pred.id ? null : pred}>
        <div class="pk-pred-card-top">
          <span class="pk-pred-entity">{pred.entity}</span>
          <span class="pk-pred-direction" style="color: {DIRECTION_COLORS[pred.direction]}">
            {pred.direction.toUpperCase()}
          </span>
          <span class="pk-pred-confidence">{(pred.confidence * 100).toFixed(0)}%</span>
          <span class="pk-pred-horizon">{pred.horizon}</span>
          <span class="pk-pred-status {pred.status}">
            {#if pred.status === 'confirmed'}<CheckCircle size={12} />
            {:else if pred.status === 'invalidated'}<XCircle size={12} />
            {:else}<Clock size={12} />{/if}
            {pred.status}
          </span>
        </div>
        <div class="pk-pred-card-mid">
          <span class="pk-pred-interval">[{(pred.intervalLow * 100).toFixed(1)}%, {(pred.intervalHigh * 100).toFixed(1)}%]</span>
          <span class="pk-pred-regime">{pred.regime}</span>
          <span class="pk-pred-model">{pred.model}</span>
        </div>

        {#if selectedPrediction?.id === pred.id}
          <div class="pk-pred-detail">
            <div class="pk-pred-section">
              <strong>Evidence</strong>
              <ul>{#each pred.evidence as e}<li>{e}</li>{/each}</ul>
            </div>
            <div class="pk-pred-section">
              <strong>Falsifiers</strong>
              <ul>{#each pred.falsifiers as f}<li>{f}</li>{/each}</ul>
            </div>
            {#if pred.status === 'pending'}
              <div class="pk-pred-actions">
                <button class="pk-btn-correct" onclick={() => resolve(pred.id, 'correct')}>Mark Correct</button>
                <button class="pk-btn-incorrect" onclick={() => resolve(pred.id, 'incorrect')}>Mark Incorrect</button>
                <button onclick={() => speak(`${pred.entity} prediction: ${pred.direction} with ${(pred.confidence * 100).toFixed(0)}% confidence. Interval: ${pred.intervalLow} to ${pred.intervalHigh}`)}>
                  <Volume2 size={12} /> Read Aloud
                </button>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/each}

    {#if predictions.length === 0}
      <div class="pk-pred-empty">
        <p>No predictions yet. Ask a question above to generate one.</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .pk-predictions { display: flex; flex-direction: column; height: 100%; background: var(--p-surface0); overflow-y: auto; }
  .pk-pred-header { padding: 14px 16px; border-bottom: 1px solid var(--p-border); }
  .pk-pred-header h3 { margin: 0 0 6px; font-size: 0.8125rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em; }
  .pk-pred-stats { display: flex; gap: 12px; font-size: 0.6875rem; font-family: var(--font-mono); }
  .stat { color: var(--p-text-dim); }
  .pk-pred-input { display: flex; gap: 6px; padding: 10px 16px; border-bottom: 1px solid var(--p-border); }
  .pk-pred-input select, .pk-pred-input input { background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 4px; color: var(--p-text); font-size: 0.75rem; padding: 6px 8px; font-family: var(--font-mono); }
  .pk-pred-input input { flex: 1; }
  .pk-pred-input button { background: var(--p-accent); color: #000; border: none; border-radius: 4px; padding: 6px 14px; font-size: 0.75rem; font-weight: 600; cursor: pointer; display: flex; align-items: center; gap: 4px; }
  .pk-pred-input button:disabled { opacity: 0.3; }
  .pk-pred-error { padding: 8px 16px; background: rgba(239,68,68,0.1); color: #f87171; font-size: 0.6875rem; font-family: var(--font-mono); }
  .pk-pred-list { flex: 1; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 6px; }
  .pk-pred-card { background: var(--p-surface1); border: 1px solid var(--p-border); border-radius: 6px; padding: 10px 12px; cursor: pointer; transition: border-color 0.15s; }
  .pk-pred-card:hover { border-color: var(--p-accent); }
  .pk-pred-card.selected { border-color: var(--p-accent); }
  .pk-pred-card-top { display: flex; align-items: center; gap: 10px; font-size: 0.75rem; font-family: var(--font-mono); }
  .pk-pred-entity { font-weight: 700; color: var(--p-text); }
  .pk-pred-direction { font-weight: 600; text-transform: uppercase; }
  .pk-pred-confidence { color: var(--p-text-dim); }
  .pk-pred-horizon { color: var(--p-text-dim); font-size: 0.625rem; }
  .pk-pred-status { display: flex; align-items: center; gap: 3px; font-size: 0.625rem; text-transform: uppercase; }
  .pk-pred-status.pending { color: #f59e0b; }
  .pk-pred-status.confirmed { color: #34d399; }
  .pk-pred-status.invalidated { color: #f87171; }
  .pk-pred-status.expired { color: #6b7280; }
  .pk-pred-card-mid { display: flex; gap: 10px; margin-top: 4px; font-size: 0.625rem; font-family: var(--font-mono); color: var(--p-text-dim); }
  .pk-pred-detail { margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--p-border); }
  .pk-pred-section { margin-bottom: 8px; }
  .pk-pred-section strong { font-size: 0.6875rem; color: var(--p-accent); text-transform: uppercase; }
  .pk-pred-section ul { margin: 4px 0 0; padding-left: 16px; font-size: 0.6875rem; color: var(--p-text-secondary); }
  .pk-pred-actions { display: flex; gap: 6px; margin-top: 8px; }
  .pk-pred-actions button { padding: 5px 10px; border: 1px solid var(--p-border); border-radius: 4px; background: var(--p-surface0); color: var(--p-text); font-size: 0.625rem; cursor: pointer; display: flex; align-items: center; gap: 4px; }
  .pk-btn-correct { border-color: #34d399 !important; color: #34d399 !important; }
  .pk-btn-incorrect { border-color: #f87171 !important; color: #f87171 !important; }
  .pk-pred-empty { display: flex; align-items: center; justify-content: center; height: 100%; color: var(--p-text-dim); font-size: 0.8125rem; }
  :global(.pk-spin) { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
