/**
 * Live Prediction Pipeline — connects model providers to the agent council,
 * generates calibrated predictions, and routes them to visualizations.
 */

export interface LivePrediction {
  id: string;
  timestamp: number;
  entity: string;
  direction: 'bullish' | 'bearish' | 'neutral';
  confidence: number;
  interval: [number, number];
  horizon: string;
  regime: string;
  evidence: string[];
  falsifiers: string[];
  model: string;
  provider: string;
  status: 'pending' | 'confirmed' | 'invalidated' | 'expired';
}

export interface PredictionRequest {
  entity: string;
  question: string;
  horizon?: string;
  providerId?: string;
  model?: string;
}

/**
 * Build a structured prediction prompt that forces the model to output
 * a calibrated prediction with intervals, evidence, and falsification conditions.
 */
export function buildPredictionPrompt(request: PredictionRequest, marketData: string): string {
  return `You are PRISMATIK's prediction engine. Analyze the following market data and produce a calibrated prediction.

MARKET DATA:
${marketData}

TASK: ${request.question}
ENTITY: ${request.entity}
HORIZON: ${request.horizon ?? '7 days'}

You MUST respond in this exact JSON format:
{
  "direction": "bullish" | "bearish" | "neutral",
  "confidence": <0.0 to 1.0>,
  "interval_low": <lower bound return>,
  "interval_high": <upper bound return>,
  "evidence": ["<evidence point 1>", "<evidence point 2>", ...],
  "falsifiers": ["<what would invalidate this>", ...],
  "regime": "<current regime assessment>",
  "reasoning": "<brief reasoning>"
}

Rules:
- Confidence is a probability, not a certainty. Be calibrated.
- Intervals must be honest — wider when uncertain.
- Evidence must cite specific data points.
- Falsifiers must be concrete and checkable.
- If data is insufficient, say so with confidence < 0.3.`;
}

/**
 * Parse a model response into a LivePrediction.
 */
export function parsePredictionResponse(
  raw: string,
  request: PredictionRequest,
  provider: string,
  model: string
): LivePrediction | null {
  try {
    // extract JSON from response (handle markdown code blocks)
    const jsonMatch = raw.match(/\{[\s\S]*\}/);
    if (!jsonMatch) return null;
    const parsed = JSON.parse(jsonMatch[0]);

    return {
      id: `pred_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      timestamp: Date.now(),
      entity: request.entity,
      direction: parsed.direction ?? 'neutral',
      confidence: Math.max(0, Math.min(1, parsed.confidence ?? 0.5)),
      interval: [parsed.interval_low ?? -0.05, parsed.interval_high ?? 0.05],
      horizon: request.horizon ?? '7d',
      regime: parsed.regime ?? 'unknown',
      evidence: parsed.evidence ?? [],
      falsifiers: parsed.falsifiers ?? [],
      model,
      provider,
      status: 'pending',
    };
  } catch {
    return null;
  }
}

/**
 * Evaluate whether a prediction's falsification conditions have been met.
 */
export function checkFalsification(
  prediction: LivePrediction,
  currentData: Record<string, number>
): LivePrediction {
  // simple: if price moved outside the interval, invalidate
  const priceKey = prediction.entity;
  if (currentData[priceKey] !== undefined) {
    const returnVal = currentData[priceKey];
    if (returnVal < prediction.interval[0] || returnVal > prediction.interval[1]) {
      return { ...prediction, status: 'invalidated' };
    }
  }

  // check expiry based on horizon
  const horizonDays = parseInt(prediction.horizon) || 7;
  const ageMs = Date.now() - prediction.timestamp;
  if (ageMs > horizonDays * 24 * 60 * 60 * 1000) {
    return { ...prediction, status: 'expired' };
  }

  return prediction;
}

/**
 * Track prediction accuracy over time for the learning loop.
 */
export interface PredictionTrackRecord {
  total: number;
  correct: number;
  accuracy: number;
  avgConfidence: number;
  calibrationError: number;  // |accuracy - avgConfidence|
  byRegime: Record<string, { total: number; correct: number }>;
  byEntity: Record<string, { total: number; correct: number }>;
}

export function computeTrackRecord(predictions: LivePrediction[]): PredictionTrackRecord {
  const resolved = predictions.filter(p => p.status === 'confirmed' || p.status === 'invalidated');
  const correct = resolved.filter(p => p.status === 'confirmed').length;
  const avgConfidence = resolved.length > 0
    ? resolved.reduce((s, p) => s + p.confidence, 0) / resolved.length
    : 0;

  const byRegime: Record<string, { total: number; correct: number }> = {};
  const byEntity: Record<string, { total: number; correct: number }> = {};

  resolved.forEach(p => {
    if (!byRegime[p.regime]) byRegime[p.regime] = { total: 0, correct: 0 };
    byRegime[p.regime].total++;
    if (p.status === 'confirmed') byRegime[p.regime].correct++;

    if (!byEntity[p.entity]) byEntity[p.entity] = { total: 0, correct: 0 };
    byEntity[p.entity].total++;
    if (p.status === 'confirmed') byEntity[p.entity].correct++;
  });

  const accuracy = resolved.length > 0 ? correct / resolved.length : 0;

  return {
    total: resolved.length,
    correct,
    accuracy,
    avgConfidence,
    calibrationError: Math.abs(accuracy - avgConfidence),
    byRegime,
    byEntity,
  };
}
