use super::data::get_param_numeric_values;
use super::SobolResult;
use crate::math::rng::SeededRng;
use crate::math::stats::{column_mean_std, value_range};
use rayon::prelude::*;

struct SobolSurrogate {
    param_means: Vec<f64>,
    param_stds: Vec<f64>,
    quad_feat_means: Vec<f64>,
    quad_feat_stds: Vec<f64>,
    betas: Vec<Vec<f64>>,
    intercepts: Vec<f64>,
    r_squared: Vec<f64>, // surrogate fit per objective
}

pub(crate) fn build_quad_features(x_std: &[f64]) -> Vec<f64> {
    let p = x_std.len();
    let n_quad = 2 * p + p * (p - 1) / 2;
    let mut feat = Vec::with_capacity(n_quad);

    feat.extend_from_slice(x_std);

    for &xi in x_std {
        feat.push(xi * xi);
    }

    for i in 0..p {
        for j in (i + 1)..p {
            feat.push(x_std[i] * x_std[j]);
        }
    }

    feat
}

fn build_row_major_matrix(columns: &[Vec<f64>], n_rows: usize) -> Vec<Vec<f64>> {
    if columns.is_empty() || n_rows == 0 {
        return vec![];
    }

    let n_cols = columns.len();
    (0..n_rows)
        .map(|row| (0..n_cols).map(|col| columns[col][row]).collect())
        .collect()
}

fn compute_param_ranges_from_columns(param_columns: &[Vec<f64>]) -> Vec<(f64, f64)> {
    param_columns
        .iter()
        .map(|col| {
            if col.is_empty() {
                return (0.0, 1.0);
            }

            let (min, max) = value_range(col.iter().copied());

            if (max - min).abs() < f64::EPSILON {
                (min - 1.0, max + 1.0)
            } else {
                (min, max)
            }
        })
        .collect()
}

fn build_sobol_surrogate(
    x_matrix: &[Vec<f64>],
    y_matrix: &[Vec<f64>],
    n_params: usize,
    alpha: f64,
) -> Option<SobolSurrogate> {
    let n = x_matrix.len();
    if n < 2 || n_params == 0 {
        return None;
    }

    let mut param_means = vec![0.0f64; n_params];
    let mut param_stds = vec![1.0f64; n_params];

    for j in 0..n_params {
        let vals: Vec<f64> = x_matrix.iter().map(|row| row[j]).collect();
        (param_means[j], param_stds[j]) = column_mean_std(&vals);
    }

    let x_std: Vec<Vec<f64>> = x_matrix
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(j, &v)| (v - param_means[j]) / param_stds[j])
                .collect()
        })
        .collect();

    let quad_feats: Vec<Vec<f64>> = x_std.iter().map(|row| build_quad_features(row)).collect();
    let n_quad = quad_feats[0].len();

    let mut quad_feat_means = vec![0.0f64; n_quad];
    let mut quad_feat_stds = vec![1.0f64; n_quad];

    for j in 0..n_quad {
        let vals: Vec<f64> = quad_feats.iter().map(|row| row[j]).collect();
        (quad_feat_means[j], quad_feat_stds[j]) = column_mean_std(&vals);
    }

    // Standardize the quadratic features exactly once, into a column-major flat
    // array, and hand that to the ridge solver's already-standardized entry
    // point. Previously the features were standardized here AND again inside
    // `compute_ridge_from_vecs`, a redundant double standardization.
    let mut x_quad_std_cols = vec![0.0f64; n * n_quad];
    for (j, (&m, &s)) in quad_feat_means
        .iter()
        .zip(quad_feat_stds.iter())
        .enumerate()
    {
        for (i, row) in quad_feats.iter().enumerate() {
            x_quad_std_cols[j * n + i] = (row[j] - m) / s;
        }
    }

    let triplets: Vec<(Vec<f64>, f64, f64)> = y_matrix
        .par_iter()
        .map(|y| {
            let y_mean = y.iter().sum::<f64>() / n as f64;
            let y_centered: Vec<f64> = y.iter().map(|&v| v - y_mean).collect();
            let ridge_res = super::ridge::compute_ridge_from_standardized_columns(
                &x_quad_std_cols,
                n,
                &y_centered,
                alpha,
            );
            (ridge_res.beta, y_mean, ridge_res.r_squared)
        })
        .collect();

    let mut betas = Vec::with_capacity(triplets.len());
    let mut intercepts = Vec::with_capacity(triplets.len());
    let mut r_squared = Vec::with_capacity(triplets.len());

    for (beta, intercept, r2) in triplets {
        betas.push(beta);
        intercepts.push(intercept);
        r_squared.push(r2);
    }

    Some(SobolSurrogate {
        param_means,
        param_stds,
        quad_feat_means,
        quad_feat_stds,
        betas,
        intercepts,
        r_squared,
    })
}

