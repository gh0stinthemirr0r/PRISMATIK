//! Risk measures: exposure, concentration (HHI), Pearson correlation / covariance.

/// Gross exposure: sum of absolute notionals in currency micros.
pub fn gross_exposure_micros(notionals_micros: &[i64]) -> i64 {
    notionals_micros
        .iter()
        .fold(0_i64, |acc, n| acc.saturating_add(n.abs()))
}

/// Herfindahl–Hirschman concentration index in micros of the unit interval.
///
/// Shares are `|x_i| / sum(|x|)`. Returns `sum(share_i^2) * 1_000_000`.
/// Empty or all-zero exposures yield `0`.
pub fn concentration_hhi(exposures_micros: &[i64]) -> u64 {
    let total = gross_exposure_micros(exposures_micros);
    if total == 0 {
        return 0;
    }
    let total_u = total as u128;
    let mut hhi: u128 = 0;
    for x in exposures_micros {
        let share = x.unsigned_abs() as u128 * 1_000_000 / total_u;
        hhi = hhi.saturating_add(share * share);
    }
    // share was in micros → share^2 is in micros^2; scale back to micros.
    (hhi / 1_000_000) as u64
}

/// Sample covariance matrix for aligned return series (`P6-QM-02`).
///
/// Each inner slice is one asset's returns over the same `n` observations.
/// Entry `(i, j)` is
/// `C_ij = 1/(n-1) · Σ_t (r_{i,t} − μ_i)(r_{j,t} − μ_j)`.
///
/// Returns `None` when there are no series, unequal lengths, or `n < 2`.
pub fn sample_covariance_matrix(returns: &[&[f64]]) -> Option<Vec<Vec<f64>>> {
    let (k, n) = aligned_shape(returns)?;
    let means: Vec<f64> = returns
        .iter()
        .map(|series| series.iter().sum::<f64>() / n as f64)
        .collect();
    let denom = (n - 1) as f64;
    let mut cov = vec![vec![0.0_f64; k]; k];
    for i in 0..k {
        for j in i..k {
            let mut s = 0.0_f64;
            for t in 0..n {
                s += (returns[i][t] - means[i]) * (returns[j][t] - means[j]);
            }
            let v = s / denom;
            cov[i][j] = v;
            cov[j][i] = v;
        }
    }
    Some(cov)
}

/// Pairwise Pearson correlation matrix for aligned return series (`P6-QM-02`).
///
/// Built from [`sample_covariance_matrix`]:
/// `ρ_ij = C_ij / √(C_ii · C_jj)`.
/// Diagonal entries are `1.0`. Returns `None` when the covariance matrix is
/// undefined or any series has zero sample variance.
pub fn pearson_correlation_matrix(returns: &[&[f64]]) -> Option<Vec<Vec<f64>>> {
    let cov = sample_covariance_matrix(returns)?;
    let k = cov.len();
    let mut corr = vec![vec![0.0_f64; k]; k];
    for i in 0..k {
        if matches!(
            cov[i][i].partial_cmp(&0.0),
            None | Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ) {
            return None;
        }
        corr[i][i] = 1.0;
        for j in (i + 1)..k {
            if matches!(
                cov[j][j].partial_cmp(&0.0),
                None | Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
            ) {
                return None;
            }
            let denom = (cov[i][i] * cov[j][j]).sqrt();
            let r = cov[i][j] / denom;
            corr[i][j] = r;
            corr[j][i] = r;
        }
    }
    Some(corr)
}

/// Pearson sample correlation of two equal-length return series.
///
/// `ρ = Σ (a−μ_a)(b−μ_b) / √(Σ(a−μ_a)² · Σ(b−μ_b)²)`.
/// Returns `None` when length `< 2`, lengths differ, or either variance is zero.
pub fn pearson_correlation(a: &[f64], b: &[f64]) -> Option<f64> {
    pearson_correlation_matrix(&[a, b]).map(|m| m[0][1])
}

