//! Module documentation.
//!
//! Module documentation.
//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/tasks/tunny-dashboard-tasks.md TASK-801

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
#[derive(Debug, Clone)]
pub struct SensitivityResult {
    /// Documentation.
    pub param_names: Vec<String>,
    /// Documentation.
    pub objective_names: Vec<String>,
    /// Documentation.
    pub spearman: Vec<Vec<f64>>,
    /// Documentation.
    pub ridge: Vec<RidgeResult>,
}

/// Documentation.
///
/// Documentation.
#[derive(Debug, Clone)]
pub struct RidgeResult {
    /// Documentation.
    pub beta: Vec<f64>,
    /// Documentation.
    pub r_squared: f64,
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
fn rank(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return vec![];
    }

    // Documentation.
    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by(|&a, &b| {
        let va = values[a];
        let vb = values[b];
        match (va.is_nan(), vb.is_nan()) {
            (true, _) => std::cmp::Ordering::Greater,
            (_, true) => std::cmp::Ordering::Less,
            _ => va.partial_cmp(&vb).unwrap(),
        }
    });

    let mut ranks = vec![0.0f64; n];
    let mut i = 0;

    while i < n {
        let val = values[indices[i]];
        // Documentation.
        if val.is_nan() {
            // Documentation.
            let avg = (i as f64 + 1.0 + n as f64) / 2.0;
            for k in i..n {
                ranks[indices[k]] = avg;
            }
            break;
        }

        // Documentation.
        let mut j = i + 1;
        while j < n && values[indices[j]] == val {
            j += 1;
        }

        // Documentation.
        let avg_rank = (i as f64 + 1.0 + j as f64) / 2.0;
        for k in i..j {
            ranks[indices[k]] = avg_rank;
        }
        i = j;
    }

    ranks
}

/// Documentation.
///
/// Documentation.
/// Documentation.
fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len();
    if n < 2 {
        return 0.0;
    }
    let nf = n as f64;

    let mean_x: f64 = x.iter().sum::<f64>() / nf;
    let mean_y: f64 = y.iter().sum::<f64>() / nf;

    let mut cov = 0.0f64;
    let mut var_x = 0.0f64;
    let mut var_y = 0.0f64;

    for (&xi, &yi) in x.iter().zip(y.iter()) {
        let dx = xi - mean_x;
        let dy = yi - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    if denom < f64::EPSILON {
        return 0.0;
    }
    cov / denom
}

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
pub fn compute_spearman(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }

    // Documentation.
    let rx = rank(&x[..n]);
    let ry = rank(&y[..n]);

    pearson_correlation(&rx, &ry)
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
fn transpose_and_standardize(x_matrix: &[Vec<f64>], n: usize, p: usize) -> Vec<f64> {
    let mut x_cols = vec![0.0f64; n * p];

    // Documentation.
    for (i, row) in x_matrix.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            x_cols[j * n + i] = v;
        }
    }

    let nf = n as f64;

    // Documentation.
    for j in 0..p {
        let col = &mut x_cols[j * n..(j + 1) * n];

        // Documentation.
        let mean: f64 = col.iter().sum::<f64>() / nf;
        // Documentation.
        let std_dev = (col.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / nf).sqrt();
        let std_dev = if std_dev < f64::EPSILON { 1.0 } else { std_dev };

        // Documentation.
        for v in col.iter_mut() {
            *v = (*v - mean) / std_dev;
        }
    }

    x_cols
}

