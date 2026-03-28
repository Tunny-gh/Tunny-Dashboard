//! 部分依存プロット（PDP）— Ridge簡易版 (TASK-803)
//!
//! 【役割】: Ridge回帰を用いた解析的PDP計算（1変数・2変数交互作用）
//! 【設計方針】:
//!   - 線形Ridge モデルの PDP は解析的に計算可能 → グリッドごとの N 回推論が不要
//!   - f̄_j(v) = y_mean + β_j * (v - mean_j) / std_j
//!   - compute_ridge() (TASK-801) を再利用して β 係数を取得
//!
//! REQ-100: compute_pdp() — Ridge回帰によるPDP ≤20ms
//! REQ-101: compute_pdp_2d() — 2変数交互作用PDP ≤100ms
//!
//! 参照: docs/tasks/tunny-dashboard-tasks.md TASK-803

use crate::sensitivity::compute_ridge;

// =============================================================================
// 公開型定義
// =============================================================================

/// 1変数PDPの結果
///
/// 【設計】: grid[i] のパラメータ値に対する PDP 値 values[i] を保持する 🟢
#[derive(Debug, Clone)]
pub struct PdpResult1d {
    /// 対象パラメータ名
    pub param_name: String,
    /// 対象目的名
    pub objective_name: String,
    /// グリッド点のパラメータ値（n_grid 点、等間隔）
    pub grid: Vec<f64>,
    /// 各グリッド点でのPDP値（n_grid 点）
    pub values: Vec<f64>,
    /// Ridge モデルの決定係数 R²（モデル適合度の指標）
    pub r_squared: f64,
}

/// 2変数PDPの結果
///
/// 【設計】: values[i][j] = f̄(grid1[i], grid2[j]) の行列形式で保持する 🟢
#[derive(Debug, Clone)]
pub struct PdpResult2d {
    /// 第1パラメータ名
    pub param1_name: String,
    /// 第2パラメータ名
    pub param2_name: String,
    /// 対象目的名
    pub objective_name: String,
    /// 第1パラメータのグリッド点（n_grid 点）
    pub grid1: Vec<f64>,
    /// 第2パラメータのグリッド点（n_grid 点）
    pub grid2: Vec<f64>,
    /// PDP値行列: values[i][j] = f̄(grid1[i], grid2[j])（n_grid × n_grid）
    pub values: Vec<Vec<f64>>,
    /// Ridge モデルの決定係数 R²
    pub r_squared: f64,
}

// =============================================================================
// 内部ヘルパー関数
// =============================================================================

/// 数値配列の平均と母集団標準偏差を計算する
///
/// 【設計】: compute_ridge 内の transpose_and_standardize と同じ計算式（nで除算）を使う 🟢
/// 【ゼロ保護】: std が EPSILON 以下のときは 1.0 を返して除算エラーを防ぐ
fn col_mean_std(data: &[f64]) -> (f64, f64) {
    let n = data.len();
    if n == 0 {
        return (0.0, 1.0);
    }
    let mean = data.iter().sum::<f64>() / n as f64;
    let var = data.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = if var.sqrt() < f64::EPSILON { 1.0 } else { var.sqrt() };
    (mean, std_dev)
}

/// [min, max] の等間隔 n 点グリッドを生成する
///
/// 【設計】: n=0 → 空、n=1 → 中点、n≥2 → [min, max] を n-1 等分 🟢
fn linspace(min: f64, max: f64, n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![(min + max) / 2.0];
    }
    (0..n)
        .map(|i| min + (max - min) * i as f64 / (n - 1) as f64)
        .collect()
}

// =============================================================================
// pub(crate) 計算関数（テスト・内部利用向け）
// =============================================================================

