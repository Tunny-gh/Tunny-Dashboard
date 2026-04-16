use crate::core::random_forest::{extract_columns, mse_on_dataset, train_rf_on_columns, Lcg};

const RF_TREES: usize = 100;
const RF_MAX_DEPTH: usize = 10;
const RF_MIN_SAMPLES_LEAF: usize = 2;
const RF_SEED: u64 = 42;
const RF_ANOVA_MAX_ROWS: usize = 2_000;

pub fn compute_rf_anova_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> Vec<f64> {
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || x_matrix.len() != n {
        return vec![];
    }

    let p = x_matrix[0].len();
    if p == 0 {
        return vec![];
    }

    if n > RF_ANOVA_MAX_ROWS {
        return vec![0.0; p];
    }

    let all_columns: Vec<usize> = (0..p).collect();
    let rf = match train_rf_on_columns(
        x_matrix,
        y,
        &all_columns,
        RF_TREES,
        RF_MAX_DEPTH,
        RF_MIN_SAMPLES_LEAF,
        RF_SEED,
    ) {
        Some(model) => model,
        None => return vec![0.0; p],
    };

    let baseline_mse = mse_on_dataset(&rf, x_matrix, y)
        .unwrap_or(0.0)
        .max(f64::EPSILON);

    let mut importances = vec![0.0; p];
    for (feature_idx, importance) in importances.iter_mut().enumerate().take(p) {
        let permuted =
            match permute_single_column(x_matrix, feature_idx, RF_SEED + feature_idx as u64) {
                Some(data) => data,
                None => continue,
            };

        let permuted_mse = mse_on_dataset(&rf, &permuted, y).unwrap_or(baseline_mse);
        *importance = (permuted_mse - baseline_mse).max(0.0);
    }

    normalize(&mut importances);
    importances
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

    let column_source = extract_columns(x_matrix, &[feature_idx])?;
    let mut column_values: Vec<f64> = column_source.iter().map(|row| row[0]).collect();

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
