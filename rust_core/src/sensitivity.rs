//! 感度分析: Spearman相関・Ridge回帰・R² (TASK-801)
//!
//! REQ-090: Spearman相関 compute_spearman() — O(n log n) × P × M
//! REQ-091: Ridge回帰 compute_ridge() → 標準化偏回帰係数β・R²
//! REQ-092: 絞り込みサブセット再計算 compute_sensitivity_selected()
//!
//! 参照: docs/tasks/tunny-dashboard-tasks.md TASK-801

// =============================================================================
// 公開型定義
// =============================================================================

/// 感度分析の結果（全パラメータ×全目的の Spearman 相関行列）
///
/// 【設計】: spearman[i][j] = param_names[i] と objective_names[j] の Spearman 相関
#[derive(Debug, Clone)]
pub struct SensitivityResult {
    /// パラメータ名リスト
    pub param_names: Vec<String>,
    /// 目的名リスト
    pub objective_names: Vec<String>,
    /// 相関行列: spearman[param_idx][objective_idx] ∈ [-1.0, 1.0]
    pub spearman: Vec<Vec<f64>>,
    /// 目的ごとの Ridge 回帰結果（パラメータ順）
    pub ridge: Vec<RidgeResult>,
}

/// Ridge 回帰の結果
///
/// 【設計】: 標準化済みデータに対するβ係数とR²
#[derive(Debug, Clone)]
pub struct RidgeResult {
    /// 標準化後の偏回帰係数 β — 絶対値が大きいほど感度が高い
    pub beta: Vec<f64>,
    /// 決定係数 R² ∈ [0.0, 1.0] — モデル適合度
    pub r_squared: f64,
}

// =============================================================================
// Spearman 順位相関係数
// =============================================================================

/// 値配列の順位を計算する（同値は平均順位・1始まり）
///
/// 【アルゴリズム】: ソート → 隣接同値グループの平均順位を割り当てる
/// 【複雑度】: O(n log n) 🟢
fn rank(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return vec![];
    }

    // 【インデックスソート】: NaN は末尾に送る
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
        // 【NaN終端】: NaN のグループはすべて末尾 → 最大順位を割り当て
        if val.is_nan() {
            // NaN には n+1 相当の大きな順位（相関計算上は問題ないが使用側で除外推奨）
            let avg = (i as f64 + 1.0 + n as f64) / 2.0;
            for k in i..n {
                ranks[indices[k]] = avg;
            }
            break;
        }

        // 【同値グループ検出】: 同じ値が連続する範囲を探す
        let mut j = i + 1;
        while j < n && values[indices[j]] == val {
            j += 1;
        }

        // 【平均順位割り当て】: 1始まりの順位で (i+1 + j) / 2
        let avg_rank = (i as f64 + 1.0 + j as f64) / 2.0;
        for k in i..j {
            ranks[indices[k]] = avg_rank;
        }
        i = j;
    }

    ranks
}

/// ピアソン積率相関係数を計算する
///
/// 【前提】: x と y は同じ長さであること
/// 【戻り値】: [-1.0, 1.0] の相関係数、定数列の場合は 0.0
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

/// Spearman 順位相関係数を計算する
///
/// 【アルゴリズム】: rank(x) と rank(y) のピアソン相関 = Spearman 相関
/// 【複雑度】: O(n log n) 🟢
/// 【戻り値】: [-1.0, 1.0] の相関係数、n < 2 の場合は 0.0
pub fn compute_spearman(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }

    // 【順位変換】: 各配列をランクに変換してピアソン相関を計算
    let rx = rank(&x[..n]);
    let ry = rank(&y[..n]);

    pearson_correlation(&rx, &ry)
}

// =============================================================================
// Ridge 回帰
// =============================================================================

/// 行優先 x_matrix (n×p) を列優先フラット配列に変換し、各列を標準化する
///
/// 【戻り値】: 列優先フラット配列 x_cols (p×n): x_cols[j*n + i] = 標準化後の x_matrix[i][j]
/// 【設計】: 列優先でXTX計算するとキャッシュ効率が大幅向上する 🟢
fn transpose_and_standardize(x_matrix: &[Vec<f64>], n: usize, p: usize) -> Vec<f64> {
    let mut x_cols = vec![0.0f64; n * p];

    // 【転置】: 行優先 → 列優先
    for (i, row) in x_matrix.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            x_cols[j * n + i] = v;
        }
    }

    let nf = n as f64;

    // 【各列を標準化】: 列ごとに平均0・標準偏差1に変換
    for j in 0..p {
        let col = &mut x_cols[j * n..(j + 1) * n];

        // 列平均
        let mean: f64 = col.iter().sum::<f64>() / nf;
        // 列標準偏差
        let std_dev = (col.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / nf).sqrt();
        let std_dev = if std_dev < f64::EPSILON { 1.0 } else { std_dev };

        // 標準化適用
        for v in col.iter_mut() {
            *v = (*v - mean) / std_dev;
        }
    }

    x_cols
}

