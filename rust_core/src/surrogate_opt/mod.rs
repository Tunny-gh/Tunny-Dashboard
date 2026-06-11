//! 応答曲面（サロゲートモデル）の学習と、その曲面上での最適化。
//!
//! サンプリング結果（trial 群）からサロゲートモデルを学習し、正規化 [0,1]^d 箱内で
//! 最適化を実行して推定最適点を返す。モデル・最適化手法はそれぞれ
//! [`SurrogateModelKind`] / [`OptimizerKind`] へバリアントを追加することで拡張する。

mod models;
mod optimizers;
pub(crate) mod validation;

pub use models::SurrogateModelKind;
pub use optimizers::OptimizerKind;
pub use validation::SurrogateValidationReport;

use crate::math::grid::linspace;
use validation::validate_surrogate;

/// サロゲート学習に必要な最小 trial 数。
pub const MIN_TRIALS_FOR_SURROGATE_OPT: usize = 10;

/// スライス格子のデフォルト解像度。
pub const DEFAULT_SLICE_GRID: usize = 20;

/// サロゲート最適化の入力。
pub struct SurrogateOptRequest {
    /// 訓練データ（行 = trial、列 = パラメータ）。元の単位。
    pub x_matrix: Vec<Vec<f64>>,
    /// 目的値（元の単位）。
    pub y: Vec<f64>,
    /// 各パラメータ列の名前（結果の `best_params` と同順）。
    pub param_names: Vec<String>,
    /// 目的の名前（表示用）。
    pub objective_name: String,
    /// true = 最小化、false = 最大化。
    pub minimize: bool,
    /// 使用するサロゲートモデル。
    pub model: SurrogateModelKind,
    /// 使用する最適化手法。
    pub optimizer: OptimizerKind,
    /// 最適点を通る応答曲面スライスを返す 2 パラメータの列 index（表示用）。
    pub slice_params: Option<(usize, usize)>,
    /// スライス格子の一辺の点数。
    pub n_grid: usize,
}

/// サロゲートの学習＋検証の入力。
pub struct SurrogateFitRequest {
    pub x_matrix: Vec<Vec<f64>>,
    pub y: Vec<f64>,
    pub param_names: Vec<String>,
    pub objective_name: String,
    pub model: SurrogateModelKind,
}

/// 検証済みの学習結果。最適化で再利用する。
pub struct TrainedSurrogate {
    pub(crate) surrogate: models::FittedSurrogate,
    pub model_kind: SurrogateModelKind,
    pub param_names: Vec<String>,
    pub objective_name: String,
    /// 学習に使った元データ（最適化の開始点に使用）。
    pub(crate) x_matrix: Vec<Vec<f64>>,
    pub(crate) y: Vec<f64>,
    pub validation: SurrogateValidationReport,
}

/// 最適化ステージの設定（学習済みモデルに対して実行する）。
pub struct SurrogateOptimizeSpec {
    pub minimize: bool,
    pub optimizer: OptimizerKind,
    pub slice_params: Option<(usize, usize)>,
    pub n_grid: usize,
}

/// 最適点を通る応答曲面の 2D スライス（他次元は最適点に固定）。
#[derive(Debug, Clone)]
pub struct SurfaceSlice {
    pub param_x_idx: usize,
    pub param_y_idx: usize,
    /// X 軸の格子値（元の単位）。
    pub x_values: Vec<f64>,
    /// Y 軸の格子値（元の単位）。
    pub y_values: Vec<f64>,
    /// 予測値格子。`z_values[i][j] = f(x_values[i], y_values[j])`。
    pub z_values: Vec<Vec<f64>>,
}

/// サロゲート最適化の結果。
#[derive(Debug, Clone)]
pub struct SurrogateOptResult {
    /// 推定最適点のパラメータ値（元の単位、`param_names` と同順）。
    pub best_params: Vec<f64>,
    /// 推定最適点でのサロゲート予測値（元の単位）。
    pub best_value: f64,
    /// 予測標準偏差（Kriging 系のみ。Ridge は None）。
    pub predicted_std: Option<f64>,
    /// 訓練データに対するサロゲートの決定係数。
    pub r_squared: f64,
    /// 最適点を通る応答曲面スライス（`slice_params` 指定時のみ）。
    pub slice: Option<SurfaceSlice>,
}

/// 入力の共通バリデーションを行う（成功時は (n, n_dims) を返す）。
fn validate_inputs(x_matrix: &[Vec<f64>], y: &[f64]) -> Result<(usize, usize), String> {
    let n = y.len();
    let n_dims = x_matrix.first().map(|r| r.len()).unwrap_or(0);

    if n < MIN_TRIALS_FOR_SURROGATE_OPT {
        return Err(format!(
            "At least {} trials required (current: {})",
            MIN_TRIALS_FOR_SURROGATE_OPT, n
        ));
    }
    if x_matrix.len() != n {
        return Err("x_matrix and y length mismatch".to_string());
    }
    if n_dims == 0 {
        return Err("No numeric parameters available".to_string());
    }
    if x_matrix.iter().any(|row| row.len() != n_dims) {
        return Err("x_matrix rows have inconsistent dimensions".to_string());
    }
    if x_matrix
        .iter()
        .flatten()
        .chain(y.iter())
        .any(|v| !v.is_finite())
    {
        return Err("Input contains non-finite values".to_string());
    }
    Ok((n, n_dims))
}