fn surrogate_eval(surrogate: &SobolSurrogate, x_raw: &[f64], obj_idx: usize) -> f64 {
    let x_std: Vec<f64> = x_raw
        .iter()
        .enumerate()
        .map(|(j, &v)| (v - surrogate.param_means[j]) / surrogate.param_stds[j])
        .collect();

    let quad = build_quad_features(&x_std);

    let beta = &surrogate.betas[obj_idx];
    let dot: f64 = beta
        .iter()
        .zip(quad.iter())
        .zip(surrogate.quad_feat_means.iter())
        .zip(surrogate.quad_feat_stds.iter())
        .map(|(((&b, &q), &m), &s)| b * (q - m) / s)
        .sum();
    dot + surrogate.intercepts[obj_idx]
}

pub(crate) fn compute_sobol_index_pair(fa_k: &[f64], fb_k: &[f64], fab_k: &[f64]) -> (f64, f64) {
    let n_f = fa_k.len() as f64;
    let mean_fa = fa_k.iter().sum::<f64>() / n_f;
    let var_y = fa_k.iter().map(|&v| (v - mean_fa).powi(2)).sum::<f64>() / n_f;

    if var_y < f64::EPSILON {
        return (0.0, 0.0);
    }

    let s_i: f64 = fb_k
        .iter()
        .zip(fab_k.iter())
        .zip(fa_k.iter())
        .map(|((&fb_j, &fab_j), &fa_j)| fb_j * (fab_j - fa_j))
        .sum::<f64>()
        / (n_f * var_y);

    let st_i: f64 = fa_k
        .iter()
        .zip(fab_k.iter())
        .map(|(&fa_j, &fab_j)| (fa_j - fab_j).powi(2))
        .sum::<f64>()
        / (2.0 * n_f * var_y);

    // With a finite-sample estimate, ST_i < S_i can occur.
    // Enforce the theoretically guaranteed ST_i >= S_i before clamping the output to [0,1].
    let st_i = st_i.max(s_i);
    (s_i.clamp(0.0, 1.0), st_i.clamp(0.0, 1.0))
}

pub fn compute_sobol(n_samples: usize) -> Option<SobolResult> {
    crate::dataframe::with_active_df(|df| compute_sobol_from_df(df, n_samples)).flatten()
}

