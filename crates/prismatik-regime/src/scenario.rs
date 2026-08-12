//! Regime-conditional block bootstrap.
//!
//! Forward paths are resampled from the instrument's *own* returns, in blocks,
//! drawing preferentially from bars that were in the same regime as today.
//! Blocks rather than single returns because sampling one bar at a time
//! destroys volatility clustering — the resulting paths look far calmer than
//! any real tape and would understate tail risk.
//!
//! These paths are explicitly a projection of a model, not observed data. What
//! makes them honest is that the model is the instrument's own empirical return
//! distribution, the draw is seeded, and the seed is reported — so any path in
//! the fan can be reproduced and audited.

use serde::{Deserialize, Serialize};

use crate::{log_returns, Bar, Regime, RegimeClassification};

/// One point on a projected path.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioPoint {
    /// Progress through the horizon, 0..1.
    pub t: f64,
    /// Cumulative return from the anchor, as a fraction.
    pub value: f64,
}

/// One projected path.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioPath {
    /// Stable identifier within this fan.
    pub id: String,
    /// Regime the blocks were drawn from.
    pub regime: Regime,
    /// Path points, including the anchor at t = 0.
    pub points: Vec<ScenarioPoint>,
    /// Terminal cumulative return, as a fraction.
    pub terminal: f64,
}

/// A fan of projected paths plus the recent realized path for context.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioPaths {
    /// Projected paths.
    pub paths: Vec<ScenarioPath>,
    /// Recent realized cumulative return, same length as the horizon.
    pub realized: Vec<ScenarioPoint>,
    /// Regime the projection is conditioned on.
    pub regime: Option<Regime>,
    /// Seed used, so the fan can be reproduced exactly.
    pub seed: u64,
    /// Return blocks available to draw from.
    pub block_pool: usize,
    /// Terminal 5th percentile, as a fraction.
    pub p05_terminal: f64,
    /// Terminal 50th percentile, as a fraction.
    pub p50_terminal: f64,
    /// Terminal 95th percentile, as a fraction.
    pub p95_terminal: f64,
}

/// SplitMix64 — small, fast, and fully determined by the seed.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next() % bound as u64) as usize
        }
    }
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let pos = q * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi {
        return sorted[lo];
    }
    let w = pos - lo as f64;
    sorted[lo] * (1.0 - w) + sorted[hi] * w
}