/// ガウス消去法（部分ピボット選択付き）で Ax = b を解く
///
/// 【前提】: A は p×p の正方行列
/// 【戻り値】: 解ベクタ x、特異行列の場合は None
fn gaussian_elimination(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let p = b.len();
    if p == 0 {
        return Some(vec![]);
    }

    for col in 0..p {
        // 【部分ピボット選択】: 最大絶対値の行を先頭に持ってくる
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
            return None; // 【特異行列】: 解が求まらない場合は None を返す
        }

        // 【前進消去】
        for row in (col + 1)..p {
            let factor = a[row][col] / pivot;
            for k in col..p {
                let v = a[col][k] * factor;
                a[row][k] -= v;
            }
            b[row] -= b[col] * factor;
        }
    }

    // 【後退代入】
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

/// Ridge 回帰を計算する（列優先最適化版）
///
/// 【アルゴリズム】: 正規方程式 (X^T X + alpha * I) * β = X^T y をガウス消去法で解く
/// 【最適化】: X を列優先フラット配列に変換し XTX 計算のキャッシュ効率を大幅向上
/// 【前処理】: X を標準化し y を中心化して切片を排除
/// 【複雑度】: O(n×p²/2) — 上三角のみ計算して対称性を利用 🟢
/// @param x_matrix 特徴行列 (n×p) — 行優先
/// @param y 目的ベクタ (n,)
/// @param alpha Ridge 正則化パラメータ（通常 1.0）
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

    // 【転置＋標準化】: 列優先フラット配列に変換 (x_cols[j*n+i] = 標準化済み値)
    let x_cols = transpose_and_standardize(x_matrix, n, p);

    // 【y の中心化】: 切片を陽に扱わない
    let y_mean: f64 = y.iter().sum::<f64>() / n as f64;
    let y_c: Vec<f64> = y.iter().map(|&yi| yi - y_mean).collect();

    // 【X^T X の計算（列優先・上三角のみ）】: p×p フラット対称行列
    // キャッシュ効率: 各列は n 個の連続 f64 → LLVM が自動ベクトル化しやすい
    let mut xtx_flat = vec![0.0f64; p * p];
    for i in 0..p {
        for j in i..p {
            let col_i = &x_cols[i * n..(i + 1) * n];
            let col_j = &x_cols[j * n..(j + 1) * n];
            let val: f64 = col_i.iter().zip(col_j.iter()).map(|(a, b)| a * b).sum();
            xtx_flat[i * p + j] = val;
            xtx_flat[j * p + i] = val; // 【対称性】: 下三角は上三角のコピー
        }
    }
    // 【Ridge 正則化】: 対角成分に alpha を加えて逆行列の安定性を高める
    for i in 0..p {
        xtx_flat[i * p + i] += alpha;
    }

    // 【X^T y の計算】: 列ベクタと y_c の内積
    let mut xty = vec![0.0f64; p];
    for j in 0..p {
        let col_j = &x_cols[j * n..(j + 1) * n];
        xty[j] = col_j.iter().zip(y_c.iter()).map(|(xij, yi)| xij * yi).sum();
    }

    // 【β の解法】: ガウス消去法 (p×p 系)
    let xtx_2d: Vec<Vec<f64>> = (0..p)
        .map(|i| xtx_flat[i * p..(i + 1) * p].to_vec())
        .collect();
    let beta = match gaussian_elimination(xtx_2d, xty) {
        Some(b) => b,
        None => vec![0.0; p],
    };

    // 【予測値計算】: ŷ_i = Σ_j x_cols[j*n+i] * beta[j]
    let y_hat: Vec<f64> = (0..n)
        .map(|i| (0..p).map(|j| x_cols[j * n + i] * beta[j]).sum())
        .collect();

    // 【R² 計算】: R² = 1 - SS_res / SS_tot
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

// =============================================================================
// DataFrame 対応 — 感度分析 API
// =============================================================================