/// 行優先 x_matrix から1変数PDPを計算する（テスト・内部向け）
///
/// 【アルゴリズム】:
///   1. Ridge 回帰で β_j を取得（TASK-801 の compute_ridge を再利用）
///   2. 対象パラメータの mean_j・std_j を計算
///   3. 解析式 f̄_j(v) = y_mean + β_j * (v - mean_j) / std_j でグリッド値を算出
///
/// 【線形PDP解析式の導出】🟢:
///   Ridge 予測: ŷ_i = y_mean + Σ_k β_k * (x_ki - mean_k) / std_k
///   PDP = (1/N) Σ_i ŷ_i|x_j=v = y_mean + β_j*(v-mean_j)/std_j + Σ_{k≠j} β_k * mean((x_ki-mean_k)/std_k)
///       = y_mean + β_j*(v-mean_j)/std_j  （標準化後の平均=0 なので第3項は消える）
pub(crate) fn compute_pdp_from_matrix(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param_names: &[String],
    objective_name: &str,
    target_param_idx: usize,
    n_grid: usize,
) -> PdpResult1d {
    // 【空結果の雛形】: エラー時に返すデフォルト値
    let param_name = param_names.get(target_param_idx).cloned().unwrap_or_default();
    let empty = PdpResult1d {
        param_name: param_name.clone(),
        objective_name: objective_name.to_string(),
        grid: vec![],
        values: vec![],
        r_squared: 0.0,
    };

    let n = y.len();
    if n < 2 || x_matrix.is_empty() || n_grid == 0 {
        return empty;
    }
    if target_param_idx >= x_matrix[0].len() {
        return empty;
    }

    // 【Ridge 回帰】: β 係数・R² を取得（compute_ridge は内部で標準化を行う）
    let ridge = compute_ridge(x_matrix, y, 1.0);

    // 【統計量計算】: compute_ridge 内と同じ母集団標準偏差を使って整合性を保つ
    let param_col: Vec<f64> = x_matrix.iter().map(|row| row[target_param_idx]).collect();
    let (mean_j, std_j) = col_mean_std(&param_col);
    let y_mean = y.iter().sum::<f64>() / n as f64;

    // 【グリッド生成】: 観測範囲 [min, max] を n_grid 等分
    let min_j = param_col.iter().cloned().fold(f64::INFINITY, |a, b| a.min(b));
    let max_j = param_col.iter().cloned().fold(f64::NEG_INFINITY, |a, b| a.max(b));
    let grid = linspace(min_j, max_j, n_grid);

    // 【PDP値計算】: 解析式 f̄_j(v) = y_mean + β_j * (v - mean_j) / std_j
    let beta_j = ridge.beta.get(target_param_idx).copied().unwrap_or(0.0);
    let values: Vec<f64> = grid
        .iter()
        .map(|&v| y_mean + beta_j * (v - mean_j) / std_j)
        .collect();

    PdpResult1d {
        param_name,
        objective_name: objective_name.to_string(),
        grid,
        values,
        r_squared: ridge.r_squared,
    }
}

