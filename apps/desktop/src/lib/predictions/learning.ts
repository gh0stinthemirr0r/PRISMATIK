/**
 * Auto-Learn / Evolve feedback loop.
 * Tracks prediction outcomes, detects drift, adjusts model weights.
 */

export interface LearningRecord {
  predictionId: string;
  timestamp: number;
  entity: string;
  predicted: { direction: string; confidence: number; interval: [number, number] };
  actual: { returnVal: number; direction: string };
  outcome: 'correct' | 'incorrect' | 'partial';
  model: string;
  regime: string;
}

export interface EvolutionSignal {
  type: 'calibration_drift' | 'regime_shift' | 'model_degradation' | 'feature_drift' | 'novel_pattern';
  severity: number; // 0-1
  description: string;
  detected: number;
  entity?: string;
  model?: string;
  regime?: string;
}

export interface ModelWeight {
  model: string;
  weight: number;
  trackRecord: { total: number; correct: number };
  lastUpdated: number;
}

export class LearningEngine {
  private records: LearningRecord[] = [];
  private weights: Map<string, ModelWeight> = new Map();
  private signals: EvolutionSignal[] = [];

  recordOutcome(record: LearningRecord) {
    this.records.push(record);
    this.updateWeights(record);
    this.detectDrift();
  }

  private updateWeights(record: LearningRecord) {
    let w = this.weights.get(record.model);
    if (!w) {
      w = { model: record.model, weight: 1.0, trackRecord: { total: 0, correct: 0 }, lastUpdated: Date.now() };
      this.weights.set(record.model, w);
    }
    w.trackRecord.total++;
    if (record.outcome === 'correct') w.trackRecord.correct++;

    // EMA-style weight update
    const accuracy = w.trackRecord.correct / w.trackRecord.total;
    const target = accuracy > 0.55 ? 1 + (accuracy - 0.5) * 2 : accuracy < 0.45 ? accuracy * 0.8 : 1;
    w.weight = w.weight * 0.9 + target * 0.1;
    w.lastUpdated = Date.now();
  }

  private detectDrift() {
    if (this.records.length < 20) return;

    const recent = this.records.slice(-20);
    const older = this.records.slice(-40, -20);

    if (older.length < 10) return;

    const recentAcc = recent.filter(r => r.outcome === 'correct').length / recent.length;
    const olderAcc = older.filter(r => r.outcome === 'correct').length / older.length;

    // calibration drift
    const recentConf = recent.reduce((s, r) => s + r.predicted.confidence, 0) / recent.length;
    if (Math.abs(recentAcc - recentConf) > 0.15) {
      this.signals.push({
        type: 'calibration_drift',
        severity: Math.abs(recentAcc - recentConf),
        description: `Model confidence (${(recentConf * 100).toFixed(0)}%) diverges from accuracy (${(recentAcc * 100).toFixed(0)}%)`,
        detected: Date.now(),
      });
    }

    // model degradation
    if (recentAcc < olderAcc - 0.1 && recentAcc < 0.45) {
      this.signals.push({
        type: 'model_degradation',
        severity: olderAcc - recentAcc,
        description: `Accuracy dropped from ${(olderAcc * 100).toFixed(0)}% to ${(recentAcc * 100).toFixed(0)}%`,
        detected: Date.now(),
      });
    }

    // regime shift detection — accuracy drop in a specific regime
    const byRegime = new Map<string, LearningRecord[]>();
    recent.forEach(r => {
      const arr = byRegime.get(r.regime) ?? [];
      arr.push(r);
      byRegime.set(r.regime, arr);
    });
    byRegime.forEach((recs, regime) => {
      const acc = recs.filter(r => r.outcome === 'correct').length / recs.length;
      if (acc < 0.3 && recs.length >= 5) {
        this.signals.push({
          type: 'regime_shift',
          severity: 1 - acc,
          description: `Low accuracy (${(acc * 100).toFixed(0)}%) in regime "${regime}" — possible structural break`,
          detected: Date.now(),
          regime,
        });
      }
    });
  }

  getModelWeights(): ModelWeight[] {
    return Array.from(this.weights.values()).sort((a, b) => b.weight - a.weight);
  }

  getSignals(): EvolutionSignal[] {
    return this.signals.slice(-20);
  }

  getRecords(): LearningRecord[] {
    return this.records;
  }

  getCalibrationCurve(): { bucket: number; predicted: number; actual: number; count: number }[] {
    const buckets: { bucket: number; predicted: number; actual: number; count: number }[] = [];
    for (let i = 0; i < 10; i++) {
      const lo = i / 10;
      const hi = (i + 1) / 10;
      const inBucket = this.records.filter(r =>
        r.predicted.confidence >= lo && r.predicted.confidence < hi
      );
      if (inBucket.length > 0) {
        buckets.push({
          bucket: lo + 0.05,
          predicted: inBucket.reduce((s, r) => s + r.predicted.confidence, 0) / inBucket.length,
          actual: inBucket.filter(r => r.outcome === 'correct').length / inBucket.length,
          count: inBucket.length,
        });
      }
    }
    return buckets;
  }
}

export const learningEngine = new LearningEngine();
