pub mod entropy;
pub mod promethee;
pub mod topsis;
pub mod vikor;

/// Normalize weights so they sum to 1.
///
/// Rejects any negative entry before checking the sum. Empty input succeeds with
/// an empty vec. A zero or nonfinite sum falls back to uniform weights.
pub fn normalize_weights(weights: &[f64]) -> Result<Vec<f64>, String> {
    if weights.iter().any(|&w| w < 0.0) {
        return Err("Criterion weights must be nonnegative".to_string());
    }
    if weights.is_empty() {
        return Ok(vec![]);
    }
    let sum: f64 = weights.iter().sum();
    if !sum.is_finite() || sum <= 0.0 {
        let n = weights.len() as f64;
        Ok(vec![1.0 / n; weights.len()])
    } else {
        Ok(weights.iter().map(|&w| w / sum).collect())
    }
}

/// Validate common MCDM input dimensions.
pub(crate) fn validate_inputs(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    weights: &[f64],
    is_minimize: &[bool],
) -> Result<(), String> {
    if n_trials == 0 {
        return Err("n_trials must be >= 1".to_string());
    }
    if n_objectives == 0 {
        return Err("n_objectives must be >= 1".to_string());
    }
    if values.len() != n_trials * n_objectives {
        return Err(format!(
            "values length mismatch: expected {}, got {}",
            n_trials * n_objectives,
            values.len()
        ));
    }
    if weights.len() != n_objectives {
        return Err(format!(
            "weights length mismatch: expected {}, got {}",
            n_objectives,
            weights.len()
        ));
    }
    if is_minimize.len() != n_objectives {
        return Err(format!(
            "is_minimize length mismatch: expected {}, got {}",
            n_objectives,
            is_minimize.len()
        ));
    }
    Ok(())
}

/// Return indices of trials whose objectives are all finite (excludes NaN and ±Inf).
pub(crate) fn filter_valid_indices(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
) -> Vec<usize> {
    (0..n_trials)
        .filter(|&i| (0..n_objectives).all(|j| values[i * n_objectives + j].is_finite()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_weights_empty() {
        assert!(normalize_weights(&[]).unwrap().is_empty());
    }

    #[test]
    fn normalize_weights_equal() {
        let result = normalize_weights(&[0.5, 0.5]).unwrap();
        assert!((result[0] - 0.5).abs() < 1e-9);
        assert!((result[1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn normalize_weights_divides_by_sum() {
        let result = normalize_weights(&[1.0, 3.0]).unwrap();
        assert!((result[0] - 0.25).abs() < 1e-9);
        assert!((result[1] - 0.75).abs() < 1e-9);
    }

    #[test]
    fn normalize_weights_zero_sum_falls_back_to_uniform() {
        assert_eq!(normalize_weights(&[0.0, 0.0]).unwrap(), vec![0.5, 0.5]);
    }

    #[test]
    fn normalize_weights_rejects_negatives_before_sum_fallback() {
        for weights in [
            [2.0, -1.0],
            [-1.0, -1.0],
            [f64::NEG_INFINITY, 1.0],
            [f64::NAN, -1.0],
            [f64::NAN, f64::NEG_INFINITY],
        ] {
            assert_eq!(
                normalize_weights(&weights).unwrap_err(),
                "Criterion weights must be nonnegative"
            );
        }
    }

    #[test]
    fn normalize_weights_nan_falls_back_to_uniform() {
        for weights in [
            [f64::NAN, 1.0],
            [f64::INFINITY, 1.0],
            [f64::MAX, f64::MAX],
            [-0.0, 0.0],
        ] {
            assert_eq!(normalize_weights(&weights).unwrap(), vec![0.5, 0.5]);
        }
        let weights = normalize_weights(&[-0.0, 2.0]).unwrap();
        assert!(weights[0].is_sign_negative());
        assert_eq!(weights, vec![-0.0, 1.0]);
    }

    #[test]
    fn rankings_reject_negative_weights_even_without_valid_rows() {
        for weights in [
            [2.0, -1.0],
            [-1.0, -1.0],
            [f64::NEG_INFINITY, 1.0],
            [f64::NAN, -1.0],
            [f64::NAN, f64::NEG_INFINITY],
        ] {
            let expected = normalize_weights(&weights).unwrap_err();
            for values in [[1.0, 2.0, 3.0, 4.0], [f64::NAN; 4]] {
                assert_eq!(
                    topsis::compute_topsis(&values, 2, 2, &weights, &[true, false]).unwrap_err(),
                    expected
                );
                assert_eq!(
                    vikor::compute_vikor(&values, 2, 2, &weights, &[true, false], 0.5).unwrap_err(),
                    expected
                );
                assert_eq!(
                    promethee::compute_promethee(&values, 2, 2, &weights, &[true, false])
                        .unwrap_err(),
                    expected
                );
            }
        }
    }

    #[test]
    fn rankings_preserve_nonnegative_normalization_and_bounds() {
        // Includes zero-valued criteria and mixed objective directions.
        let values = [0.0, 4.0, 2.0, 0.0, 3.0, 2.0];
        let directions = [true, false];
        for weights in [
            [0.0, 0.0],
            [2.0, 6.0],
            [0.0, 2.0],
            [-0.0, 2.0],
            [f64::NAN, 1.0],
            [f64::INFINITY, 1.0],
            [f64::MAX, f64::MAX],
        ] {
            let normalized = normalize_weights(&weights).unwrap();
            let t = topsis::compute_topsis(&values, 3, 2, &weights, &directions).unwrap();
            let tn = topsis::compute_topsis(&values, 3, 2, &normalized, &directions).unwrap();
            assert_eq!(t.scores, tn.scores);
            assert_eq!(t.ranked_indices, tn.ranked_indices);
            assert_eq!(t.positive_ideal, tn.positive_ideal);
            assert_eq!(t.negative_ideal, tn.negative_ideal);
            assert!(t.scores.iter().all(|s| (0.0..=1.0).contains(s)));

            let v = vikor::compute_vikor(&values, 3, 2, &weights, &directions, 0.5).unwrap();
            let vn = vikor::compute_vikor(&values, 3, 2, &normalized, &directions, 0.5).unwrap();
            assert_eq!(v.s_values, vn.s_values);
            assert_eq!(v.r_values, vn.r_values);
            assert_eq!(v.q_values, vn.q_values);
            assert_eq!(v.display_scores, vn.display_scores);
            assert_eq!(v.ranked_indices, vn.ranked_indices);
            assert_eq!(v.compromise_indices, vn.compromise_indices);
            assert!(v
                .s_values
                .iter()
                .chain(&v.r_values)
                .chain(&v.q_values)
                .chain(&v.display_scores)
                .all(|s| (0.0..=1.0).contains(s)));

            let p = promethee::compute_promethee(&values, 3, 2, &weights, &directions).unwrap();
            let pn = promethee::compute_promethee(&values, 3, 2, &normalized, &directions).unwrap();
            assert_eq!(p.phi_plus, pn.phi_plus);
            assert_eq!(p.phi_minus, pn.phi_minus);
            assert_eq!(p.phi_net, pn.phi_net);
            assert_eq!(p.ranked_indices_i, pn.ranked_indices_i);
            assert_eq!(p.ranked_indices_ii, pn.ranked_indices_ii);
            assert_eq!(p.incomparable_counts, pn.incomparable_counts);
            assert!(p
                .phi_plus
                .iter()
                .chain(&p.phi_minus)
                .all(|s| (0.0..=1.0).contains(s)));
            assert!(p.phi_net.iter().all(|s| (-1.0..=1.0).contains(s)));
        }
    }
}