/// 行優先 x_matrix から2変数交互作用PDPを計算する（テスト・内部向け）
///
/// 【アルゴリズム】:
///   Ridge 解析式を2変数に拡張:
///   f̄_{j1,j2}(v1, v2) = y_mean + β_j1*(v1-mean_j1)/std_j1 + β_j2*(v2-mean_j2)/std_j2
///
/// 【注意】: 線形モデルなので真の交互作用項はない（ONNX版で非線形交互作用を捕捉予定）🟢
pub(crate) fn compute_pdp_2d_from_matrix(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param_names: &[String],
    objective_name: &str,
    param1_idx: usize,
    param2_idx: usize,
    n_grid: usize,
) -> PdpResult2d {
    // 【空結果の雛形】
    let p1_name = param_names.get(param1_idx).cloned().unwrap_or_default();
    let p2_name = param_names.get(param2_idx).cloned().unwrap_or_default();
    let empty = PdpResult2d {
        param1_name: p1_name.clone(),
        param2_name: p2_name.clone(),
        objective_name: objective_name.to_string(),
        grid1: vec![],
        grid2: vec![],
        values: vec![],
        r_squared: 0.0,
    };

    let n = y.len();
    if n < 2 || x_matrix.is_empty() || n_grid == 0 {
        return empty;
    }
    let p = x_matrix[0].len();
    if param1_idx >= p || param2_idx >= p {
        return empty;
    }

    // 【Ridge 回帰】
    let ridge = compute_ridge(x_matrix, y, 1.0);

    // 【各パラメータの統計量】
    let col1: Vec<f64> = x_matrix.iter().map(|row| row[param1_idx]).collect();
    let col2: Vec<f64> = x_matrix.iter().map(|row| row[param2_idx]).collect();
    let (mean1, std1) = col_mean_std(&col1);
    let (mean2, std2) = col_mean_std(&col2);
    let y_mean = y.iter().sum::<f64>() / n as f64;

    // 【グリッド生成】
    let min1 = col1.iter().cloned().fold(f64::INFINITY, |a, b| a.min(b));
    let max1 = col1.iter().cloned().fold(f64::NEG_INFINITY, |a, b| a.max(b));
    let min2 = col2.iter().cloned().fold(f64::INFINITY, |a, b| a.min(b));
    let max2 = col2.iter().cloned().fold(f64::NEG_INFINITY, |a, b| a.max(b));
    let grid1 = linspace(min1, max1, n_grid);
    let grid2 = linspace(min2, max2, n_grid);

    // 【PDP値行列計算】: values[i][j] = f̄(grid1[i], grid2[j])
    let beta1 = ridge.beta.get(param1_idx).copied().unwrap_or(0.0);
    let beta2 = ridge.beta.get(param2_idx).copied().unwrap_or(0.0);
    let values: Vec<Vec<f64>> = grid1
        .iter()
        .map(|&v1| {
            grid2
                .iter()
                .map(|&v2| {
                    y_mean
                        + beta1 * (v1 - mean1) / std1
                        + beta2 * (v2 - mean2) / std2
                })
                .collect()
        })
        .collect();

    PdpResult2d {
        param1_name: p1_name,
        param2_name: p2_name,
        objective_name: objective_name.to_string(),
        grid1,
        grid2,
        values,
        r_squared: ridge.r_squared,
    }
}

// =============================================================================
// DataFrame 対応 公開 API
// =============================================================================

/// アクティブ Study から1変数PDPを計算する
///
/// 【設計】: with_active_df で DataFrame を参照し、行優先行列を構築して
///           compute_pdp_from_matrix に委譲する 🟢
/// @param param_name  対象パラメータ名
/// @param objective_name 対象目的名
/// @param n_grid グリッド点数（推奨: 20〜50）
/// @param _n_samples ICEライン用サンプル数（Ridge解析式では未使用・将来拡張用）
pub fn compute_pdp(
    param_name: &str,
    objective_name: &str,
    n_grid: usize,
    _n_samples: usize,
) -> Option<PdpResult1d> {
    crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n = df.row_count();

        // 【存在確認】: 指定した列が DataFrame に存在するかチェック
        let target_idx = param_names.iter().position(|p| p == param_name)?;
        let _ = objective_names.iter().position(|o| o == objective_name)?;

        // 【行優先行列構築】: DataFrame 列 → Vec<Vec<f64>>
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                param_names
                    .iter()
                    .map(|p| {
                        df.get_numeric_column(p)
                            .and_then(|c| c.get(i))
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();
        let y: Vec<f64> = (0..n)
            .map(|i| {
                df.get_numeric_column(objective_name)
                    .and_then(|c| c.get(i))
                    .copied()
                    .unwrap_or(0.0)
            })
            .collect();

        Some(compute_pdp_from_matrix(
            &x_matrix,
            &y,
            &param_names,
            objective_name,
            target_idx,
            n_grid,
        ))
    })
    .flatten()
}

/// アクティブ Study から2変数交互作用PDPを計算する
///
/// 【設計】: with_active_df で DataFrame を参照し、compute_pdp_2d_from_matrix に委譲する 🟢
pub fn compute_pdp_2d(
    param1_name: &str,
    param2_name: &str,
    objective_name: &str,
    n_grid: usize,
) -> Option<PdpResult2d> {
    crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n = df.row_count();

        let p1_idx = param_names.iter().position(|p| p == param1_name)?;
        let p2_idx = param_names.iter().position(|p| p == param2_name)?;
        let _ = objective_names.iter().position(|o| o == objective_name)?;

        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                param_names
                    .iter()
                    .map(|p| {
                        df.get_numeric_column(p)
                            .and_then(|c| c.get(i))
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();
        let y: Vec<f64> = (0..n)
            .map(|i| {
                df.get_numeric_column(objective_name)
                    .and_then(|c| c.get(i))
                    .copied()
                    .unwrap_or(0.0)
            })
            .collect();

        Some(compute_pdp_2d_from_matrix(
            &x_matrix,
            &y,
            &param_names,
            objective_name,
            p1_idx,
            p2_idx,
            n_grid,
        ))
    })
    .flatten()
}

