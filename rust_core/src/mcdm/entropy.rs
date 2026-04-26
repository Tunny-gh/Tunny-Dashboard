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
                        0.0
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
        if ln_m > 0.0 {
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
        assert!(result.entropies.iter().all(|&e| e >= 0.0 && e <= 1.0));
    }

    #[test]
    fn tc_entropy_10_equal_weights_sum() {
        let values = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
        let result = compute_entropy_weights(&values, 4, 2).unwrap();
        let sum: f64 = result.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9, "weights sum = {}", sum);
    }

    #[test]
    fn tc_entropy_perf_01_50k_trials() {
        let n_trials = 50_000;
        let n_objectives = 4;
        let values: Vec<f64> = (0..n_trials * n_objectives)
            .map(|i| (i as f64 * 0.001).sin() + 1.0)
            .collect();
        let start = std::time::Instant::now();
        let result = compute_entropy_weights(&values, n_trials, n_objectives).unwrap();
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        assert!(elapsed < 100.0, "took {}ms, expected < 100ms", elapsed);
        assert_eq!(result.weights.len(), n_objectives);
    }
}
