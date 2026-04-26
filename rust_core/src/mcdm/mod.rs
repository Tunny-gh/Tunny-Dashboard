pub mod entropy;
pub mod topsis;
pub mod vikor;

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

/// Return indices of trials that have no NaN objectives.
pub(crate) fn filter_valid_indices(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
) -> Vec<usize> {
    (0..n_trials)
        .filter(|&i| !(0..n_objectives).any(|j| values[i * n_objectives + j].is_nan()))
        .collect()
}