/// 学習済みサロゲートに対して最適化を実行し、結果を返す共通ロジック。
fn run_optimize(
    surrogate: &models::FittedSurrogate,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    minimize: bool,
    optimizer: OptimizerKind,
    slice_params: Option<(usize, usize)>,
    n_grid: usize,
) -> SurrogateOptResult {
    let n_dims = x_matrix.first().map(|r| r.len()).unwrap_or(0);

    // 観測ベスト点（最適化のスタート点に使う）。
    let best_observed_idx = best_observed_index(y, minimize);
    let start_norm = surrogate.to_norm_x(&x_matrix[best_observed_idx]);

    let t_best = optimizers::minimize_on_surrogate(surrogate, minimize, optimizer, &start_norm);

    let best_value = surrogate.to_original_y(surrogate.predict_norm(&t_best));
    let predicted_std = surrogate
        .predict_var_norm(&t_best)
        .map(|v| v.max(0.0).sqrt() * surrogate.y_std);

    let slice = slice_params
        .and_then(|(px, py)| build_slice(surrogate, &t_best, px, py, n_grid.max(2), n_dims));

    SurrogateOptResult {
        best_params: surrogate.to_original_x(&t_best),
        best_value,
        predicted_std,
        r_squared: surrogate.r_squared,
        slice,
    }
}

/// サロゲートを学習し、ホールドアウト＋k-fold CV で検証した結果を返す。
///
/// 検証シードは 42 を使用する。
pub fn fit_surrogate_with_validation(
    req: &SurrogateFitRequest,
) -> Result<TrainedSurrogate, String> {
    validate_inputs(&req.x_matrix, &req.y)?;

    // CV・ホールドアウト検証を実施する。
    let mut report = validate_surrogate(req.model, &req.x_matrix, &req.y, 42)?;

    // 全データで最終モデルを学習する。
    let surrogate = models::fit_surrogate(req.model, &req.x_matrix, &req.y)?;

    // 全データ訓練 R² を最終モデルから設定する。
    report.train_r2 = surrogate.r_squared;

    Ok(TrainedSurrogate {
        surrogate,
        model_kind: req.model,
        param_names: req.param_names.clone(),
        objective_name: req.objective_name.clone(),
        x_matrix: req.x_matrix.clone(),
        y: req.y.clone(),
        validation: report,
    })
}

/// 学習済みサロゲートモデルに対して最適化を実行する。
pub fn optimize_on_trained(
    trained: &TrainedSurrogate,
    spec: &SurrogateOptimizeSpec,
) -> SurrogateOptResult {
    run_optimize(
        &trained.surrogate,
        &trained.x_matrix,
        &trained.y,
        spec.minimize,
        spec.optimizer,
        spec.slice_params,
        spec.n_grid,
    )
}

/// サロゲートモデルを学習し、その曲面上で最適化を実行する。
///
/// バックグラウンドスレッドから呼べるよう、スレッドローカルの DataFrame には依存しない。
pub fn run_surrogate_optimization(req: &SurrogateOptRequest) -> Result<SurrogateOptResult, String> {
    validate_inputs(&req.x_matrix, &req.y)?;

    let surrogate = models::fit_surrogate(req.model, &req.x_matrix, &req.y)?;

    Ok(run_optimize(
        &surrogate,
        &req.x_matrix,
        &req.y,
        req.minimize,
        req.optimizer,
        req.slice_params,
        req.n_grid,
    ))
}

/// 観測値ベストの行 index（minimize なら最小、maximize なら最大）。
fn best_observed_index(y: &[f64], minimize: bool) -> usize {
    let mut best = 0usize;
    for (i, &v) in y.iter().enumerate() {
        let better = if minimize { v < y[best] } else { v > y[best] };
        if better {
            best = i;
        }
    }
    best
}

/// 最適点 `t_best`（正規化空間）を通る 2D スライス格子をサロゲートで評価する。
fn build_slice(
    surrogate: &models::FittedSurrogate,
    t_best: &[f64],
    param_x_idx: usize,
    param_y_idx: usize,
    n_grid: usize,
    n_dims: usize,
) -> Option<SurfaceSlice> {
    if param_x_idx >= n_dims || param_y_idx >= n_dims || param_x_idx == param_y_idx {
        return None;
    }
    let (min_x, range_x) = surrogate.col_stats[param_x_idx];
    let (min_y, range_y) = surrogate.col_stats[param_y_idx];
    let x_values = linspace(min_x, min_x + range_x, n_grid);
    let y_values = linspace(min_y, min_y + range_y, n_grid);

    let z_values: Vec<Vec<f64>> = x_values
        .iter()
        .map(|&vx| {
            y_values
                .iter()
                .map(|&vy| {
                    let mut pt = t_best.to_vec();
                    pt[param_x_idx] = (vx - min_x) / range_x;
                    pt[param_y_idx] = (vy - min_y) / range_y;
                    surrogate.to_original_y(surrogate.predict_norm(&pt))
                })
                .collect()
        })
        .collect();

    Some(SurfaceSlice {
        param_x_idx,
        param_y_idx,
        x_values,
        y_values,
        z_values,
    })
}

#[cfg(test)]
mod tests;