/// Project `path_count` paths `horizon` bars forward.
///
/// Blocks are drawn from bars classified into the current regime; if that
/// regime has too few bars to form even a couple of blocks, the pool falls back
/// to the whole series so the fan still reflects this instrument rather than
/// nothing at all. The fallback is visible in `block_pool`.
pub fn bootstrap_paths(
    bars: &[Bar],
    classification: &RegimeClassification,
    horizon: usize,
    path_count: usize,
    block_size: usize,
    seed: u64,
) -> ScenarioPaths {
    let returns = log_returns(bars);
    let regime = classification.current;
    let block_size = block_size.max(2);

    // Start indices whose whole block sits inside the chosen regime.
    let mut pool: Vec<usize> = Vec::new();
    if let Some(regime) = regime {
        for start in 0..returns.len().saturating_sub(block_size) {
            let in_regime = (start..start + block_size)
                .all(|i| classification.samples.get(i).and_then(|s| s.regime) == Some(regime));
            if in_regime {
                pool.push(start);
            }
        }
    }
    if pool.len() < 2 {
        pool = (0..returns.len().saturating_sub(block_size)).collect();
    }

    let mut rng = SplitMix64(seed);
    let mut paths = Vec::with_capacity(path_count);
    let mut terminals = Vec::with_capacity(path_count);

    if !pool.is_empty() && horizon > 0 {
        for p in 0..path_count {
            let mut points = Vec::with_capacity(horizon + 1);
            points.push(ScenarioPoint { t: 0.0, value: 0.0 });
            let mut cumulative_log = 0.0_f64;
            let mut step = 0_usize;
            while step < horizon {
                let start = pool[rng.below(pool.len())];
                for offset in 0..block_size {
                    if step >= horizon {
                        break;
                    }
                    if let Some(r) = returns.get(start + offset) {
                        cumulative_log += r;
                    }
                    step += 1;
                    points.push(ScenarioPoint {
                        t: step as f64 / horizon as f64,
                        value: cumulative_log.exp() - 1.0,
                    });
                }
            }
            let terminal = cumulative_log.exp() - 1.0;
            terminals.push(terminal);
            paths.push(ScenarioPath {
                id: format!("p{p}"),
                regime: regime.unwrap_or(Regime::CalmMeanRevert),
                points,
                terminal,
            });
        }
    }

    // Recent realized path, rebased to the same anchor for visual comparison.
    let mut realized = Vec::new();
    if returns.len() >= horizon && horizon > 0 {
        let tail = &returns[returns.len() - horizon..];
        let mut cumulative = 0.0_f64;
        realized.push(ScenarioPoint { t: 0.0, value: 0.0 });
        for (i, r) in tail.iter().enumerate() {
            cumulative += r;
            realized.push(ScenarioPoint {
                t: (i + 1) as f64 / horizon as f64,
                value: cumulative.exp() - 1.0,
            });
        }
    }

    terminals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    ScenarioPaths {
        paths,
        realized,
        regime,
        seed,
        block_pool: pool.len(),
        p05_terminal: quantile(&terminals, 0.05),
        p50_terminal: quantile(&terminals, 0.50),
        p95_terminal: quantile(&terminals, 0.95),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{classify, RegimeParams};

    fn wobble(i: usize, scale: f64) -> f64 {
        let x = ((i as f64) * 12.9898).sin() * 43_758.545_312;
        (x - x.floor() - 0.5) * scale
    }

    fn sample_bars() -> Vec<Bar> {
        (0..700)
            .map(|i| {
                let c = 100.0 * (1.0 + 0.0008 * f64::from(i)) + wobble(i as usize, 1.2);
                Bar {
                    t: i64::from(i) * 86_400_000,
                    o: c,
                    h: c,
                    l: c,
                    c,
                    v: 1.0,
                }
            })
            .collect()
    }

    #[test]
    fn the_same_seed_reproduces_the_same_fan() {
        let bars = sample_bars();
        let classification = classify(&bars, &RegimeParams::daily());
        let a = bootstrap_paths(&bars, &classification, 30, 50, 5, 42);
        let b = bootstrap_paths(&bars, &classification, 30, 50, 5, 42);
        assert_eq!(a.paths.len(), b.paths.len());
        for (pa, pb) in a.paths.iter().zip(b.paths.iter()) {
            assert_eq!(pa.terminal.to_bits(), pb.terminal.to_bits());
        }
    }

    #[test]
    fn a_different_seed_gives_a_different_fan() {
        let bars = sample_bars();
        let classification = classify(&bars, &RegimeParams::daily());
        let a = bootstrap_paths(&bars, &classification, 30, 50, 5, 1);
        let b = bootstrap_paths(&bars, &classification, 30, 50, 5, 2);
        let same = a
            .paths
            .iter()
            .zip(b.paths.iter())
            .all(|(x, y)| x.terminal.to_bits() == y.terminal.to_bits());
        assert!(!same, "different seeds produced an identical fan");
    }

    #[test]
    fn every_path_spans_the_full_horizon_and_starts_at_the_anchor() {
        let bars = sample_bars();
        let classification = classify(&bars, &RegimeParams::daily());
        let fan = bootstrap_paths(&bars, &classification, 24, 20, 5, 7);
        for path in &fan.paths {
            assert_eq!(path.points.len(), 25, "expected anchor plus horizon");
            assert_eq!(path.points[0].t, 0.0);
            assert_eq!(path.points[0].value, 0.0);
            assert!((path.points.last().unwrap().t - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn terminal_quantiles_are_ordered() {
        let bars = sample_bars();
        let classification = classify(&bars, &RegimeParams::daily());
        let fan = bootstrap_paths(&bars, &classification, 30, 200, 5, 11);
        assert!(fan.p05_terminal <= fan.p50_terminal);
        assert!(fan.p50_terminal <= fan.p95_terminal);
    }

    #[test]
    fn an_empty_series_yields_no_paths() {
        let classification = classify(&[], &RegimeParams::daily());
        let fan = bootstrap_paths(&[], &classification, 30, 10, 5, 3);
        assert!(fan.paths.is_empty());
        assert!(fan.realized.is_empty());
    }
}