fn aligned_shape(returns: &[&[f64]]) -> Option<(usize, usize)> {
    let k = returns.len();
    if k == 0 {
        return None;
    }
    let n = returns[0].len();
    if n < 2 {
        return None;
    }
    if returns.iter().any(|s| s.len() != n) {
        return None;
    }
    Some((k, n))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    #[test]
    fn exposure_sums_abs() {
        assert_eq!(gross_exposure_micros(&[100, -50, 25]), 175);
    }

    #[test]
    fn hhi_single_name_is_one() {
        assert_eq!(concentration_hhi(&[1_000_000]), 1_000_000);
    }

    #[test]
    fn hhi_equal_two_names() {
        // Two equal shares → 0.5^2 + 0.5^2 = 0.5 → 500_000 micros.
        assert_eq!(concentration_hhi(&[100, 100]), 500_000);
    }

    #[test]
    fn correlation_rejects_empty_and_short() {
        assert_eq!(pearson_correlation_matrix(&[]), None);
        assert_eq!(pearson_correlation_matrix(&[&[1.0]]), None);
        assert_eq!(sample_covariance_matrix(&[&[1.0, 2.0], &[1.0]]), None);
        assert_eq!(pearson_correlation(&[1.0], &[2.0]), None);
    }

    #[test]
    fn perfect_positive_correlation() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [2.0, 4.0, 6.0, 8.0];
        let r = pearson_correlation(&a, &b).expect("defined");
        assert!((r - 1.0).abs() < EPS);
    }

    #[test]
    fn perfect_negative_correlation() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [8.0, 6.0, 4.0, 2.0];
        let r = pearson_correlation(&a, &b).expect("defined");
        assert!((r + 1.0).abs() < EPS);
    }

    #[test]
    fn uncorrelated_orthogonal_series() {
        // Mean-zero orthogonal vectors → Pearson ρ = 0.
        let a = [-1.0, 1.0, -1.0, 1.0];
        let b = [-1.0, -1.0, 1.0, 1.0];
        let r = pearson_correlation(&a, &b).expect("defined");
        assert!(r.abs() < EPS);
    }

    #[test]
    fn zero_variance_is_undefined() {
        let a = [1.0, 1.0, 1.0];
        let b = [1.0, 2.0, 3.0];
        assert_eq!(pearson_correlation(&a, &b), None);
        assert_eq!(pearson_correlation_matrix(&[&a, &b]), None);
    }

    #[test]
    fn pearson_matrix_is_symmetric() {
        let x = [0.1, -0.2, 0.05, 0.3, -0.1];
        let y = [0.05, 0.1, -0.15, 0.2, 0.0];
        let z = [-0.05, 0.25, 0.1, -0.2, 0.15];
        let m = pearson_correlation_matrix(&[&x, &y, &z]).expect("defined");
        for i in 0..3 {
            for j in 0..3 {
                assert!((m[i][j] - m[j][i]).abs() < EPS, "asymmetry at ({i},{j})");
            }
        }
    }

    #[test]
    fn pearson_matrix_diagonal_is_one() {
        let x = [1.0, 2.0, 3.0, 5.0];
        let y = [2.0, 1.0, 4.0, 0.0];
        let m = pearson_correlation_matrix(&[&x, &y]).expect("defined");
        assert!((m[0][0] - 1.0).abs() < EPS);
        assert!((m[1][1] - 1.0).abs() < EPS);
    }

    #[test]
    fn pearson_entries_bounded() {
        let series: [&[f64]; 3] = [
            &[0.02, -0.01, 0.03, -0.02, 0.01, 0.00],
            &[0.01, 0.02, -0.01, 0.00, -0.03, 0.02],
            &[-0.01, 0.00, 0.02, 0.01, -0.02, 0.03],
        ];
        let m = pearson_correlation_matrix(&series).expect("defined");
        for row in &m {
            for &v in row {
                assert!((-1.0 - EPS..=1.0 + EPS).contains(&v), "out of bounds: {v}");
            }
        }
    }

    #[test]
    fn covariance_matches_known_pair() {
        // n=3, a=[1,2,3], b=[2,4,6] → μ_a=2, μ_b=4
        // Σ(a−μ)(b−μ) = (−1)(−2)+(0)(0)+(1)(2)=4; C_ab = 4/2 = 2
        // C_aa = ((−1)²+0+1²)/2 = 1; C_bb = 4
        let a = [1.0, 2.0, 3.0];
        let b = [2.0, 4.0, 6.0];
        let c = sample_covariance_matrix(&[&a, &b]).expect("defined");
        assert!((c[0][0] - 1.0).abs() < EPS);
        assert!((c[1][1] - 4.0).abs() < EPS);
        assert!((c[0][1] - 2.0).abs() < EPS);
        assert!((c[1][0] - 2.0).abs() < EPS);
    }

    #[test]
    fn covariance_is_symmetric() {
        let x = [1.0, 3.0, 2.0, 5.0];
        let y = [4.0, 1.0, 3.0, 2.0];
        let c = sample_covariance_matrix(&[&x, &y]).expect("defined");
        assert!((c[0][1] - c[1][0]).abs() < EPS);
    }
}
