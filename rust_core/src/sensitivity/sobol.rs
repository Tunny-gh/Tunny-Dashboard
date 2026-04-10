use super::{compute_ridge, SobolResult};

struct SobolSurrogate {
    param_means: Vec<f64>,
    param_stds: Vec<f64>,
    quad_feat_means: Vec<f64>,
    quad_feat_stds: Vec<f64>,
    betas: Vec<Vec<f64>>,
    intercepts: Vec<f64>,
}

fn column_mean_std(vals: &[f64]) -> (f64, f64) {
    let n = vals.len() as f64;
    let mean = vals.iter().sum::<f64>() / n;
    let std_dev = (vals.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n).sqrt();
    (mean, if std_dev < f64::EPSILON { 1.0 } else { std_dev })
}

pub(crate) fn lcg_next(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 11) as f64) / ((1u64 << 53) as f64)
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

    let x_quad_std: Vec<Vec<f64>> = quad_feats
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(j, &v)| (v - quad_feat_means[j]) / quad_feat_stds[j])
                .collect()
        })
        .collect();

    let n_objectives = y_matrix.len();
    let mut betas = Vec::with_capacity(n_objectives);
    let mut intercepts = Vec::with_capacity(n_objectives);

    for y in y_matrix {
        let y_mean = y.iter().sum::<f64>() / n as f64;
        let y_centered: Vec<f64> = y.iter().map(|&v| v - y_mean).collect();

        let ridge_res = compute_ridge(&x_quad_std, &y_centered, alpha);
        betas.push(ridge_res.beta);
        intercepts.push(y_mean);
    }

    Some(SobolSurrogate {
        param_means,
        param_stds,
        quad_feat_means,
        quad_feat_stds,
        betas,
        intercepts,
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

pub fn compute_sobol(n_samples: usize) -> Option<SobolResult> {
    crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n = df.row_count();
        let n_params = param_names.len();
        let n_objectives = objective_names.len();

        if n < 2 || n_params == 0 || n_objectives == 0 {
            return None;
        }

        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|row| {
                param_names
                    .iter()
                    .map(|name| {
                        df.get_numeric_column(name)
                            .and_then(|col| col.get(row).copied())
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();

        let y_matrix: Vec<Vec<f64>> = objective_names
            .iter()
            .map(|name| {
                (0..n)
                    .map(|row| {
                        df.get_numeric_column(name)
                            .and_then(|col| col.get(row).copied())
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();

        let surrogate = build_sobol_surrogate(&x_matrix, &y_matrix, n_params, 1.0)?;

        let mut rng_state: u64 = 0xDEAD_BEEF_1234_5678;

        let param_ranges: Vec<(f64, f64)> = param_names
            .iter()
            .map(|name| {
                let col = df.get_numeric_column(name).unwrap_or(&[]);
                if col.is_empty() {
                    return (0.0, 1.0);
                }
                let (min, max) = col
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
                        (lo.min(v), hi.max(v))
                    });
                if (max - min).abs() < f64::EPSILON {
                    (min - 1.0, max + 1.0)
                } else {
                    (min, max)
                }
            })
            .collect();

        let mat_a: Vec<Vec<f64>> = (0..n_samples)
            .map(|_| {
                param_ranges
                    .iter()
                    .map(|(lo, hi)| lo + lcg_next(&mut rng_state) * (hi - lo))
                    .collect()
            })
            .collect();

        let mat_b: Vec<Vec<f64>> = (0..n_samples)
            .map(|_| {
                param_ranges
                    .iter()
                    .map(|(lo, hi)| lo + lcg_next(&mut rng_state) * (hi - lo))
                    .collect()
            })
            .collect();

        let f_a: Vec<Vec<f64>> = (0..n_objectives)
            .map(|k| {
                mat_a
                    .iter()
                    .map(|row| surrogate_eval(&surrogate, row, k))
                    .collect()
            })
            .collect();

        let f_b: Vec<Vec<f64>> = (0..n_objectives)
            .map(|k| {
                mat_b
                    .iter()
                    .map(|row| surrogate_eval(&surrogate, row, k))
                    .collect()
            })
            .collect();

        let mut first_order = vec![vec![0.0f64; n_objectives]; n_params];
        let mut total_effect = vec![vec![0.0f64; n_objectives]; n_params];

        for pi in 0..n_params {
            let f_ab_pi: Vec<Vec<f64>> = {
                let ab_pi: Vec<Vec<f64>> = mat_a
                    .iter()
                    .zip(mat_b.iter())
                    .map(|(a_row, b_row)| {
                        let mut row = a_row.clone();
                        row[pi] = b_row[pi];
                        row
                    })
                    .collect();

                (0..n_objectives)
                    .map(|k| {
                        ab_pi
                            .iter()
                            .map(|row| surrogate_eval(&surrogate, row, k))
                            .collect()
                    })
                    .collect()
            };

            for k in 0..n_objectives {
                let fa = &f_a[k];
                let fb = &f_b[k];
                let fab = &f_ab_pi[k];

                let n_f = n_samples as f64;
                let mean_fa = fa.iter().sum::<f64>() / n_f;
                let var_y = fa.iter().map(|&v| (v - mean_fa).powi(2)).sum::<f64>() / n_f;

                if var_y < f64::EPSILON {
                    first_order[pi][k] = 0.0;
                    total_effect[pi][k] = 0.0;
                    continue;
                }

                let s_i: f64 = fb
                    .iter()
                    .zip(fab.iter())
                    .zip(fa.iter())
                    .map(|((&fb_j, &fab_j), &fa_j)| fb_j * (fab_j - fa_j))
                    .sum::<f64>()
                    / (n_f * var_y);

                let st_i: f64 = fa
                    .iter()
                    .zip(fab.iter())
                    .map(|(&fa_j, &fab_j)| (fa_j - fab_j).powi(2))
                    .sum::<f64>()
                    / (2.0 * n_f * var_y);

                first_order[pi][k] = s_i.clamp(0.0, 1.0);
                total_effect[pi][k] = st_i.clamp(0.0, 1.0);
            }
        }

        Some(SobolResult {
            param_names,
            objective_names,
            first_order,
            total_effect,
            n_samples,
        })
    })?
}
