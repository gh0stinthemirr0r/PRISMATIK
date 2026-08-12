//! Realized-volatility surface.
//!
//! This is the honest substitute for an implied-volatility surface when no
//! options chain is available. Instead of strike × expiry × IV it plots
//! lookback × horizon × *realized* volatility: for each estimation window, how
//! volatile has the instrument actually been, and how much did that estimate
//! hold up over the following window?
//!
//! The diagonal structure is the interesting part — when short-lookback vol
//! sits far above long-lookback vol, the instrument is in a volatility
//! expansion, which is a genuinely tradable observation rather than a
//! decorative surface.

use serde::{Deserialize, Serialize};

use crate::{log_returns, variance, Bar};

/// One cell of the surface.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolCell {
    /// Bars used to estimate volatility.
    pub lookback: usize,
    /// Bars ahead the estimate was checked against.
    pub horizon: usize,
    /// Annualized realized volatility over the lookback, as a fraction.
    pub realized_vol: f64,
    /// Annualized realized volatility actually delivered over the horizon.
    pub forward_vol: f64,
    /// forward / realized; above 1 means the estimate understated what came.
    pub ratio: f64,
    /// Historical windows behind this cell.
    pub sample_size: usize,
}

/// Lookback × horizon realized-volatility surface.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolSurface {
    /// All cells, ordered lookback-major.
    pub cells: Vec<VolCell>,
    /// Distinct lookbacks, ascending.
    pub lookbacks: Vec<usize>,
    /// Distinct horizons, ascending.
    pub horizons: Vec<usize>,
    /// Current annualized volatility at the shortest lookback.
    pub spot_vol: f64,
    /// Bars available.
    pub bar_count: usize,
}

/// Annualized volatility of a return slice.
fn annualized(slice: &[f64], periods_per_year: f64) -> f64 {
    if slice.len() < 2 {
        return 0.0;
    }
    variance(slice).sqrt() * periods_per_year.sqrt()
}

/// Build the surface.
///
/// A cell is emitted only when at least one complete lookback+horizon window
/// exists; short series simply produce a smaller surface rather than cells
/// padded out to a fixed grid.
pub fn realized_vol_surface(
    bars: &[Bar],
    lookbacks: &[usize],
    horizons: &[usize],
    periods_per_year: f64,
) -> VolSurface {
    let returns = log_returns(bars);
    let mut cells = Vec::new();

    for &lookback in lookbacks {
        for &horizon in horizons {
            if lookback < 2 || horizon < 2 || returns.len() < lookback + horizon {
                continue;
            }
            let mut realized_sum = 0.0;
            let mut forward_sum = 0.0;
            let mut samples = 0_usize;
            // Step by the horizon so the forward windows do not overlap; an
            // overlapping forward estimate would understate its own variance.
            let mut start = 0_usize;
            while start + lookback + horizon <= returns.len() {
                let back = annualized(&returns[start..start + lookback], periods_per_year);
                let fwd = annualized(
                    &returns[start + lookback..start + lookback + horizon],
                    periods_per_year,
                );
                if back.is_finite() && fwd.is_finite() && back > 0.0 {
                    realized_sum += back;
                    forward_sum += fwd;
                    samples += 1;
                }
                start += horizon;
            }
            if samples == 0 {
                continue;
            }
            let realized_vol = realized_sum / samples as f64;
            let forward_vol = forward_sum / samples as f64;
            cells.push(VolCell {
                lookback,
                horizon,
                realized_vol,
                forward_vol,
                ratio: if realized_vol > 0.0 {
                    forward_vol / realized_vol
                } else {
                    1.0
                },
                sample_size: samples,
            });
        }
    }

    let shortest = lookbacks.iter().copied().min().unwrap_or(21);
    let spot_vol = if returns.len() >= shortest {
        annualized(&returns[returns.len() - shortest..], periods_per_year)
    } else {
        0.0
    };

    let mut sorted_lookbacks: Vec<usize> = cells.iter().map(|c| c.lookback).collect();
    sorted_lookbacks.sort_unstable();
    sorted_lookbacks.dedup();
    let mut sorted_horizons: Vec<usize> = cells.iter().map(|c| c.horizon).collect();
    sorted_horizons.sort_unstable();
    sorted_horizons.dedup();

    VolSurface {
        cells,
        lookbacks: sorted_lookbacks,
        horizons: sorted_horizons,
        spot_vol,
        bar_count: bars.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars_from(closes: &[f64]) -> Vec<Bar> {
        closes
            .iter()
            .enumerate()
            .map(|(i, &c)| Bar {
                t: i as i64 * 86_400_000,
                o: c,
                h: c,
                l: c,
                c,
                v: 1.0,
            })
            .collect()
    }

    fn wobble(i: usize, scale: f64) -> f64 {
        let x = ((i as f64) * 12.9898).sin() * 43_758.545_312;
        (x - x.floor() - 0.5) * scale
    }

    #[test]
    fn a_short_series_produces_an_empty_surface() {
        let bars = bars_from(&(0..10).map(|i| 100.0 + f64::from(i)).collect::<Vec<_>>());
        let surface = realized_vol_surface(&bars, &[21, 63], &[5, 21], 252.0);
        assert!(surface.cells.is_empty());
    }

    #[test]
    fn a_calm_series_has_lower_vol_than_a_violent_one() {
        let calm = bars_from(
            &(0..600)
                .map(|i| 100.0 + wobble(i, 0.05))
                .collect::<Vec<_>>(),
        );
        let wild = bars_from(&(0..600).map(|i| 100.0 + wobble(i, 5.0)).collect::<Vec<_>>());
        let calm_surface = realized_vol_surface(&calm, &[21], &[5], 252.0);
        let wild_surface = realized_vol_surface(&wild, &[21], &[5], 252.0);
        assert!(calm_surface.cells[0].realized_vol < wild_surface.cells[0].realized_vol);
    }

    #[test]
    fn every_cell_reports_a_positive_sample() {
        let bars = bars_from(
            &(0..800)
                .map(|i| 100.0 + wobble(i, 1.5) + 0.01 * i as f64)
                .collect::<Vec<_>>(),
        );
        let surface = realized_vol_surface(&bars, &[21, 63], &[5, 21], 252.0);
        assert!(!surface.cells.is_empty());
        assert!(surface.cells.iter().all(|c| c.sample_size > 0));
        assert!(surface.cells.iter().all(|c| c.realized_vol >= 0.0));
    }

    #[test]
    fn axes_are_deduplicated_and_sorted() {
        let bars = bars_from(&(0..900).map(|i| 100.0 + wobble(i, 1.0)).collect::<Vec<_>>());
        let surface = realized_vol_surface(&bars, &[63, 21], &[21, 5], 252.0);
        assert!(surface.lookbacks.windows(2).all(|w| w[0] < w[1]));
        assert!(surface.horizons.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn spot_vol_tracks_the_recent_tape() {
        // Calm history that turns violent at the end: spot vol should exceed
        // the long-run average from the surface.
        let mut closes: Vec<f64> = (0..600).map(|i| 100.0 + wobble(i, 0.05)).collect();
        closes.extend((0..40).map(|i| 100.0 + wobble(600 + i, 8.0)));
        let bars = bars_from(&closes);
        let surface = realized_vol_surface(&bars, &[21], &[5], 252.0);
        assert!(surface.spot_vol > surface.cells[0].realized_vol);
    }
}
