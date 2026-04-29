use super::data::sample_rows;
use crate::core::lgbm::{lgbm_mse, train_lgbm_rf, LgbmRfConfig};
use crate::core::random_forest::Lcg;

const RF_TREES: usize = 100;
const RF_MAX_DEPTH: usize = 10;
const RF_MIN_SAMPLES_LEAF: usize = 2;
const RF_SEED: u64 = 42;
const RF_ANOVA_MAX_ROWS: usize = 2_000;

/// Returns (importances, r_squared).
pub fn compute_rf_anova_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || x_matrix.len() != n {
        return (vec![], 0.0);
    }

    let p = x_matrix[0].len();
    if p == 0 {
        return (vec![], 0.0);
    }

    // Filter out rows where y or any x value is non-finite (NaN/Inf from failed trials)
    let valid_indices: Vec<usize> = (0..n)
        .filter(|&i| y[i].is_finite() && x_matrix[i].iter().all(|v| v.is_finite()))
        .collect();

    let n_valid = valid_indices.len();
    if n_valid < 2 {
        return (vec![0.0; p], 0.0);
    }

    // Filter and/or downsample, avoiding a redundant full clone when all rows are valid
    let (x_data, y_data) = if n_valid < n {
        let x_clean: Vec<Vec<f64>> = valid_indices.iter().map(|&i| x_matrix[i].clone()).collect();
        let y_clean: Vec<f64> = valid_indices.iter().map(|&i| y[i]).collect();
        if n_valid > RF_ANOVA_MAX_ROWS {
            sample_rows(&x_clean, &y_clean, RF_ANOVA_MAX_ROWS, RF_SEED)
        } else {
            (x_clean, y_clean)
        }
    } else if n > RF_ANOVA_MAX_ROWS {
        sample_rows(x_matrix, y, RF_ANOVA_MAX_ROWS, RF_SEED)
    } else {
        (x_matrix.to_vec(), y.to_vec())
    };

    let n = y_data.len();
    let x_matrix = &x_data;
    let y = &y_data;

    // 80/20 holdout split: train RF on train set, evaluate permutation importance on
    // held-out eval set. This prevents in-sample over-fitting from collapsing all
    // importances to zero (permuted MSE ≈ baseline MSE on memorised training data).
    // Shuffle first so train/eval sets cover the same input distribution.
    const MIN_EVAL: usize = 2;
    const MIN_TRAIN: usize = 2;
    let use_holdout = n >= MIN_TRAIN + MIN_EVAL;
    let split_idx = if use_holdout {
        ((n * 4) / 5).max(MIN_TRAIN)
    } else {
        n
    };

    let mut shuffle_idx: Vec<usize> = (0..n).collect();
    let mut rng_split = Lcg::new(RF_SEED.wrapping_add(1));
    for i in (1..n).rev() {
        let j = rng_split.next_usize(i + 1);
        shuffle_idx.swap(i, j);
    }
    let x_shuffled: Vec<Vec<f64>> = shuffle_idx.iter().map(|&i| x_matrix[i].clone()).collect();
    let y_shuffled: Vec<f64> = shuffle_idx.iter().map(|&i| y[i]).collect();

    let (x_train, x_eval, y_train, y_eval) = if use_holdout {
        (
            &x_shuffled[..split_idx],
            &x_shuffled[split_idx..],
            &y_shuffled[..split_idx],
            &y_shuffled[split_idx..],
        )
    } else {
        (
            x_shuffled.as_slice(),
            x_shuffled.as_slice(),
            y_shuffled.as_slice(),
            y_shuffled.as_slice(),
        )
    };

    let config = LgbmRfConfig {
        num_iterations: RF_TREES,
        max_depth: RF_MAX_DEPTH as i32,
        min_data_in_leaf: RF_MIN_SAMPLES_LEAF as i32,
        seed: RF_SEED as i32,
        ..Default::default()
    };
    let booster = match train_lgbm_rf(x_train, y_train, &config) {
        Some(b) => b,
        None => return (vec![0.0; p], 0.0),
    };

    let baseline_mse = lgbm_mse(&booster, x_eval, y_eval)
        .unwrap_or(0.0)
        .max(f64::EPSILON);

    let n_eval = y_eval.len();
    let y_mean = y_eval.iter().sum::<f64>() / n_eval as f64;
    let ss_tot: f64 = y_eval.iter().map(|&v| (v - y_mean).powi(2)).sum();
    let r_squared = if ss_tot < f64::EPSILON {
        0.0
    } else {
        (1.0 - baseline_mse * n_eval as f64 / ss_tot).max(0.0)
    };

    let mut importances = vec![0.0; p];
    for (feature_idx, importance) in importances.iter_mut().enumerate().take(p) {
        let permuted =
            match permute_single_column(x_eval, feature_idx, RF_SEED + feature_idx as u64) {
                Some(data) => data,
                None => continue,
            };

        let permuted_mse = lgbm_mse(&booster, &permuted, y_eval).unwrap_or(baseline_mse);
        *importance = (permuted_mse - baseline_mse).max(0.0);
    }

    normalize(&mut importances);
    (importances, r_squared)
}

fn permute_single_column(
    x_matrix: &[Vec<f64>],
    feature_idx: usize,
    seed: u64,
) -> Option<Vec<Vec<f64>>> {
    let n = x_matrix.len();
    if n == 0 {
        return None;
    }

    let mut column_values: Vec<f64> = x_matrix.iter().map(|row| row[feature_idx]).collect();

    let mut rng = Lcg::new(seed);
    for i in (1..n).rev() {
        let j = rng.next_usize(i + 1);
        column_values.swap(i, j);
    }

    let mut permuted = x_matrix.to_vec();
    for (i, row) in permuted.iter_mut().enumerate() {
        row[feature_idx] = column_values[i];
    }
    Some(permuted)
}

fn normalize(values: &mut [f64]) {
    let sum = values.iter().sum::<f64>();
    if sum < f64::EPSILON {
        for v in values.iter_mut() {
            *v = 0.0;
        }
        return;
    }
    for v in values.iter_mut() {
        *v /= sum;
    }
}
