use std::time::Instant;

#[derive(Debug, Clone, serde::Serialize)]
pub struct EntropyResult {
    pub weights: Vec<f64>,
    pub entropies: Vec<f64>,
    pub diversities: Vec<f64>,
    pub normalized_matrix: Vec<f64>,
    pub duration_ms: f64,
}

pub fn compute_entropy_weights(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
) -> Result<EntropyResult, String> {
    let start = Instant::now();

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

    let valid_indices: Vec<usize> = super::filter_valid_indices(values, n_trials, n_objectives);
    if valid_indices.is_empty() {
        return Err("No valid trials for entropy computation (all NaN)".to_string());
    }
    let m = valid_indices.len();
    let constant_columns: Vec<bool> = (0..n_objectives)
        .map(|j| {
            let first = values[valid_indices[0] * n_objectives + j];
            valid_indices
                .iter()
                .all(|&i| values[i * n_objectives + j] == first)
        })
        .collect();

    // Step: preprocess negative values (min-max normalization per column if needed)
    let processed: Vec<f64> = {
        let mut has_negative = vec![false; n_objectives];
        for &i in &valid_indices {
            for j in 0..n_objectives {
                if values[i * n_objectives + j] < 0.0 {
                    has_negative[j] = true;
                }
            }
        }

        let mut result = vec![0.0; m * n_objectives];
        for j in 0..n_objectives {
            if has_negative[j] {
                let col_vals: Vec<f64> = valid_indices
                    .iter()
                    .map(|&i| values[i * n_objectives + j])
                    .collect();
                let min_v = col_vals.iter().cloned().fold(f64::INFINITY, f64::min);
                let max_v = col_vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let range = max_v - min_v;
                for (row, &i) in valid_indices.iter().enumerate() {
                    result[row * n_objectives + j] = if range > 0.0 {
                        (values[i * n_objectives + j] - min_v) / range
                    } else {
                        // A constant column gets 1.0 for every row. With 0.0, ratio
                        // normalization would give p=0 -> e_j=0 -> d_j=1, letting a
                        // zero-information column win the largest weight (a positive
                        // constant column stays 1.0 either way, so the result is the same).
                        1.0
                    };
                }
            } else {
                for (row, &i) in valid_indices.iter().enumerate() {
                    result[row * n_objectives + j] = values[i * n_objectives + j];
                }
            }
        }
        result
    };

    // Step: proportional normalization p_ij = x_ij / sum_i(x_ij)
    let mut normalized_matrix = vec![0.0; m * n_objectives];
    for j in 0..n_objectives {
        if m > 1 && constant_columns[j] {
            for i in 0..m {
                normalized_matrix[i * n_objectives + j] = 1.0 / m as f64;
            }
            continue;
        }
        let sum_j: f64 = (0..m).map(|i| processed[i * n_objectives + j]).sum();
        if sum_j > 0.0 {
            for i in 0..m {
                normalized_matrix[i * n_objectives + j] = processed[i * n_objectives + j] / sum_j;
            }
        }
    }

    // Step: information entropy e_j = -(1/ln(m)) * sum(p_ij * ln(p_ij))
    let ln_m = (m as f64).ln();
    let mut entropies = vec![0.0; n_objectives];
    for j in 0..n_objectives {
        if m > 1 && constant_columns[j] {
            // Exact unit entropy avoids rounding residuals in the all-constant fallback.
            entropies[j] = 1.0;
        } else if ln_m > 0.0 {
            let sum: f64 = (0..m)
                .map(|i| {
                    let p = normalized_matrix[i * n_objectives + j];
                    if p > 0.0 {
                        p * p.ln()
                    } else {
                        0.0
                    }
                })
                .sum();
            entropies[j] = -sum / ln_m;
        }
        // if ln_m == 0 (m == 1): entropy stays 0.0
    }

    // Step: diversity degree d_j = 1 - e_j
    let diversities: Vec<f64> = entropies.iter().map(|&e| 1.0 - e).collect();

    // Step: weights w_j = d_j / sum(d_k), uniform if sum == 0
    let sum_d: f64 = diversities.iter().sum();
    let weights: Vec<f64> = if sum_d > 0.0 {
        diversities.iter().map(|&d| d / sum_d).collect()
    } else {
        vec![1.0 / n_objectives as f64; n_objectives]
    };

    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    Ok(EntropyResult {
        weights,
        entropies,
        diversities,
        normalized_matrix,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (&actual, &expected) in actual.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
        }
    }

    #[test]
    fn constant_columns_mixed_with_varying_column() {
        for constant in [0.0, 5.0, -3.0, f64::MAX, -f64::MAX] {
            let values = [constant, 1.0, constant, 2.0, constant, 3.0];
            let result = compute_entropy_weights(&values, 3, 2).unwrap();
            assert_close(
                &result.normalized_matrix,
                &[1.0 / 3.0, 1.0 / 6.0, 1.0 / 3.0, 2.0 / 6.0, 1.0 / 3.0, 0.5],
            );
            assert_eq!(result.entropies[0], 1.0);
            assert_eq!(result.diversities[0], 0.0);
            assert_close(&result.weights, &[0.0, 1.0]);
        }
    }

    #[test]
    fn all_constant_columns_use_equal_weight_fallback() {
        for row in [[5.0, 5.0, 5.0], [0.0, 0.0, 0.0], [5.0, -3.0, 0.0]] {
            for m in [2, 3, 7] {
                let result = compute_entropy_weights(&row.repeat(m), m, 3).unwrap();
                assert_close(&result.normalized_matrix, &vec![1.0 / m as f64; m * 3]);
                assert_eq!(result.entropies, vec![1.0; 3]);
                assert_eq!(result.diversities, vec![0.0; 3]);
                assert_close(&result.weights, &[1.0 / 3.0; 3]);
            }
        }
    }

    #[test]
    fn single_valid_trial_preserves_existing_policy() {
        let row = [5.0, -3.0, 0.0];
        let filtered = [f64::NAN, 1.0, 2.0, 5.0, -3.0, 0.0, 1.0, f64::INFINITY, 2.0];
        for values in [row.as_slice(), filtered.as_slice()] {
            let result = compute_entropy_weights(values, values.len() / 3, 3).unwrap();
            assert_eq!(result.normalized_matrix, vec![1.0, 1.0, 0.0]);
            assert_eq!(result.entropies, vec![0.0; 3]);
            assert_eq!(result.diversities, vec![1.0; 3]);
            assert_close(&result.weights, &[1.0 / 3.0; 3]);
        }
    }

    #[test]
    fn constancy_uses_only_retained_finite_rows() {
        let values = [
            0.0,
            1.0,
            9.0,
            f64::NAN,
            0.0,
            2.0,
            8.0,
            f64::INFINITY,
            0.0,
            3.0,
            7.0,
            f64::NEG_INFINITY,
        ];
        let result = compute_entropy_weights(&values, 6, 2).unwrap();
        let retained = compute_entropy_weights(&[0.0, 1.0, 0.0, 2.0, 0.0, 3.0], 3, 2).unwrap();
        assert_eq!(result.normalized_matrix, retained.normalized_matrix);
        assert_eq!(result.entropies, retained.entropies);
        assert_eq!(result.diversities, retained.diversities);
        assert_close(&result.weights, &[0.0, 1.0]);
    }

    #[test]
    fn nonconstant_calculations_are_unchanged() {
        // Positive ratios, negative min-max preprocessing, and nonconstant zeros.
        let values = [1.0, -2.0, 0.0, 2.0, 0.0, 1.0, 3.0, 2.0, 1.0];
        let result = compute_entropy_weights(&values, 3, 3).unwrap();
        let probabilities: [f64; 9] = [
            1.0 / 6.0,
            0.0,
            0.0,
            1.0 / 3.0,
            1.0 / 3.0,
            0.5,
            0.5,
            2.0 / 3.0,
            0.5,
        ];
        let entropies: Vec<f64> = (0..3)
            .map(|j| {
                -(0..3)
                    .map(|i| {
                        let p = probabilities[i * 3 + j];
                        if p > 0.0 {
                            p * p.ln()
                        } else {
                            0.0
                        }
                    })
                    .sum::<f64>()
                    / 3.0_f64.ln()
            })
            .collect();
        let diversities: Vec<f64> = entropies.iter().map(|e| 1.0 - e).collect();
        let sum: f64 = diversities.iter().sum();
        let weights: Vec<f64> = diversities.iter().map(|d| d / sum).collect();
        assert_close(&result.normalized_matrix, &probabilities);
        assert_close(&result.entropies, &entropies);
        assert_close(&result.diversities, &diversities);
        assert_close(&result.weights, &weights);
    }

    #[test]
    fn near_constant_column_is_not_constant() {
        let values = [0.0, 1.0, 1e-15, 2.0, 2e-15, 3.0];
        let result = compute_entropy_weights(&values, 3, 2).unwrap();
        assert_close(
            &result.normalized_matrix,
            &[0.0, 1.0 / 6.0, 1.0 / 3.0, 1.0 / 3.0, 2.0 / 3.0, 0.5],
        );
        assert!(result.entropies[0] < 1.0);
        assert!(result.diversities[0] > 0.0);
        assert!(result.weights[0] > 0.0);
    }

    #[test]
    fn tc_entropy_01_basic_2objectives() {
        let values = [1.0, 4.0, 3.0, 1.0, 2.0, 3.0];
        let result = compute_entropy_weights(&values, 3, 2).unwrap();
        assert_eq!(result.weights.len(), 2);
        let sum: f64 = result.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9, "weights sum = {}", sum);
        for w in &result.weights {
            assert!(*w >= 0.0 && *w <= 1.0, "weight = {}", w);
        }
    }

    #[test]
    fn tc_entropy_02_3objectives() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let result = compute_entropy_weights(&values, 3, 3).unwrap();
        assert_eq!(result.weights.len(), 3);
        let sum: f64 = result.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn tc_entropy_03_high_variance_higher_weight() {
        let values = [5.0, 1.0, 5.0, 2.0, 5.0, 3.0];
        let result = compute_entropy_weights(&values, 3, 2).unwrap();
        assert!(
            result.weights[1] > result.weights[0],
            "obj1 (varied) should have higher weight than obj0 (constant): w0={}, w1={}",
            result.weights[0],
            result.weights[1]
        );
    }

    #[test]
    fn tc_entropy_b01_single_objective() {
        let values = [1.0, 2.0, 3.0];
        let result = compute_entropy_weights(&values, 3, 1).unwrap();
        assert_eq!(result.weights, vec![1.0]);
    }

    #[test]
    fn tc_entropy_b02_single_trial() {
        let values = [1.0, 2.0];
        let result = compute_entropy_weights(&values, 1, 2).unwrap();
        let sum: f64 = result.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9);
        assert!(
            (result.weights[0] - 0.5).abs() < 1e-9,
            "single trial should give uniform weights"
        );
    }

    #[test]
    fn tc_entropy_b03_all_same_values() {
        let values = [5.0, 5.0, 5.0, 5.0, 5.0, 5.0];
        let result = compute_entropy_weights(&values, 3, 2).unwrap();
        assert!((result.weights[0] - 0.5).abs() < 1e-9);
        assert!((result.weights[1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn tc_entropy_04_proportional_normalization() {
        let values = [1.0, 3.0, 3.0, 4.0];
        let result = compute_entropy_weights(&values, 2, 2).unwrap();
        for j in 0..2 {
            let col_sum: f64 = (0..2).map(|i| result.normalized_matrix[i * 2 + j]).sum();
            assert!((col_sum - 1.0).abs() < 1e-9, "col {} sum = {}", j, col_sum);
        }
    }

    #[test]
    fn tc_entropy_05_zero_variance_objective() {
        let values = [5.0, 1.0, 5.0, 2.0, 5.0, 3.0];
        let result = compute_entropy_weights(&values, 3, 2).unwrap();
        assert!(
            (result.entropies[0] - 1.0).abs() < 1e-9,
            "e0 = {}",
            result.entropies[0]
        );
        assert!(
            (result.diversities[0]).abs() < 1e-9,
            "d0 = {}",
            result.diversities[0]
        );
        assert!(
            (result.weights[0]).abs() < 1e-9,
            "w0 = {}",
            result.weights[0]
        );
        assert!(
            (result.weights[1] - 1.0).abs() < 1e-9,
            "w1 = {}",
            result.weights[1]
        );
    }

    #[test]
    fn tc_entropy_06_nan_exclusion() {
        let values: Vec<f64> = vec![1.0, 2.0, f64::NAN, 1.0, 3.0, 4.0];
        let result = compute_entropy_weights(&values, 3, 2).unwrap();
        assert_eq!(result.weights.len(), 2);
        let sum: f64 = result.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn tc_entropy_07_all_nan_error() {
        let values: Vec<f64> = vec![f64::NAN, f64::NAN, f64::NAN, f64::NAN];
        let result = compute_entropy_weights(&values, 2, 2);
        assert!(result.is_err());
    }

    #[test]
    fn tc_entropy_08_negative_values() {
        let values = [-1.0, 2.0, 3.0, -4.0, 0.0, 1.0];
        let result = compute_entropy_weights(&values, 3, 2).unwrap();
        assert_eq!(result.weights.len(), 2);
        let sum: f64 = result.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9, "weights sum = {}", sum);
    }

    #[test]
    fn tc_entropy_09_zero_handling() {
        let values = [0.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let result = compute_entropy_weights(&values, 3, 2).unwrap();
        assert_eq!(result.weights.len(), 2);
        let sum: f64 = result.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9);
        assert!(result.entropies.iter().all(|&e| (0.0..=1.0).contains(&e)));
    }

    #[test]
    fn tc_entropy_10_equal_weights_sum() {
        let values = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
        let result = compute_entropy_weights(&values, 4, 2).unwrap();
        let sum: f64 = result.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9, "weights sum = {}", sum);
    }

    #[test]
    fn tc_entropy_11_negative_constant_column() {
        // obj0 is a negative constant column (no information), obj1 varies.
        // The constant column must get weight ~= 0 and the varying column ~= 1.
        let values = [-3.0, 1.0, -3.0, 2.0, -3.0, 3.0];
        let result = compute_entropy_weights(&values, 3, 2).unwrap();
        assert!(
            result.weights[0].abs() < 1e-9,
            "constant negative column should have weight ~0: w0={}",
            result.weights[0]
        );
        assert!(
            (result.weights[1] - 1.0).abs() < 1e-9,
            "varying column should have weight ~1: w1={}",
            result.weights[1]
        );
    }

    #[test]
    fn tc_entropy_perf_01_50k_trials() {
        let n_trials = 50_000;
        let n_objectives = 4;
        let values: Vec<f64> = (0..n_trials * n_objectives)
            .map(|i| (i as f64 * 0.001).sin() + 1.0)
            .collect();
        let result = compute_entropy_weights(&values, n_trials, n_objectives).unwrap();
        assert_eq!(result.weights.len(), n_objectives);
    }
}