/// DataFrame の全パラメータ×全目的の感度行列を計算する
///
/// 【設計】: Spearman + Ridge を同時計算して SensitivityResult に集約する
/// 【複雑度】: O(P × M × n log n) + O(P × M × n × P) 🟢
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

    // 【Spearman 行列計算】: param_names[i] × objective_names[j]
    let spearman: Vec<Vec<f64>> = param_names
        .iter()
        .map(|p_name| {
            let x = match df.get_numeric_column(p_name) {
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
                    compute_spearman(x, y)
                })
                .collect()
        })
        .collect();

    // 【X の列優先変換＋標準化】: n×P → P×n 列優先フラット配列
    // DataFrame の列を直接コピーして行列構築のオーバーヘッドを削減
    let num_params = param_names.len();
    let mut x_cols_flat = vec![0.0f64; n * num_params];
    for (j, p_name) in param_names.iter().enumerate() {
        if let Some(col) = df.get_numeric_column(p_name) {
            for (i, &v) in col.iter().enumerate().take(n) {
                x_cols_flat[j * n + i] = v;
            }
        }
    }
    // 各列を標準化する
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

    // 【X^T X の事前計算】: 全目的で共有（コスト節約）
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

    // 【Ridge 回帰】: XTX を再利用して目的ごとに β・R² を計算
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

            // ガウス消去法 (XTX を clone して使う)
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

/// アクティブ Study の全試行に対して感度分析を実行する
pub fn compute_sensitivity() -> Option<SensitivityResult> {
    crate::dataframe::with_active_df(compute_sensitivity_all)
}

