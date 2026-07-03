pub mod entropy;
pub mod promethee;
pub mod topsis;
pub mod vikor;

/// Normalize weights so they sum to 1.
///
/// Empty input returns an empty vec. If the sum is not finite/positive (e.g. all
/// zero, NaN, or negative), falls back to uniform weights instead of dividing by
/// a degenerate sum.
pub fn normalize_weights(weights: &[f64]) -> Vec<f64> {
    if weights.is_empty() {
        return vec![];
    }
    let sum: f64 = weights.iter().sum();
    if !sum.is_finite() || sum <= 0.0 {
        let n = weights.len() as f64;
        vec![1.0 / n; weights.len()]
    } else {
        weights.iter().map(|&w| w / sum).collect()
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
        assert!(normalize_weights(&[]).is_empty());
    }

    #[test]
    fn normalize_weights_equal() {
        let result = normalize_weights(&[0.5, 0.5]);
        assert!((result[0] - 0.5).abs() < 1e-9);
        assert!((result[1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn normalize_weights_divides_by_sum() {
        let result = normalize_weights(&[1.0, 3.0]);
        assert!((result[0] - 0.25).abs() < 1e-9);
        assert!((result[1] - 0.75).abs() < 1e-9);
    }

    #[test]
    fn normalize_weights_zero_sum_falls_back_to_uniform() {
        assert_eq!(normalize_weights(&[0.0, 0.0]), vec![0.5, 0.5]);
    }

    #[test]
    fn normalize_weights_negative_sum_falls_back_to_uniform() {
        assert_eq!(normalize_weights(&[-1.0, -1.0]), vec![0.5, 0.5]);
    }

    #[test]
    fn normalize_weights_nan_falls_back_to_uniform() {
        assert_eq!(normalize_weights(&[f64::NAN, 1.0]), vec![0.5, 0.5]);
    }
}
