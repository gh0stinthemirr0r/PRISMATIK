//! Public-address-only behavior analytics with explicit non-identity semantics.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Publicly observable address activity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressActivity {
    /// Opaque public address; never mapped to a person here.
    pub address: String,
    /// Market identifier.
    pub market_id: String,
    /// Signed position change in settlement micros.
    pub quantity_delta_micros: i64,
    /// Realized profit/loss micros when observable.
    pub realized_pnl_micros: Option<i64>,
    /// Observation time.
    pub observed_at: OffsetDateTime,
    /// Evidence identifier.
    pub evidence_id: String,
}

/// Explainable public-address profile, never an identity or insider label.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicAddressProfile {
    /// Opaque address.
    pub address: String,
    /// Number of observations.
    pub observation_count: u64,
    /// Distinct markets.
    pub distinct_markets: u64,
    /// Absolute turnover micros.
    pub turnover_micros: u64,
    /// Largest market share of turnover in ppm.
    pub concentration_ppm: u32,
    /// True only when fewer than the caller's minimum observations exist.
    pub insufficient_history: bool,
    /// Evidence included.
    pub evidence_ids: Vec<String>,
}

/// Analyze one opaque address with a caller-defined minimum history threshold.
pub fn analyze_public_address(
    address: &str,
    activities: &[AddressActivity],
    minimum_observations: u64,
) -> Option<PublicAddressProfile> {
    let rows = activities
        .iter()
        .filter(|row| row.address == address)
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return None;
    }
    let mut per_market = std::collections::BTreeMap::<&str, u64>::new();
    for row in &rows {
        *per_market.entry(&row.market_id).or_default() = per_market
            .get(row.market_id.as_str())
            .copied()
            .unwrap_or(0)
            .saturating_add(row.quantity_delta_micros.unsigned_abs());
    }
    let turnover = per_market
        .values()
        .copied()
        .fold(0_u64, u64::saturating_add);
    let largest = per_market.values().copied().max().unwrap_or(0);
    let concentration_ppm = if turnover == 0 {
        0
    } else {
        u32::try_from(u128::from(largest) * 1_000_000 / u128::from(turnover)).unwrap_or(1_000_000)
    };
    Some(PublicAddressProfile {
        address: address.into(),
        observation_count: rows.len() as u64,
        distinct_markets: per_market.len() as u64,
        turnover_micros: turnover,
        concentration_ppm,
        insufficient_history: (rows.len() as u64) < minimum_observations,
        evidence_ids: rows.iter().map(|row| row.evidence_id.clone()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remains_opaque_and_flags_small_samples() {
        let row = AddressActivity {
            address: "0xabc".into(),
            market_id: "m".into(),
            quantity_delta_micros: 10,
            realized_pnl_micros: None,
            observed_at: OffsetDateTime::UNIX_EPOCH,
            evidence_id: "e".into(),
        };
        let profile = analyze_public_address("0xabc", &[row], 5).unwrap();
        assert!(profile.insufficient_history);
        assert_eq!(profile.concentration_ppm, 1_000_000);
    }
}