/// アクティブ Study の指定インデックスサブセットで感度分析を再計算する
///
/// 【設計】: Brushing 選択後のサブセットで感度を即時再計算する（REQ-092）
/// 【複雑度】: サブセット n_sub に対して O(P × M × n_sub log n_sub) 🟢
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

        // 【有効インデックスフィルタ】: 範囲外インデックスをスキップ
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

        // 【Spearman サブセット計算】
        let spearman: Vec<Vec<f64>> = param_names
            .iter()
            .map(|p_name| {
                let full_x = match df.get_numeric_column(p_name) {
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

        // 【Ridge サブセット計算】: サブセット X 行列
        let x_matrix: Vec<Vec<f64>> = valid_idx
            .iter()
            .map(|&row_idx| {
                param_names
                    .iter()
                    .map(|p| {
                        df.get_numeric_column(p)
                            .map(|col| col[row_idx])
                            .unwrap_or(0.0)
                    })
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
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
    use std::collections::HashMap;

    // -------------------------------------------------------------------------
    // テストヘルパー
    // -------------------------------------------------------------------------

    /// 多パラメータ・多目的の TrialRow を生成するヘルパー
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

    /// DataFrame を構築してアクティブに設定するヘルパー
    fn setup_df(rows: Vec<TrialRow>, params: &[&str], objs: &[&str]) -> DataFrame {
        let param_names: Vec<String> = params.iter().map(|s| s.to_string()).collect();
        let obj_names: Vec<String> = objs.iter().map(|s| s.to_string()).collect();
        let df = DataFrame::from_trials(&rows, &param_names, &obj_names, &[], &[], 0);
        store_dataframes(vec![df.clone()]);
        select_study(0).expect("study 0 は存在するはず");
        df
    }

    // =========================================================================
    // Spearman 相関係数 — 正常系
    // =========================================================================

    #[test]
    fn tc_801_01_spearman_perfect_positive() {
        // 【テスト目的】: 完全正相関データで Spearman = 1.0 を返す 🟢
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0]; // x の 2 倍

        let r = compute_spearman(&x, &y);

        assert!(
            (r - 1.0).abs() < 1e-9,
            "完全正相関のSpearmanは1.0のはず: {}",
            r
        );
    }

    #[test]
    fn tc_801_02_spearman_perfect_negative() {
        // 【テスト目的】: 完全負相関データで Spearman = -1.0 を返す 🟢
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 4.0, 3.0, 2.0, 1.0]; // 順位が完全逆

        let r = compute_spearman(&x, &y);

        assert!(
            (r + 1.0).abs() < 1e-9,
            "完全負相関のSpearmanは-1.0のはず: {}",
            r
        );
    }

    #[test]
    fn tc_801_03_spearman_known_example() {
        // 【テスト目的】: 既知の数値例で Spearman 相関が期待値と一致する 🟢
        // x の順位: [1, 2, 3, 4, 5, 6]
        // y=[4,1,2,5,6,3] の順位: [4, 1, 2, 5, 6, 3]
        // 順位差 d: [-3, 1, 1, -1, -1, 3] → Σd²=22
        // 簡易公式: 1 - 6Σd²/n(n²-1) = 1 - 132/210 = 13/35 ≈ 0.37143
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let y = vec![4.0, 1.0, 2.0, 5.0, 6.0, 3.0];

        let r = compute_spearman(&x, &y);

        let expected = 13.0 / 35.0; // = 0.37142857...
        assert!(
            (r - expected).abs() < 1e-9,
            "Spearman相関が期待値と不一致: expected={}, got={}",
            expected,
            r
        );
    }

    #[test]
    fn tc_801_04_spearman_tied_ranks() {
        // 【テスト目的】: 同値（タイ）がある場合でも正常に計算される 🟢
        let x = vec![1.0, 2.0, 2.0, 3.0]; // 2 が 2 個
        let y = vec![1.0, 2.0, 3.0, 4.0];

        let r = compute_spearman(&x, &y);

        // 同値あり → 1.0 にはならないが正の相関であること
        assert!(r > 0.9, "正の相関のはずが: {}", r);
    }

    #[test]
    fn tc_801_05_spearman_n_less_than_2_returns_zero() {
        // 【テスト目的】: n < 2 の場合は 0.0 を返す（除算エラー回避）🟢
        let r1 = compute_spearman(&[], &[]);
        let r2 = compute_spearman(&[1.0], &[1.0]);

        assert_eq!(r1, 0.0, "空データは0.0のはず");
        assert_eq!(r2, 0.0, "n=1は0.0のはず");
    }

    // =========================================================================
    // Ridge 回帰 — 正常系
    // =========================================================================

    #[test]
    fn tc_801_06_ridge_perfect_linear_r_squared_near_1() {
        // 【テスト目的】: 完全線形データで R² ≈ 1.0 になる 🟢
        let n = 50;
        let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let y: Vec<f64> = (0..n).map(|i| 2.0 * i as f64 + 1.0).collect();

        let result = compute_ridge(&x_matrix, &y, 0.001);

        assert!(
            result.r_squared > 0.99,
            "完全線形データのR²は1.0に近いはず: {}",
            result.r_squared
        );
    }

    #[test]
    fn tc_801_07_ridge_beta_sign_correct() {
        // 【テスト目的】: 正の関係でβ係数の符号が正であること 🟢
        let n = 20;
        let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let y: Vec<f64> = (0..n).map(|i| 3.0 * i as f64).collect();

        let result = compute_ridge(&x_matrix, &y, 0.01);

        assert!(
            result.beta[0] > 0.0,
            "正の関係ではβ>0のはず: {}",
            result.beta[0]
        );
    }

    #[test]
    fn tc_801_08_ridge_two_params_identifies_stronger() {
        // 【テスト目的】: 2変数で強い関係のパラメータのβ絶対値が大きい 🟢
        let n = 50;
        // x1 は y と強い正相関、x2 は弱い相関
        let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (i % 5) as f64]).collect();
        let y: Vec<f64> = (0..n).map(|i| i as f64 + 0.1 * (i % 5) as f64).collect();

        let result = compute_ridge(&x_matrix, &y, 0.01);

        assert_eq!(result.beta.len(), 2, "β係数は2個のはず");
        assert!(
            result.beta[0].abs() > result.beta[1].abs(),
            "x1の感度がx2より大きいはず: beta={:?}",
            result.beta
        );
    }

    #[test]
    fn tc_801_09_ridge_empty_returns_zero_r_squared() {
        // 【テスト目的】: n < 2 の場合は空の RidgeResult を返す 🟢
        let result = compute_ridge(&[], &[], 1.0);

        assert_eq!(result.beta.len(), 0, "空データはβなし");
        assert_eq!(result.r_squared, 0.0, "空データはR²=0.0");
    }

    // =========================================================================
    // compute_sensitivity_all — DataFrame 対応
    // =========================================================================

    #[test]
    fn tc_801_10_sensitivity_all_correct_structure() {
        // 【テスト目的】: compute_sensitivity_all が P×M の相関行列を返す 🟢
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

        // 【確認】構造が正しいこと
        assert_eq!(result.param_names.len(), 2); // x1, x2
        assert_eq!(result.objective_names.len(), 2); // obj0, obj1
        assert_eq!(result.spearman.len(), 2); // 2パラメータ行
        assert_eq!(result.spearman[0].len(), 2); // 2目的列
        assert_eq!(result.ridge.len(), 2); // 2目的
    }

    #[test]
    fn tc_801_11_sensitivity_all_known_correlations() {
        // 【テスト目的】: x1↑→obj0↑、x2↑→obj0↓の相関符号が正しい 🟢
        let rows: Vec<TrialRow> = (0..20)
            .map(|i| {
                make_row_multi(
                    i,
                    &[("x1", i as f64), ("x2", (20 - i) as f64)],
                    vec![i as f64], // obj0 = x1 と完全正相関
                )
            })
            .collect();
        let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

        let result = compute_sensitivity_all(&df);

        // 【確認】x1 と obj0 の相関は正、x2 と obj0 の相関は負
        assert!(
            result.spearman[0][0] > 0.99,
            "x1-obj0は正相関のはず: {}",
            result.spearman[0][0]
        );
        assert!(
            result.spearman[1][0] < -0.99,
            "x2-obj0は負相関のはず: {}",
            result.spearman[1][0]
        );
    }

    // =========================================================================
    // compute_sensitivity_selected — サブセット計算
    // =========================================================================

    #[test]
    fn tc_801_12_sensitivity_selected_subset() {
        // 【テスト目的】: compute_sensitivity_selected が指定インデックスのサブセットで計算する 🟢
        let rows: Vec<TrialRow> = (0..20)
            .map(|i| make_row_multi(i, &[("x1", i as f64)], vec![i as f64]))
            .collect();
        setup_df(rows, &["x1"], &["obj0"]);

        // 先頭 10 件のサブセット (indices 0-9)
        let indices: Vec<u32> = (0..10).collect();
        let result = compute_sensitivity_selected(&indices).expect("結果があるはず");

        assert_eq!(result.param_names, vec!["x1"]);
        assert_eq!(result.objective_names, vec!["obj0"]);
        // サブセットでも x1-obj0 は完全正相関
        assert!(
            result.spearman[0][0] > 0.99,
            "サブセットでも正相関のはず: {}",
            result.spearman[0][0]
        );
    }

    #[test]
    fn tc_801_13_sensitivity_selected_empty_indices() {
        // 【テスト目的】: 空インデックスで空の SensitivityResult が返る 🟢
        let rows: Vec<TrialRow> = (0..5)
            .map(|i| make_row_multi(i, &[("x1", i as f64)], vec![i as f64]))
            .collect();
        setup_df(rows, &["x1"], &["obj0"]);

        let result = compute_sensitivity_selected(&[]).expect("結果があるはず");

        assert!(
            result.spearman.is_empty(),
            "空インデックスはspearman空のはず"
        );
    }

    // =========================================================================
    // パフォーマンステスト
    // =========================================================================

    #[test]
    fn tc_801_p01_spearman_50000_x_30_x_4_under_500ms() {
        // 【テスト目的】: Spearman を大規模データで 500ms 以内に完了 🟢
        // デバッグビルド: 5,000点×10パラメータ×4目的（≤500ms）
        // リリースビルド: 50,000点×30パラメータ×4目的（≤500ms）
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
            "Spearman計算が{}msかかった（期待: ≤500ms, n={}, params={}, objs={}）",
            elapsed.as_millis(),
            n,
            n_params,
            n_objs
        );
    }

    #[test]
    fn tc_801_p02_ridge_50000_x_30_under_300ms() {
        // 【テスト目的】: Ridge を大規模データで 300ms 以内に完了 🟢
        // デバッグビルド: 5,000点×10パラメータ×4目的（≤300ms）
        // リリースビルド: 50,000点×30パラメータ×4目的（≤300ms）
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
            "Ridge計算が{}msかかった（期待: ≤300ms, n={}, params={}）",
            elapsed.as_millis(),
            n,
            n_params
        );
    }

    #[test]
    fn tc_801_p03_sensitivity_selected_under_50ms() {
        // 【テスト目的】: compute_sensitivity_selected が 50ms 以内に完了 🟢
        // デバッグビルド: 5,000点×1パラメータ×4目的（≤50ms）
        // リリースビルド: 50,000点×1パラメータ×4目的（≤50ms）
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
            "compute_sensitivity_selected が {}ms かかった（期待: ≤50ms, n={}）",
            elapsed.as_millis(),
            n
        );
    }
}
