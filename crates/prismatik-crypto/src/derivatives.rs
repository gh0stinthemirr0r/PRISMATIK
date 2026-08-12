//! Provenance-bearing derivatives state without inferred or fabricated observations.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// One venue derivatives observation in fixed-point units.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivativesObservation {
    /// Venue identifier.
    pub venue: String,
    /// Canonical underlying identifier.
    pub underlying: String,
    /// Perpetual/future price micros.
    pub derivative_price_micros: u64,
    /// Spot reference price micros.
    pub spot_price_micros: u64,
    /// Funding rate in signed parts per billion.
    pub funding_ppb: i64,
    /// Open interest in quote-currency micros.
    pub open_interest_micros: u64,
    /// Long liquidations in quote-currency micros.
    pub long_liquidations_micros: u64,
    /// Short liquidations in quote-currency micros.
    pub short_liquidations_micros: u64,
    /// Observation time.
    pub observed_at: OffsetDateTime,
    /// Evidence record.
    pub evidence_id: String,
}

/// Explainable aggregate derivatives state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivativesState {
    /// Weighted annualized-style basis proxy in parts per million.
    pub basis_ppm: i64,
    /// Open-interest change from first to last observation, signed ppm.
    pub open_interest_change_ppm: i64,
    /// Net liquidation pressure, positive when short liquidations dominate.
    pub liquidation_imbalance_ppm: i64,
    /// Venues represented.
    pub venues: Vec<String>,
    /// Evidence records represented.
    pub evidence_ids: Vec<String>,
}

/// Analyze supplied observations; empty or zero-denominator input returns no state.
pub fn analyze_derivatives(observations: &[DerivativesObservation]) -> Option<DerivativesState> {
    let first = observations.first()?;
    let last = observations.last()?;
    if last.spot_price_micros == 0 || first.open_interest_micros == 0 {
        return None;
    }
    let basis = i128::from(last.derivative_price_micros) - i128::from(last.spot_price_micros);
    let basis_ppm = i64::try_from(basis * 1_000_000 / i128::from(last.spot_price_micros)).ok()?;
    let oi_delta = i128::from(last.open_interest_micros) - i128::from(first.open_interest_micros);
    let open_interest_change_ppm =
        i64::try_from(oi_delta * 1_000_000 / i128::from(first.open_interest_micros)).ok()?;
    let liquidation_total = last
        .long_liquidations_micros
        .saturating_add(last.short_liquidations_micros);
    let liquidation_imbalance_ppm = if liquidation_total == 0 {
        0
    } else {
        i64::try_from(
            (i128::from(last.short_liquidations_micros)
                - i128::from(last.long_liquidations_micros))
                * 1_000_000
                / i128::from(liquidation_total),
        )
        .ok()?
    };
    let mut venues = observations
        .iter()
        .map(|item| item.venue.clone())
        .collect::<Vec<_>>();
    venues.sort();
    venues.dedup();
    Some(DerivativesState {
        basis_ppm,
        open_interest_change_ppm,
        liquidation_imbalance_ppm,
        venues,
        evidence_ids: observations
            .iter()
            .map(|item| item.evidence_id.clone())
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn computes_fixed_point_state() {
        let at = OffsetDateTime::UNIX_EPOCH;
        let rows = [
            DerivativesObservation {
                venue: "v".into(),
                underlying: "btc".into(),
                derivative_price_micros: 101,
                spot_price_micros: 100,
                funding_ppb: 1,
                open_interest_micros: 100,
                long_liquidations_micros: 0,
                short_liquidations_micros: 0,
                observed_at: at,
                evidence_id: "a".into(),
            },
            DerivativesObservation {
                venue: "v".into(),
                underlying: "btc".into(),
                derivative_price_micros: 102,
                spot_price_micros: 100,
                funding_ppb: 2,
                open_interest_micros: 110,
                long_liquidations_micros: 25,
                short_liquidations_micros: 75,
                observed_at: at,
                evidence_id: "b".into(),
            },
        ];
        let state = analyze_derivatives(&rows).unwrap();
        assert_eq!(state.basis_ppm, 20_000);
        assert_eq!(state.liquidation_imbalance_ppm, 500_000);
    }
}