/// Documentation.
///
/// Documentation.
/// Documentation.
fn gaussian_elimination(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let p = b.len();
    if p == 0 {
        return Some(vec![]);
    }

    for col in 0..p {
        // Documentation.
        let pivot_row = (col..p)
            .max_by(|&i, &j| {
                a[i][col]
                    .abs()
                    .partial_cmp(&a[j][col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(col);

        a.swap(col, pivot_row);
        b.swap(col, pivot_row);

        let pivot = a[col][col];
        if pivot.abs() < 1e-12 {
            return None; // Documentation.
        }

        // Documentation.
        for row in (col + 1)..p {
            let factor = a[row][col] / pivot;
            for k in col..p {
                let v = a[col][k] * factor;
                a[row][k] -= v;
            }
            b[row] -= b[col] * factor;
        }
    }

    // Documentation.
    let mut x = vec![0.0f64; p];
    for i in (0..p).rev() {
        let mut sum = b[i];
        for j in (i + 1)..p {
            sum -= a[i][j] * x[j];
        }
        x[i] = sum / a[i][i];
    }

    Some(x)
}

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
pub fn compute_ridge(x_matrix: &[Vec<f64>], y: &[f64], alpha: f64) -> RidgeResult {
    let n = y.len();
    let empty = RidgeResult {
        beta: vec![],
        r_squared: 0.0,
    };

    if n < 2 || x_matrix.len() != n {
        return empty;
    }
    let p = x_matrix[0].len();
    if p == 0 {
        return empty;
    }

    // Documentation.
    let x_cols = transpose_and_standardize(x_matrix, n, p);

    // Documentation.
    let y_mean: f64 = y.iter().sum::<f64>() / n as f64;
    let y_c: Vec<f64> = y.iter().map(|&yi| yi - y_mean).collect();

    // Documentation.
    // Documentation.
    let mut xtx_flat = vec![0.0f64; p * p];
    for i in 0..p {
        for j in i..p {
            let col_i = &x_cols[i * n..(i + 1) * n];
            let col_j = &x_cols[j * n..(j + 1) * n];
            let val: f64 = col_i.iter().zip(col_j.iter()).map(|(a, b)| a * b).sum();
            xtx_flat[i * p + j] = val;
            xtx_flat[j * p + i] = val; // Documentation.
        }
    }
    // Documentation.
    for i in 0..p {
        xtx_flat[i * p + i] += alpha;
    }

    // Documentation.
    let mut xty = vec![0.0f64; p];
    for j in 0..p {
        let col_j = &x_cols[j * n..(j + 1) * n];
        xty[j] = col_j.iter().zip(y_c.iter()).map(|(xij, yi)| xij * yi).sum();
    }

    // Documentation.
    let xtx_2d: Vec<Vec<f64>> = (0..p)
        .map(|i| xtx_flat[i * p..(i + 1) * p].to_vec())
        .collect();
    let beta = match gaussian_elimination(xtx_2d, xty) {
        Some(b) => b,
        None => vec![0.0; p],
    };

    // Documentation.
    let y_hat: Vec<f64> = (0..n)
        .map(|i| (0..p).map(|j| x_cols[j * n + i] * beta[j]).sum())
        .collect();

    // Documentation.
    let ss_res: f64 = y_c
        .iter()
        .zip(y_hat.iter())
        .map(|(yi, yhi)| (yi - yhi).powi(2))
        .sum();
    let ss_tot: f64 = y_c.iter().map(|&yi| yi.powi(2)).sum();

    let r_squared = if ss_tot < f64::EPSILON {
        0.0
    } else {
        (1.0 - ss_res / ss_tot).max(0.0)
    };

    RidgeResult { beta, r_squared }
}

fn get_param_numeric_values(
    df: &crate::dataframe::DataFrame,
    param_name: &str,
    n: usize,
) -> Option<Vec<f64>> {
    if let Some(col) = df.get_numeric_column(param_name) {
        return Some(col.iter().take(n).copied().collect());
    }

    // Categorical params are stored as string columns; encode labels to stable ordinal ids.
    if let Some(col) = df.get_string_column(param_name) {
        use std::collections::HashMap;

        let mut label_to_id: HashMap<String, f64> = HashMap::new();
        let mut next_id = 0.0f64;
        let mut out = Vec::with_capacity(n);

        for label in col.iter().take(n) {
            let id = match label_to_id.get(label) {
                Some(v) => *v,
                None => {
                    let v = next_id;
                    label_to_id.insert(label.clone(), v);
                    next_id += 1.0;
                    v
                }
            };
            out.push(id);
        }

        return Some(out);
    }

    None
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn compute_sensitivity_all(df: &crate::dataframe::DataFrame) -> SensitivityResult {
    let param_names = df.param_col_names().to_vec();
    let objective_names = df.objective_col_names().to_vec();
    let n = df.row_count();

    if n < 2 || param_names.is_empty() || objective_names.is_empty() {
        return SensitivityResult {
            param_names,
            objective_names,
            spearman: vec![],
            ridge: vec![],
        };
    }

    // Documentation.
    let spearman: Vec<Vec<f64>> = param_names
        .iter()
        .map(|p_name| {
            let x = match get_param_numeric_values(df, p_name, n) {
                Some(col) => col,
                None => return vec![0.0; objective_names.len()],
            };
            objective_names
                .iter()
                .map(|o_name| {
                    let y = match df.get_numeric_column(o_name) {
                        Some(col) => col,
                        None => return 0.0,
                    };
                    compute_spearman(&x, y)
                })
                .collect()
        })
        .collect();

    // Documentation.
    // Documentation.
    let num_params = param_names.len();
    let mut x_cols_flat = vec![0.0f64; n * num_params];
    for (j, p_name) in param_names.iter().enumerate() {
        if let Some(col) = get_param_numeric_values(df, p_name, n) {
            for (i, &v) in col.iter().enumerate().take(n) {
                x_cols_flat[j * n + i] = v;
            }
        }
    }
    // Documentation.
    let nf = n as f64;
    for j in 0..num_params {
        let col = &mut x_cols_flat[j * n..(j + 1) * n];
        let mean: f64 = col.iter().sum::<f64>() / nf;
        let std_dev = (col.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / nf).sqrt();
        let std_dev = if std_dev < f64::EPSILON { 1.0 } else { std_dev };
        for v in col.iter_mut() {
            *v = (*v - mean) / std_dev;
        }
    }

    // Documentation.
    let mut xtx_flat = vec![0.0f64; num_params * num_params];
    for i in 0..num_params {
        for j in i..num_params {
            let col_i = &x_cols_flat[i * n..(i + 1) * n];
            let col_j = &x_cols_flat[j * n..(j + 1) * n];
            let val: f64 = col_i.iter().zip(col_j.iter()).map(|(a, b)| a * b).sum();
            xtx_flat[i * num_params + j] = val;
            xtx_flat[j * num_params + i] = val;
        }
    }
    for i in 0..num_params {
        xtx_flat[i * num_params + i] += 1.0; // Ridge alpha=1.0
    }

    // Documentation.
    let ridge: Vec<RidgeResult> = objective_names
        .iter()
        .map(|o_name| {
            let y: Vec<f64> = df
                .get_numeric_column(o_name)
                .map(|col| col[..n].to_vec())
                .unwrap_or_else(|| vec![0.0; n]);
            let y_mean = y.iter().sum::<f64>() / n as f64;
            let y_c: Vec<f64> = y.iter().map(|&v| v - y_mean).collect();

            // X^T y
            let mut xty = vec![0.0f64; num_params];
            for j in 0..num_params {
                let col_j = &x_cols_flat[j * n..(j + 1) * n];
                xty[j] = col_j.iter().zip(y_c.iter()).map(|(x, yy)| x * yy).sum();
            }

            // Documentation.
            let xtx_2d: Vec<Vec<f64>> = (0..num_params)
                .map(|i| xtx_flat[i * num_params..(i + 1) * num_params].to_vec())
                .collect();
            let beta = match gaussian_elimination(xtx_2d, xty) {
                Some(b) => b,
                None => vec![0.0; num_params],
            };

            let y_hat: Vec<f64> = (0..n)
                .map(|i| {
                    (0..num_params)
                        .map(|j| x_cols_flat[j * n + i] * beta[j])
                        .sum()
                })
                .collect();
            let ss_res: f64 = y_c
                .iter()
                .zip(y_hat.iter())
                .map(|(yi, yhi)| (yi - yhi).powi(2))
                .sum();
            let ss_tot: f64 = y_c.iter().map(|&yi| yi.powi(2)).sum();
            let r_squared = if ss_tot < f64::EPSILON {
                0.0
            } else {
                (1.0 - ss_res / ss_tot).max(0.0)
            };

            RidgeResult { beta, r_squared }
        })
        .collect();

    SensitivityResult {
        param_names,
        objective_names,
        spearman,
        ridge,
    }
}

/// Documentation.
pub fn compute_sensitivity() -> Option<SensitivityResult> {
    crate::dataframe::with_active_df(compute_sensitivity_all)
}

/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn compute_sensitivity_selected(indices: &[u32]) -> Option<SensitivityResult> {
    crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n_rows = df.row_count();

        if indices.is_empty() || param_names.is_empty() || objective_names.is_empty() {
            return SensitivityResult {
                param_names,
                objective_names,
                spearman: vec![],
                ridge: vec![],
            };
        }

        // Documentation.
        let valid_idx: Vec<usize> = indices
            .iter()
            .filter_map(|&i| {
                let u = i as usize;
                if u < n_rows {
                    Some(u)
                } else {
                    None
                }
            })
            .collect();

        if valid_idx.is_empty() {
            return SensitivityResult {
                param_names,
                objective_names,
                spearman: vec![],
                ridge: vec![],
            };
        }

        // Documentation.
        let spearman: Vec<Vec<f64>> = param_names
            .iter()
            .map(|p_name| {
                let full_x = match get_param_numeric_values(df, p_name, n_rows) {
                    Some(col) => col,
                    None => return vec![0.0; objective_names.len()],
                };
                let x_sub: Vec<f64> = valid_idx.iter().map(|&i| full_x[i]).collect();

                objective_names
                    .iter()
                    .map(|o_name| {
                        let full_y = match df.get_numeric_column(o_name) {
                            Some(col) => col,
                            None => return 0.0,
                        };
                        let y_sub: Vec<f64> = valid_idx.iter().map(|&i| full_y[i]).collect();
                        compute_spearman(&x_sub, &y_sub)
                    })
                    .collect()
            })
            .collect();

        // Documentation.
        let param_cols_all: Vec<Vec<f64>> = param_names
            .iter()
            .map(|p| get_param_numeric_values(df, p, n_rows).unwrap_or_else(|| vec![0.0; n_rows]))
            .collect();

        let x_matrix: Vec<Vec<f64>> = valid_idx
            .iter()
            .map(|&row_idx| {
                param_cols_all
                    .iter()
                    .map(|col| col.get(row_idx).copied().unwrap_or(0.0))
                    .collect()
            })
            .collect();

        let ridge: Vec<RidgeResult> = objective_names
            .iter()
            .map(|o_name| {
                let y_sub: Vec<f64> = valid_idx
                    .iter()
                    .map(|&row_idx| {
                        df.get_numeric_column(o_name)
                            .map(|col| col[row_idx])
                            .unwrap_or(0.0)
                    })
                    .collect();
                compute_ridge(&x_matrix, &y_sub, 1.0)
            })
            .collect();

        SensitivityResult {
            param_names,
            objective_names,
            spearman,
            ridge,
        }
    })
}

// =============================================================================
// Sobol 感度指数（サロゲート法・二次Ridge + Saltelli サンプリング）
// =============================================================================

/// 一次・全効果 Sobol 指数の計算結果
#[derive(Debug, Clone)]
pub struct SobolResult {
    pub param_names:     Vec<String>,
    pub objective_names: Vec<String>,
    /// 一次 Sobol 指数 [param_idx][obj_idx]  値域 [0, 1]
    pub first_order:     Vec<Vec<f64>>,
    /// 全効果 Sobol 指数 [param_idx][obj_idx]  値域 [0, 1]
    pub total_effect:    Vec<Vec<f64>>,
    pub n_samples:       usize,
}

/// 二次 Ridge サロゲートモデル
struct SobolSurrogate {
    n_params:        usize,
    param_means:     Vec<f64>,
    param_stds:      Vec<f64>,
    quad_feat_means: Vec<f64>,
    quad_feat_stds:  Vec<f64>,
    betas:           Vec<Vec<f64>>,   // [obj_idx][quad_feat_idx]
    intercepts:      Vec<f64>,        // [obj_idx]
}

/// LCG64 疑似乱数生成器（外部クレート不要）
fn lcg_next(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 11) as f64) / ((1u64 << 53) as f64)
}

/// 二次特徴量の構築
/// 入力: x_std (標準化済みの線形特徴量, 長さ p)
/// 出力: [x_1,...,x_p, x_1^2,...,x_p^2, x_1*x_2, x_1*x_3, ..., x_{p-1}*x_p]
fn build_quad_features(x_std: &[f64]) -> Vec<f64> {
    let p = x_std.len();
    let n_quad = 2 * p + p * (p - 1) / 2;
    let mut feat = Vec::with_capacity(n_quad);

    // 線形項
    feat.extend_from_slice(x_std);

    // 二乗項
    for &xi in x_std {
        feat.push(xi * xi);
    }

    // 交差項
    for i in 0..p {
        for j in (i + 1)..p {
            feat.push(x_std[i] * x_std[j]);
        }
    }

    feat
}

/// 二次 Ridge サロゲートを学習データから構築する
fn build_sobol_surrogate(
    x_matrix: &[Vec<f64>],  // [n_trials][n_params]
    y_matrix: &[Vec<f64>],  // [n_objectives][n_trials]
    n_params: usize,
    alpha: f64,
) -> Option<SobolSurrogate> {
    let n = x_matrix.len();
    if n < 2 || n_params == 0 {
        return None;
    }

    // --- Step 1: 線形特徴量を Z スコア標準化 ---
    let mut param_means = vec![0.0f64; n_params];
    let mut param_stds  = vec![1.0f64; n_params];

    for j in 0..n_params {
        let vals: Vec<f64> = x_matrix.iter().map(|row| row[j]).collect();
        let mean = vals.iter().sum::<f64>() / n as f64;
        let std_dev = (vals.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
        param_means[j] = mean;
        param_stds[j]  = if std_dev < f64::EPSILON { 1.0 } else { std_dev };
    }

    // 標準化後の線形特徴量行列を構築
    let x_std: Vec<Vec<f64>> = x_matrix
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(j, &v)| (v - param_means[j]) / param_stds[j])
                .collect()
        })
        .collect();

    // --- Step 2: 二次特徴量を構築 ---
    let quad_feats: Vec<Vec<f64>> = x_std.iter().map(|row| build_quad_features(row)).collect();
    let n_quad = quad_feats[0].len();

    // --- Step 3: 二次特徴量を標準化 ---
    let mut quad_feat_means = vec![0.0f64; n_quad];
    let mut quad_feat_stds  = vec![1.0f64; n_quad];

    for j in 0..n_quad {
        let vals: Vec<f64> = quad_feats.iter().map(|row| row[j]).collect();
        let mean = vals.iter().sum::<f64>() / n as f64;
        let std_dev = (vals.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
        quad_feat_means[j] = mean;
        quad_feat_stds[j]  = if std_dev < f64::EPSILON { 1.0 } else { std_dev };
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

    // --- Step 4: 各目的関数に Ridge 回帰を適合 ---
    let n_objectives = y_matrix.len();
    let mut betas     = Vec::with_capacity(n_objectives);
    let mut intercepts = Vec::with_capacity(n_objectives);

    for y in y_matrix {
        let y_mean = y.iter().sum::<f64>() / n as f64;
        let y_centered: Vec<f64> = y.iter().map(|&v| v - y_mean).collect();

        let ridge_res = compute_ridge(&x_quad_std, &y_centered, alpha);
        betas.push(ridge_res.beta);
        intercepts.push(y_mean);
    }

    Some(SobolSurrogate {
        n_params,
        param_means,
        param_stds,
        quad_feat_means,
        quad_feat_stds,
        betas,
        intercepts,
    })
}

/// サロゲートモデルで 1 点を評価する
fn surrogate_eval(surrogate: &SobolSurrogate, x_raw: &[f64], obj_idx: usize) -> f64 {
    // 線形標準化
    let x_std: Vec<f64> = x_raw
        .iter()
        .enumerate()
        .map(|(j, &v)| (v - surrogate.param_means[j]) / surrogate.param_stds[j])
        .collect();

    // 二次特徴量構築
    let quad = build_quad_features(&x_std);

    // 二次特徴量標準化
    let quad_std: Vec<f64> = quad
        .iter()
        .enumerate()
        .map(|(j, &v)| (v - surrogate.quad_feat_means[j]) / surrogate.quad_feat_stds[j])
        .collect();

    // Ridge モデル評価
    let beta = &surrogate.betas[obj_idx];
    let dot: f64 = beta.iter().zip(quad_std.iter()).map(|(&b, &x)| b * x).sum();
    dot + surrogate.intercepts[obj_idx]
}

/// Sobol 指数を計算する（全トライアル対象）
pub fn compute_sobol(n_samples: usize) -> Option<SobolResult> {
    crate::dataframe::with_active_df(|df| {
        let param_names     = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n               = df.row_count();
        let n_params        = param_names.len();
        let n_objectives    = objective_names.len();

        if n < 2 || n_params == 0 || n_objectives == 0 {
            return None;
        }

        // DataFrameからX行列とY行列を取得
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

        // 二次 Ridge サロゲートを構築
        let surrogate = build_sobol_surrogate(&x_matrix, &y_matrix, n_params, 1.0)?;

        let mut rng_state: u64 = 0xDEAD_BEEF_1234_5678;

        // パラメータ範囲を DataFrameから取得（min-max）
        let param_ranges: Vec<(f64, f64)> = param_names
            .iter()
            .map(|name| {
                let col = df.get_numeric_column(name).unwrap_or(&[]);
                if col.is_empty() {
                    return (0.0, 1.0);
                }
                let min = col.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = col.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                if (max - min).abs() < f64::EPSILON {
                    (min - 1.0, max + 1.0)
                } else {
                    (min, max)
                }
            })
            .collect();

        // 行列 A, B を生成: N × n_params
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

        // f_A[obj_idx][sample_idx]
        let f_a: Vec<Vec<f64>> = (0..n_objectives)
            .map(|k| {
                mat_a
                    .iter()
                    .map(|row| surrogate_eval(&surrogate, row, k))
                    .collect()
            })
            .collect();

        // f_B[obj_idx][sample_idx]
        let f_b: Vec<Vec<f64>> = (0..n_objectives)
            .map(|k| {
                mat_b
                    .iter()
                    .map(|row| surrogate_eval(&surrogate, row, k))
                    .collect()
            })
            .collect();

        // === Jansen/Saltelli 推定量 ===
        let mut first_order = vec![vec![0.0f64; n_objectives]; n_params];
        let mut total_effect = vec![vec![0.0f64; n_objectives]; n_params];

        for pi in 0..n_params {
            // AB_pi: A の第 pi 列を B の第 pi 列で置換
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
                let fa  = &f_a[k];
                let fb  = &f_b[k];
                let fab = &f_ab_pi[k];

                let n_f = n_samples as f64;
                let mean_fa  = fa.iter().sum::<f64>() / n_f;
                let var_y    = fa.iter().map(|&v| (v - mean_fa).powi(2)).sum::<f64>() / n_f;

                if var_y < f64::EPSILON {
                    first_order[pi][k]  = 0.0;
                    total_effect[pi][k] = 0.0;
                    continue;
                }

                // 一次 Sobol 指数 (Saltelli 2010)
                let s_i: f64 = fb.iter()
                    .zip(fab.iter())
                    .zip(fa.iter())
                    .map(|((&fb_j, &fab_j), &fa_j)| fb_j * (fab_j - fa_j))
                    .sum::<f64>()
                    / (n_f * var_y);

                // 全効果 Sobol 指数 (Jansen 1999)
                let st_i: f64 = fa.iter()
                    .zip(fab.iter())
                    .map(|(&fa_j, &fab_j)| (fa_j - fab_j).powi(2))
                    .sum::<f64>()
                    / (2.0 * n_f * var_y);

                first_order[pi][k]  = s_i.clamp(0.0, 1.0);
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

// =============================================================================
// Documentation.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
    use std::collections::HashMap;

    // -------------------------------------------------------------------------
    // Documentation.
    // -------------------------------------------------------------------------

    /// Documentation.
    fn make_row_multi(trial_id: u32, params: &[(&str, f64)], objectives: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id,
            param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            param_category_label: HashMap::new(),
            objective_values: objectives,
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }
    }

    /// Documentation.
    fn setup_df(rows: Vec<TrialRow>, params: &[&str], objs: &[&str]) -> DataFrame {
        let param_names: Vec<String> = params.iter().map(|s| s.to_string()).collect();
        let obj_names: Vec<String> = objs.iter().map(|s| s.to_string()).collect();
        let df = DataFrame::from_trials(&rows, &param_names, &obj_names, &[], &[], 0);
        store_dataframes(vec![df.clone()]);
        select_study(0).expect("study 0 translated");
        df
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_801_01_spearman_perfect_positive() {
        // Documentation.
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0]; // Documentation.

        let r = compute_spearman(&x, &y);

        assert!(
            (r - 1.0).abs() < 1e-9,
            "translatedSpearmantranslated1.0translated: {}",
            r
        );
    }

    #[test]
    fn tc_801_02_spearman_perfect_negative() {
        // Documentation.
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 4.0, 3.0, 2.0, 1.0]; // Documentation.

        let r = compute_spearman(&x, &y);

        assert!(
            (r + 1.0).abs() < 1e-9,
            "translatedSpearmantranslated-1.0translated: {}",
            r
        );
    }

    #[test]
    fn tc_801_03_spearman_known_example() {
        // Documentation.
        // Documentation.
        // Documentation.
        // Documentation.
        // Documentation.
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let y = vec![4.0, 1.0, 2.0, 5.0, 6.0, 3.0];

        let r = compute_spearman(&x, &y);

        let expected = 13.0 / 35.0; // = 0.37142857...
        assert!(
            (r - expected).abs() < 1e-9,
            "Spearmantranslated: expected={}, got={}",
            expected,
            r
        );
    }

    #[test]
    fn tc_801_04_spearman_tied_ranks() {
        // Documentation.
        let x = vec![1.0, 2.0, 2.0, 3.0]; // Documentation.
        let y = vec![1.0, 2.0, 3.0, 4.0];

        let r = compute_spearman(&x, &y);

        // Documentation.
        assert!(r > 0.9, "translated: {}", r);
    }

    #[test]
    fn tc_801_05_spearman_n_less_than_2_returns_zero() {
        // Documentation.
        let r1 = compute_spearman(&[], &[]);
        let r2 = compute_spearman(&[1.0], &[1.0]);

        assert_eq!(r1, 0.0, "translated0.0translated");
        assert_eq!(r2, 0.0, "n=1translated0.0translated");
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_801_06_ridge_perfect_linear_r_squared_near_1() {
        // Documentation.
        let n = 50;
        let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let y: Vec<f64> = (0..n).map(|i| 2.0 * i as f64 + 1.0).collect();

        let result = compute_ridge(&x_matrix, &y, 0.001);

        assert!(
            result.r_squared > 0.99,
            "translatedR²translated1.0translated: {}",
            result.r_squared
        );
    }

    #[test]
    fn tc_801_07_ridge_beta_sign_correct() {
        // Documentation.
        let n = 20;
        let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let y: Vec<f64> = (0..n).map(|i| 3.0 * i as f64).collect();

        let result = compute_ridge(&x_matrix, &y, 0.01);

        assert!(
            result.beta[0] > 0.0,
            "translatedβ>0translated: {}",
            result.beta[0]
        );
    }

    #[test]
    fn tc_801_08_ridge_two_params_identifies_stronger() {
        // Documentation.
        let n = 50;
        // Documentation.
        let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (i % 5) as f64]).collect();
        let y: Vec<f64> = (0..n).map(|i| i as f64 + 0.1 * (i % 5) as f64).collect();

        let result = compute_ridge(&x_matrix, &y, 0.01);

        assert_eq!(result.beta.len(), 2, "βtranslated2translated");
        assert!(
            result.beta[0].abs() > result.beta[1].abs(),
            "x1translatedx2translated: beta={:?}",
            result.beta
        );
    }

    #[test]
    fn tc_801_09_ridge_empty_returns_zero_r_squared() {
        // Documentation.
        let result = compute_ridge(&[], &[], 1.0);

        assert_eq!(result.beta.len(), 0, "translatedβtranslated");
        assert_eq!(result.r_squared, 0.0, "translatedR²=0.0");
    }

    // =========================================================================
    // compute_sensitivity_all — DataFrame support
    // =========================================================================

    #[test]
    fn tc_801_10_sensitivity_all_correct_structure() {
        // Documentation.
        let rows: Vec<TrialRow> = (0..10)
            .map(|i| {
                make_row_multi(
                    i,
                    &[("x1", i as f64), ("x2", (10 - i) as f64)],
                    vec![i as f64, (10 - i) as f64],
                )
            })
            .collect();
        let df = setup_df(rows, &["x1", "x2"], &["obj0", "obj1"]);

        let result = compute_sensitivity_all(&df);

        // Documentation.
        assert_eq!(result.param_names.len(), 2); // x1, x2
        assert_eq!(result.objective_names.len(), 2); // obj0, obj1
        assert_eq!(result.spearman.len(), 2); // Documentation.
        assert_eq!(result.spearman[0].len(), 2); // Documentation.
        assert_eq!(result.ridge.len(), 2); // Documentation.
    }

    #[test]
    fn tc_801_11_sensitivity_all_known_correlations() {
        // Documentation.
        let rows: Vec<TrialRow> = (0..20)
            .map(|i| {
                make_row_multi(
                    i,
                    &[("x1", i as f64), ("x2", (20 - i) as f64)],
                    vec![i as f64], // Documentation.
                )
            })
            .collect();
        let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

        let result = compute_sensitivity_all(&df);

        // Documentation.
        assert!(
            result.spearman[0][0] > 0.99,
            "x1-obj0translated: {}",
            result.spearman[0][0]
        );
        assert!(
            result.spearman[1][0] < -0.99,
            "x2-obj0translated: {}",
            result.spearman[1][0]
        );
    }

    #[test]
    fn tc_801_11b_sensitivity_all_categorical_param_non_zero() {
        use std::collections::HashMap;

        let labels = ["A", "B", "C", "A", "B", "C"];
        let y_vals = [1.0, 2.0, 3.0, 1.2, 2.2, 3.2];

        let rows: Vec<TrialRow> = labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let idx = match *label {
                    "A" => 0.0,
                    "B" => 1.0,
                    _ => 2.0,
                };

                let mut param_display = HashMap::new();
                param_display.insert("cat".to_string(), idx);

                let mut param_category_label = HashMap::new();
                param_category_label.insert("cat".to_string(), (*label).to_string());

                TrialRow {
                    trial_id: i as u32,
                    param_display,
                    param_category_label,
                    objective_values: vec![y_vals[i]],
                    user_attrs_numeric: HashMap::new(),
                    user_attrs_string: HashMap::new(),
                    constraint_values: vec![],
                }
            })
            .collect();

        let df = setup_df(rows, &["cat"], &["obj0"]);
        let result = compute_sensitivity_all(&df);

        assert_eq!(result.param_names, vec!["cat"]);
        assert_eq!(result.objective_names, vec!["obj0"]);
        assert!(
            result.spearman[0][0].abs() > 0.7,
            "categorical param should contribute to sensitivity: {}",
            result.spearman[0][0]
        );
        assert!(
            result.ridge[0].beta[0].abs() > 0.0,
            "categorical param beta should not be zero"
        );
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_801_12_sensitivity_selected_subset() {
        // Documentation.
        let rows: Vec<TrialRow> = (0..20)
            .map(|i| make_row_multi(i, &[("x1", i as f64)], vec![i as f64]))
            .collect();
        setup_df(rows, &["x1"], &["obj0"]);

        // Documentation.
        let indices: Vec<u32> = (0..10).collect();
        let result = compute_sensitivity_selected(&indices).expect("translated");

        assert_eq!(result.param_names, vec!["x1"]);
        assert_eq!(result.objective_names, vec!["obj0"]);
        // Documentation.
        assert!(
            result.spearman[0][0] > 0.99,
            "translated: {}",
            result.spearman[0][0]
        );
    }

    #[test]
    fn tc_801_13_sensitivity_selected_empty_indices() {
        // Documentation.
        let rows: Vec<TrialRow> = (0..5)
            .map(|i| make_row_multi(i, &[("x1", i as f64)], vec![i as f64]))
            .collect();
        setup_df(rows, &["x1"], &["obj0"]);

        let result = compute_sensitivity_selected(&[]).expect("translated");

        assert!(result.spearman.is_empty(), "translatedspearmantranslated");
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_801_p01_spearman_50000_x_30_x_4_under_500ms() {
        // Documentation.
        // Documentation.
        // Documentation.
        #[cfg(debug_assertions)]
        let (n, n_params, n_objs) = (5_000usize, 10usize, 4usize);
        #[cfg(not(debug_assertions))]
        let (n, n_params, n_objs) = (50_000usize, 30usize, 4usize);

        let param_cols: Vec<Vec<f64>> = (0..n_params)
            .map(|p| (0..n).map(|i| (i + p) as f64).collect())
            .collect();
        let obj_cols: Vec<Vec<f64>> = (0..n_objs)
            .map(|o| (0..n).map(|i| (i * (o + 1)) as f64).collect())
            .collect();

        let start = std::time::Instant::now();
        for p in 0..n_params {
            for o in 0..n_objs {
                let _ = compute_spearman(&param_cols[p], &obj_cols[o]);
            }
        }
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() <= 500,
            "Spearmantranslated{}mstranslated（translated: ≤500ms, n={}, params={}, objs={}）",
            elapsed.as_millis(),
            n,
            n_params,
            n_objs
        );
    }

    #[test]
    fn tc_801_p02_ridge_50000_x_30_under_300ms() {
        // Documentation.
        // Documentation.
        // Documentation.
        #[cfg(debug_assertions)]
        let (n, n_params, n_objs) = (5_000usize, 10usize, 4usize);
        #[cfg(not(debug_assertions))]
        let (n, n_params, n_objs) = (50_000usize, 30usize, 4usize);

        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..n_params).map(|p| (i + p) as f64).collect())
            .collect();
        let y_vecs: Vec<Vec<f64>> = (0..n_objs)
            .map(|o| (0..n).map(|i| (i * (o + 1)) as f64).collect())
            .collect();

        let start = std::time::Instant::now();
        for y in &y_vecs {
            let _ = compute_ridge(&x_matrix, y, 1.0);
        }
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() <= 300,
            "Ridgetranslated{}mstranslated（translated: ≤300ms, n={}, params={}）",
            elapsed.as_millis(),
            n,
            n_params
        );
    }

    #[test]
    fn tc_801_p03_sensitivity_selected_under_50ms() {
        // Documentation.
        // Documentation.
        // Documentation.
        #[cfg(debug_assertions)]
        let n = 5_000usize;
        #[cfg(not(debug_assertions))]
        let n = 50_000usize;

        let rows: Vec<TrialRow> = (0..n)
            .map(|i| make_row_multi(i as u32, &[("x1", i as f64)], vec![i as f64; 4]))
            .collect();
        setup_df(rows, &["x1"], &["obj0", "obj1", "obj2", "obj3"]);

        let indices: Vec<u32> = (0..n as u32).collect();
        let start = std::time::Instant::now();
        let _ = compute_sensitivity_selected(&indices);
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() <= 50,
            "compute_sensitivity_selected translated {}ms translated（translated: ≤50ms, n={}）",
            elapsed.as_millis(),
            n
        );
    }

    // =========================================================================
    // TASK-1610: Sobol tests
    // =========================================================================

    #[test]
    fn tc_1610_01_build_quad_features_output_length() {
        // Given: p=3 の入力ベクトル
        let x = vec![1.0, 2.0, 3.0];
        let feats = build_quad_features(&x);
        // p(p+3)/2 = 3*6/2 = 9
        assert_eq!(feats.len(), 9);
    }

    #[test]
    fn tc_1610_02_lcg_next_range() {
        let mut state: u64 = 12345;
        for _ in 0..1000 {
            let v = lcg_next(&mut state);
            assert!(v >= 0.0 && v < 1.0, "lcg_next out of [0,1): {}", v);
        }
    }

    #[test]
    fn tc_1610_03_compute_sobol_insufficient_data_returns_none() {
        // Given: DataFrame に 1 行のみ
        let rows: Vec<TrialRow> = vec![make_row_multi(0, &[("x1", 1.0), ("x2", 2.0)], vec![3.0])];
        setup_df(rows, &["x1", "x2"], &["obj0"]);
        let result = compute_sobol(1024);
        assert!(result.is_none(), "n<2 の場合 None を返すこと");
    }

    #[test]
    fn tc_1610_04_sobol_indices_in_range() {
        // Given: 2 パラメータ・1 目的関数
        let rows: Vec<TrialRow> = (0..50)
            .map(|i| {
                let x1 = i as f64;
                let x2 = (i * 2) as f64;
                let y  = x1 * 2.0; // x1 のみが目的関数に影響
                make_row_multi(i as u32, &[("x1", x1), ("x2", x2)], vec![y])
            })
            .collect();
        setup_df(rows, &["x1", "x2"], &["obj0"]);
        let result = compute_sobol(1024);
        assert!(result.is_some());
        let r = result.unwrap();
        for pi in 0..r.param_names.len() {
            for k in 0..r.objective_names.len() {
                assert!(r.first_order[pi][k] >= 0.0 && r.first_order[pi][k] <= 1.0);
                assert!(r.total_effect[pi][k] >= 0.0 && r.total_effect[pi][k] <= 1.0);
            }
        }
    }

    #[test]
    fn tc_1610_integration_output_shape() {
        // p=5, n=100 のダミーデータ
        let rows: Vec<TrialRow> = (0..100)
            .map(|i| {
                make_row_multi(
                    i as u32,
                    &[
                        ("p0", i as f64),
                        ("p1", (i + 1) as f64),
                        ("p2", (i + 2) as f64),
                        ("p3", (i + 3) as f64),
                        ("p4", (i + 4) as f64),
                    ],
                    vec![i as f64],
                )
            })
            .collect();
        setup_df(rows, &["p0", "p1", "p2", "p3", "p4"], &["obj0"]);
        let result = compute_sobol(512);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.first_order.len(), 5);
        assert_eq!(r.first_order[0].len(), 1);
        assert_eq!(r.total_effect.len(), 5);
    }
}
