use super::data::sample_rows;
use crate::core::lgbm::{lgbm_mse, mse_to_r_squared, train_lgbm_rf, LgbmRfConfig};
use crate::core::random_forest::Lcg;

const N_REPEATS: usize = 5;
const PFI_MAX_ROWS: usize = 2_000;
const PFI_SEED_BASE: u64 = 42;
const PFI_SPLIT_SEED: u64 = 43;
const PFI_TREES: usize = 100;
const PFI_MAX_DEPTH: i32 = 10;
const PFI_MIN_DATA_LEAF: i32 = 2;

/// Permutation Feature Importance via LightGBM Random Forest (n_repeats=5).
/// Returns (importances_normalized, r_squared).
/// importances.sum() ≈ 1.0 when valid data exists.
pub fn compute_permutation_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
    let n = y.len();
    if n == 0 || x_matrix.is_empty() || x_matrix.len() != n {
        return (vec![], 0.0);
    }

    let p = x_matrix[0].len();
    if p == 0 {
        return (vec![], 0.0);
    }

    let valid_indices: Vec<usize> = (0..n)
        .filter(|&i| y[i].is_finite() && x_matrix[i].iter().all(|v| v.is_finite()))
        .collect();

    let n_valid = valid_indices.len();
    if n_valid < 2 {
        return (vec![0.0; p], 0.0);
    }

    let (x_data, y_data) = if n_valid < n {
        let x_clean: Vec<Vec<f64>> = valid_indices.iter().map(|&i| x_matrix[i].clone()).collect();
        let y_clean: Vec<f64> = valid_indices.iter().map(|&i| y[i]).collect();
        if n_valid > PFI_MAX_ROWS {
            sample_rows(&x_clean, &y_clean, PFI_MAX_ROWS, PFI_SEED_BASE)
        } else {
            (x_clean, y_clean)
        }
    } else if n > PFI_MAX_ROWS {
        sample_rows(x_matrix, y, PFI_MAX_ROWS, PFI_SEED_BASE)
    } else {
        (x_matrix.to_vec(), y.to_vec())
    };

    let n = y_data.len();

    const MIN_EVAL: usize = 2;
    const MIN_TRAIN: usize = 2;
    let use_holdout = n >= MIN_TRAIN + MIN_EVAL;
    let split_idx = if use_holdout {
        ((n * 4) / 5).max(MIN_TRAIN)
    } else {
        n
    };

    let mut shuffle_idx: Vec<usize> = (0..n).collect();
    let mut rng_split = Lcg::new(PFI_SPLIT_SEED);
    for i in (1..n).rev() {
        let j = rng_split.next_usize(i + 1);
        shuffle_idx.swap(i, j);
    }

    let x_shuffled: Vec<Vec<f64>> = shuffle_idx.iter().map(|&i| x_data[i].clone()).collect();
    let y_shuffled: Vec<f64> = shuffle_idx.iter().map(|&i| y_data[i]).collect();

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
        num_iterations: PFI_TREES,
        max_depth: PFI_MAX_DEPTH,
        min_data_in_leaf: PFI_MIN_DATA_LEAF,
        seed: PFI_SEED_BASE as i32,
        ..Default::default()
    };

    let booster = match train_lgbm_rf(x_train, y_train, &config) {
        Some(b) => b,
        None => return (vec![0.0; p], 0.0),
    };

    let baseline_mse = lgbm_mse(&booster, x_eval, y_eval)
        .unwrap_or(0.0)
        .max(f64::EPSILON);

    let mut importances = vec![0.0f64; p];
    for (feature_idx, importance) in importances.iter_mut().enumerate() {
        let mut delta_sum = 0.0f64;
        for repeat_idx in 0..N_REPEATS {
            let seed =
                PFI_SEED_BASE + (feature_idx as u64) * (N_REPEATS as u64) + (repeat_idx as u64);
            let permuted = match permute_single_column(x_eval, feature_idx, seed) {
                Some(p) => p,
                None => continue,
            };
            let permuted_mse = lgbm_mse(&booster, &permuted, y_eval).unwrap_or(baseline_mse);
            delta_sum += (permuted_mse - baseline_mse).max(0.0);
        }
        *importance = delta_sum / N_REPEATS as f64;
    }

    normalize(&mut importances);

    let r_squared = mse_to_r_squared(baseline_mse, y_eval);
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
    let mut column: Vec<f64> = x_matrix.iter().map(|row| row[feature_idx]).collect();
    let mut rng = Lcg::new(seed);
    for i in (1..n).rev() {
        let j = rng.next_usize(i + 1);
        column.swap(i, j);
    }
    let mut permuted = x_matrix.to_vec();
    for (i, row) in permuted.iter_mut().enumerate() {
        row[feature_idx] = column[i];
    }
    Some(permuted)
}

fn normalize(values: &mut [f64]) {
    let sum: f64 = values.iter().sum();
    if sum < f64::EPSILON {
        values.iter_mut().for_each(|v| *v = 0.0);
        return;
    }
    values.iter_mut().for_each(|v| *v /= sum);
}