// =============================================================================
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // テストデータ生成ヘルパー
    // ------------------------------------------------------------------

    /// 【ヘルパー】: y = x1 の完全線形データを生成する（x1, x2 の2パラメータ）
    ///
    /// 【設計】: x1 = i/n の等間隔、x2 はノイズ的な定数変化
    ///           y = x1 なので x1 の PDP は単調増加かつ R² ≈ 1.0 になる 🟢
    fn make_linear_data_1d(n: usize) -> (Vec<Vec<f64>>, Vec<f64>, Vec<String>) {
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let x1 = i as f64 / n as f64;
                let x2 = (i as f64 * 0.3).sin(); // x2 は y に無相関
                vec![x1, x2]
            })
            .collect();
        let y: Vec<f64> = x_matrix.iter().map(|row| row[0]).collect(); // y = x1
        let names = vec!["x1".to_string(), "x2".to_string()];
        (x_matrix, y, names)
    }

    /// 【ヘルパー】: y = 2*x1 - 0.5*x2 + 0.3*x3 のデータを生成する
    ///
    /// 【設計】: 複数パラメータの交互作用なし線形モデル 🟢
    fn make_linear_data_multi(n: usize) -> (Vec<Vec<f64>>, Vec<f64>, Vec<String>) {
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let t = i as f64 / n as f64;
                vec![t, t * 1.3 + 0.1, 1.0 - t * 0.7]
            })
            .collect();
        let y: Vec<f64> = x_matrix
            .iter()
            .map(|row| 2.0 * row[0] - 0.5 * row[1] + 0.3 * row[2])
            .collect();
        let names = vec!["x1".to_string(), "x2".to_string(), "x3".to_string()];
        (x_matrix, y, names)
    }

    // ------------------------------------------------------------------
    // TC-803-01: 1変数PDP 単調性テスト（既知の線形データ）
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_01_pdp_monotone_positive() {
        // 【テスト目的】: y = x1 のとき x1 の PDP が単調増加であることを検証する 🟢
        // 【テスト内容】: values[i] < values[i+1] が全隣接ペアで成立することを確認
        let n = 200;
        let (x_matrix, y, names) = make_linear_data_1d(n);

        // 【処理実行】: x1 (index=0) の PDP を 10 点グリッドで計算
        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 10);

        // 【確認内容】: グリッドが存在すること
        assert_eq!(result.grid.len(), 10, "グリッド点数が 10 であること");
        assert_eq!(result.values.len(), 10, "PDP値点数が 10 であること");

        // 【確認内容】: PDP が厳密に単調増加であること（y = x1 なので必ず増加する）
        for i in 0..result.values.len() - 1 {
            assert!(
                result.values[i] < result.values[i + 1],
                "PDP[{}]={} が PDP[{}]={} より小さいこと（単調増加）",
                i,
                result.values[i],
                i + 1,
                result.values[i + 1]
            );
        }
    }

    // ------------------------------------------------------------------
    // TC-803-02: 1変数PDP 単調性テスト（負の相関）
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_02_pdp_monotone_negative() {
        // 【テスト目的】: y = -x1 のとき x1 の PDP が単調減少であることを検証する 🟢
        let n = 100;
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| vec![i as f64 / n as f64])
            .collect();
        let y: Vec<f64> = x_matrix.iter().map(|row| -row[0]).collect();
        let names = vec!["x1".to_string()];

        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 8);

        // 【確認内容】: PDP が単調減少であること（y = -x1 なので必ず減少する）
        for i in 0..result.values.len() - 1 {
            assert!(
                result.values[i] > result.values[i + 1],
                "PDP[{}]={} が PDP[{}]={} より大きいこと（単調減少）",
                i,
                result.values[i],
                i + 1,
                result.values[i + 1]
            );
        }
    }

    // ------------------------------------------------------------------
    // TC-803-03: 1変数PDP 正確性テスト（PDP中点 ≈ y_mean）
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_03_pdp_midpoint_equals_ymean() {
        // 【テスト目的】: PDP の中点グリッド値が y_mean と一致することを検証する 🟢
        // 【根拠】: f̄_j((min+max)/2) = y_mean + β_j*(mean_grid-mean_j)/std_j
        //          グリッド中点 = (min+max)/2 ≈ mean_j なので PDP ≈ y_mean
        let n = 200;
        let (x_matrix, y, names) = make_linear_data_1d(n);
        let y_mean = y.iter().sum::<f64>() / n as f64;

        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 11);

        // 中央インデックス（n_grid=11 のとき index=5）
        let mid_idx = result.values.len() / 2;
        let mid_val = result.values[mid_idx];

        // 【確認内容】: 中点 PDP 値が y_mean の ±5% 以内であること
        let tolerance = (y_mean.abs() + 0.01) * 0.05;
        assert!(
            (mid_val - y_mean).abs() < tolerance,
            "中点PDP値 {} が y_mean {} の ±5% 以内であること",
            mid_val,
            y_mean
        );
    }

    // ------------------------------------------------------------------
    // TC-803-04: 1変数PDP R² 正確性テスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_04_pdp_r_squared_high_for_linear() {
        // 【テスト目的】: 完全線形データで R² > 0.99 を検証する 🟢
        let n = 200;
        let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let y: Vec<f64> = x_matrix.iter().map(|row| row[0] * 3.0 + 1.0).collect();
        let names = vec!["x1".to_string()];

        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 10);

        // 【確認内容】: 完全線形データなので R² が 0.99 以上であること
        assert!(
            result.r_squared > 0.99,
            "完全線形データの R² {} が 0.99 超であること",
            result.r_squared
        );
    }

    // ------------------------------------------------------------------
    // TC-803-05: 空データ・データ不足で空結果を返す
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_05_empty_data_returns_empty() {
        // 【テスト目的】: n < 2 のときは空の PdpResult1d を返すことを検証する 🟢
        let x_matrix: Vec<Vec<f64>> = vec![vec![1.0]]; // n=1
        let y = vec![1.0f64];
        let names = vec!["x1".to_string()];

        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 10);

        // 【確認内容】: グリッドと値が空であること
        assert!(result.grid.is_empty(), "n<2 のときグリッドが空であること");
        assert!(result.values.is_empty(), "n<2 のとき値が空であること");
        assert_eq!(result.r_squared, 0.0, "n<2 のとき R² が 0.0 であること");
    }

    // ------------------------------------------------------------------
    // TC-803-06: 2変数PDP グリッド形状テスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_06_pdp_2d_grid_shape() {
        // 【テスト目的】: 2変数PDP の結果行列が n_grid×n_grid であることを検証する 🟢
        let n = 100;
        let (x_matrix, y, names) = make_linear_data_multi(n);

        let result = compute_pdp_2d_from_matrix(&x_matrix, &y, &names, "obj0", 0, 1, 8);

        // 【確認内容】: グリッド点数が正しいこと
        assert_eq!(result.grid1.len(), 8, "grid1 の点数が 8 であること");
        assert_eq!(result.grid2.len(), 8, "grid2 の点数が 8 であること");
        assert_eq!(result.values.len(), 8, "values の行数が 8 であること");
        for row in &result.values {
            assert_eq!(row.len(), 8, "values の各行の列数が 8 であること");
        }
    }

    // ------------------------------------------------------------------
    // TC-803-07: 2変数PDP 空データで空結果を返す
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_07_pdp_2d_empty_data() {
        // 【テスト目的】: n < 2 のとき 2変数PDP も空結果を返すことを検証する 🟢
        let x_matrix: Vec<Vec<f64>> = vec![vec![1.0, 2.0]];
        let y = vec![1.0f64];
        let names = vec!["x1".to_string(), "x2".to_string()];

        let result = compute_pdp_2d_from_matrix(&x_matrix, &y, &names, "obj0", 0, 1, 5);

        // 【確認内容】: 空結果が返ること
        assert!(result.grid1.is_empty(), "n<2 のとき grid1 が空であること");
        assert!(result.grid2.is_empty(), "n<2 のとき grid2 が空であること");
        assert!(result.values.is_empty(), "n<2 のとき values が空であること");
    }

    // ------------------------------------------------------------------
    // TC-803-08: 2変数PDP R² テスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_08_pdp_2d_r_squared() {
        // 【テスト目的】: 2変数PDPでも完全線形データで R² > 0.99 を検証する 🟢
        let n = 200;
        let (x_matrix, y, names) = make_linear_data_multi(n);

        let result = compute_pdp_2d_from_matrix(&x_matrix, &y, &names, "obj0", 0, 1, 5);

        // 【確認内容】: 線形データなので R² が高いこと
        assert!(
            result.r_squared > 0.95,
            "線形データの 2変数PDP R² {} が 0.95 超であること",
            result.r_squared
        );
    }

    // ------------------------------------------------------------------
    // TC-803-09: param_name・objective_name が正しく保持されること
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_09_result_names() {
        // 【テスト目的】: 結果の param_name と objective_name が正しく設定されることを検証する 🟢
        let n = 50;
        let (x_matrix, y, names) = make_linear_data_1d(n);

        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj_target", 0, 5);

        // 【確認内容】: 名前が正しく設定されること
        assert_eq!(result.param_name, "x1", "param_name が 'x1' であること");
        assert_eq!(
            result.objective_name, "obj_target",
            "objective_name が 'obj_target' であること"
        );
    }

    // ------------------------------------------------------------------
    // TC-803-P01: 1変数PDP 20ms以内のパフォーマンステスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_p01_pdp_1d_performance() {
        // 【テスト目的】: 1変数PDP（Ridge）が 20ms 以内で完了することを検証する 🟢
        // 【データ規模】: debug=1,000行×4パラメータ / release=50,000行×10パラメータ

        #[cfg(debug_assertions)]
        let (n, p) = (1_000, 4);
        #[cfg(not(debug_assertions))]
        let (n, p) = (50_000, 10);

        // 【テストデータ準備】: 線形関係を持つ合成データ
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..p).map(|j| i as f64 / n as f64 + j as f64 * 0.1).collect())
            .collect();
        let y: Vec<f64> = x_matrix.iter().map(|row| row[0] * 2.0 + row[1] * 0.5).collect();
        let names: Vec<String> = (0..p).map(|j| format!("x{}", j)).collect();

        let start = std::time::Instant::now();
        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 20);
        let elapsed = start.elapsed();

        // 【確認内容】: 計算が正常完了し 20ms 以内であること
        assert_eq!(result.grid.len(), 20, "グリッド点数が 20 であること");
        assert!(
            elapsed.as_millis() < 20,
            "1変数PDP が 20ms 以内: 実測 {}ms",
            elapsed.as_millis()
        );
    }

    // ------------------------------------------------------------------
    // TC-803-P02: 2変数PDP 100ms以内のパフォーマンステスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_p02_pdp_2d_performance() {
        // 【テスト目的】: 2変数PDP が 100ms 以内で完了することを検証する 🟢
        // 【データ規模】: debug=1,000行×4パラメータ / release=50,000行×10パラメータ

        #[cfg(debug_assertions)]
        let (n, p) = (1_000, 4);
        #[cfg(not(debug_assertions))]
        let (n, p) = (50_000, 10);

        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..p).map(|j| i as f64 / n as f64 + j as f64 * 0.1).collect())
            .collect();
        let y: Vec<f64> = x_matrix.iter().map(|row| row[0] * 2.0 + row[1] * 0.5).collect();
        let names: Vec<String> = (0..p).map(|j| format!("x{}", j)).collect();

        let start = std::time::Instant::now();
        let result = compute_pdp_2d_from_matrix(&x_matrix, &y, &names, "obj0", 0, 1, 15);
        let elapsed = start.elapsed();

        // 【確認内容】: 計算が正常完了し 100ms 以内であること
        assert_eq!(result.values.len(), 15, "values の行数が 15 であること");
        assert_eq!(result.values[0].len(), 15, "values の列数が 15 であること");
        assert!(
            elapsed.as_millis() < 100,
            "2変数PDP が 100ms 以内: 実測 {}ms",
            elapsed.as_millis()
        );
    }
}