pub fn compute_sobol_from_df(
    df: &crate::dataframe::DataFrame,
    n_samples: usize,
) -> Option<SobolResult> {
    let param_names = df.param_col_names().to_vec();
    let objective_names = df.objective_col_names().to_vec();
    let n = df.row_count();
    let n_params = param_names.len();
    let n_objectives = objective_names.len();

    if n < 2 || n_params == 0 || n_objectives == 0 || n_samples == 0 {
        return None;
    }

    let raw_param_columns: Vec<Vec<f64>> = param_names
        .iter()
        .map(|name| get_param_numeric_values(df, name, n).unwrap_or_else(|| vec![0.0; n]))
        .collect();
    let objective_columns: Vec<Vec<f64>> = objective_names
        .iter()
        .map(|name| {
            df.get_numeric_column(name)
                .map(|col| col.iter().take(n).copied().collect())
                .unwrap_or_else(|| vec![0.0; n])
        })
        .collect();

    // As with the other sensitivity metrics (`prepare_training_data`), the surrogate model is
    // built only from rows where the parameters and all objective values are finite. If a NaN
    // objective from a failed/pruned trial propagated into the ridge regression, all metrics
    // would come out as a "silently broken" NaN result.
    let valid_indices: Vec<usize> = (0..n)
        .filter(|&i| {
            raw_param_columns.iter().all(|col| col[i].is_finite())
                && objective_columns.iter().all(|col| col[i].is_finite())
        })
        .collect();
    if valid_indices.len() < 2 {
        return None;
    }
    let n = valid_indices.len();
    let param_columns: Vec<Vec<f64>> = raw_param_columns
        .iter()
        .map(|col| valid_indices.iter().map(|&i| col[i]).collect())
        .collect();
    let y_matrix: Vec<Vec<f64>> = objective_columns
        .iter()
        .map(|col| valid_indices.iter().map(|&i| col[i]).collect())
        .collect();
    let x_matrix = build_row_major_matrix(&param_columns, n);

    let surrogate = build_sobol_surrogate(&x_matrix, &y_matrix, n_params, 1.0)?;

    let mut rng = SeededRng::from_seed(0xDEAD_BEEF_1234_5678);

    let param_ranges = compute_param_ranges_from_columns(&param_columns);

    let mat_a: Vec<Vec<f64>> = (0..n_samples)
        .map(|_| {
            param_ranges
                .iter()
                .map(|(lo, hi)| lo + rng.next_f64() * (hi - lo))
                .collect()
        })
        .collect();

    let mat_b: Vec<Vec<f64>> = (0..n_samples)
        .map(|_| {
            param_ranges
                .iter()
                .map(|(lo, hi)| lo + rng.next_f64() * (hi - lo))
                .collect()
        })
        .collect();

    let f_a: Vec<Vec<f64>> = (0..n_objectives)
        .into_par_iter()
        .map(|k| {
            mat_a
                .iter()
                .map(|row| surrogate_eval(&surrogate, row, k))
                .collect()
        })
        .collect();

    let f_b: Vec<Vec<f64>> = (0..n_objectives)
        .into_par_iter()
        .map(|k| {
            mat_b
                .iter()
                .map(|row| surrogate_eval(&surrogate, row, k))
                .collect()
        })
        .collect();

    let sobol_pairs: Vec<(Vec<f64>, Vec<f64>)> = (0..n_params)
        .into_par_iter()
        .map(|pi| {
            let ab_pi: Vec<Vec<f64>> = mat_a
                .iter()
                .zip(mat_b.iter())
                .map(|(a_row, b_row)| {
                    let mut row = a_row.clone();
                    row[pi] = b_row[pi];
                    row
                })
                .collect();

            let f_ab_pi: Vec<Vec<f64>> = (0..n_objectives)
                .map(|k| {
                    ab_pi
                        .iter()
                        .map(|row| surrogate_eval(&surrogate, row, k))
                        .collect()
                })
                .collect();

            let mut fo_vec = Vec::with_capacity(n_objectives);
            let mut te_vec = Vec::with_capacity(n_objectives);
            for k in 0..n_objectives {
                let (fo, te) = compute_sobol_index_pair(&f_a[k], &f_b[k], &f_ab_pi[k]);
                fo_vec.push(fo);
                te_vec.push(te);
            }
            (fo_vec, te_vec)
        })
        .collect();

    let (first_order, total_effect): (Vec<Vec<f64>>, Vec<Vec<f64>>) =
        sobol_pairs.into_iter().unzip();

    Some(SobolResult {
        param_names,
        objective_names,
        first_order,
        total_effect,
        r_squared: surrogate.r_squared,
        n_samples,
    })
}
