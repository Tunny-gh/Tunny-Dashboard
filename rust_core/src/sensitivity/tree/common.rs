use crate::math::rng::SeededRng;
use crate::sensitivity::data::sample_rows;
use std::sync::Arc;

/// Return type of `PreparedData::split`
type SplitData<'a> = (&'a [Vec<f64>], &'a [Vec<f64>], &'a [f64], &'a [f64]);

/// Result of NaN/Inf filtering, downsampling, shuffling, and holdout splitting
pub(crate) struct PreparedData {
    pub x_shuffled: Arc<Vec<Vec<f64>>>,
    pub y_shuffled: Vec<f64>,
    /// x_shuffled[..split_idx] is the training data, [split_idx..] is the eval data
    pub split_idx: usize,
    /// If false, train == eval (too little data)
    pub use_holdout: bool,
}

impl PreparedData {
    /// Returns (x_train, x_eval, y_train, y_eval).
    /// If use_holdout is false, train and eval point to the same slice.
    pub(crate) fn split(&self) -> SplitData<'_> {
        let x = self.x_shuffled.as_slice();
        if self.use_holdout {
            (
                &x[..self.split_idx],
                &x[self.split_idx..],
                &self.y_shuffled[..self.split_idx],
                &self.y_shuffled[self.split_idx..],
            )
        } else {
            (x, x, self.y_shuffled.as_slice(), self.y_shuffled.as_slice())
        }
    }
}

/// Computes the 80/20 holdout split parameters (use_holdout, split_idx).
fn compute_split(n: usize) -> (bool, usize) {
    const MIN_EVAL: usize = 2;
    const MIN_TRAIN: usize = 2;
    let use_holdout = n >= MIN_TRAIN + MIN_EVAL;
    let split_idx = if use_holdout {
        ((n * 4) / 5).max(MIN_TRAIN)
    } else {
        n
    };
    (use_holdout, split_idx)
}

/// Runs NaN/Inf filtering, downsampling, shuffling, and holdout splitting in one pass.
/// Returns `None` if the number of valid rows is fewer than 2.
pub(crate) fn prepare_training_data(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    max_rows: usize,
    data_seed: u64,
    split_seed: u64,
) -> Option<PreparedData> {
    let n = y.len();

    let valid_indices: Vec<usize> = (0..n)
        .filter(|&i| y[i].is_finite() && x_matrix[i].iter().all(|v| v.is_finite()))
        .collect();

    let n_valid = valid_indices.len();
    if n_valid < 2 {
        return None;
    }

    let (x_data, y_data) = if n_valid < n {
        let x_clean: Vec<Vec<f64>> = valid_indices.iter().map(|&i| x_matrix[i].clone()).collect();
        let y_clean: Vec<f64> = valid_indices.iter().map(|&i| y[i]).collect();
        if n_valid > max_rows {
            sample_rows(&x_clean, &y_clean, max_rows, data_seed)
        } else {
            (x_clean, y_clean)
        }
    } else if n > max_rows {
        sample_rows(x_matrix, y, max_rows, data_seed)
    } else {
        (x_matrix.to_vec(), y.to_vec())
    };

    let n = y_data.len();
    let (use_holdout, split_idx) = compute_split(n);

    let mut indices: Vec<usize> = (0..n).collect();
    let mut rng = SeededRng::from_seed(split_seed);
    rng.shuffle(&mut indices);

    let x_shuffled: Vec<Vec<f64>> = indices.iter().map(|&i| x_data[i].clone()).collect();
    let y_shuffled: Vec<f64> = indices.iter().map(|&i| y_data[i]).collect();

    Some(PreparedData {
        x_shuffled: Arc::new(x_shuffled),
        y_shuffled,
        split_idx,
        use_holdout,
    })
}

/// Replaces the specified column in place via a Fisher-Yates shuffle.
/// Callers should save the original column values beforehand if needed and restore them afterward.
pub(crate) fn permute_column_inplace(matrix: &mut [Vec<f64>], feature_idx: usize, seed: u64) {
    let n = matrix.len();
    if n == 0 {
        return;
    }
    let mut rng = SeededRng::from_seed(seed);
    let mut col: Vec<f64> = matrix.iter().map(|row| row[feature_idx]).collect();
    rng.shuffle(&mut col);
    for (row, &v) in matrix.iter_mut().zip(col.iter()) {
        row[feature_idx] = v;
    }
}

/// Restore a single feature column from a saved backup slice.
pub(in crate::sensitivity) fn restore_column(
    matrix: &mut [Vec<f64>],
    feature_idx: usize,
    orig_col: &[f64],
) {
    for (row, &v) in matrix.iter_mut().zip(orig_col.iter()) {
        row[feature_idx] = v;
    }
}

/// Shared entry-point pipeline for the public `compute_*_importances` functions.
///
/// Validates input, runs `prepare_training_data`, then calls `compute_fn`.
/// Returns `(vec![], 0.0)` for invalid input and `(vec![0.0; p], 0.0)` when
/// training fails so callers stay zero-copy identical.
pub(in crate::sensitivity) fn run_importances_pipeline<F>(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    max_rows: usize,
    data_seed: u64,
    split_seed: u64,
    compute_fn: F,
) -> (Vec<f64>, f64)
where
    F: FnOnce(&PreparedData) -> Option<(Vec<f64>, f64)>,
{
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || x_matrix.len() != n {
        return (vec![], 0.0);
    }
    let p = x_matrix[0].len();
    if p == 0 {
        return (vec![], 0.0);
    }
    match prepare_training_data(x_matrix, y, max_rows, data_seed, split_seed) {
        Some(data) => compute_fn(&data).unwrap_or((vec![0.0; p], 0.0)),
        None => (vec![0.0; p], 0.0),
    }
}

/// Normalizes values by their sum. If the sum is <= 0, sets all elements to 0.0.
///
/// NOTE: This is deliberately a different convention from `mcdm::normalize_weights`.
/// That one falls back to a uniform distribution `1/n` in the degenerate case
/// (sum non-finite or <= 0), but the input here is feature importances, and a
/// sum of 0 means "the tree could not make any splits, so no feature has any
/// importance." Substituting a uniform distribution would falsely convey that
/// "all features are equally important," so returning all-zero elements
/// (i.e., no importance) is the semantically correct choice for callers.
pub(crate) fn normalize(values: &mut [f64]) {
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
