//! 応答曲面（サロゲートモデル）の学習と、その曲面上での最適化。
//!
//! サンプリング結果（trial 群）からサロゲートモデルを学習し、正規化 [0,1]^d 箱内で
//! 最適化を実行して推定最適点を返す。モデル・最適化手法はそれぞれ
//! [`SurrogateModelKind`] / [`OptimizerKind`] へバリアントを追加することで拡張する。

mod acquisition;
mod ehvi;
pub(crate) mod feasibility;
mod models;
mod optimizers;
pub(crate) mod validation;

pub use acquisition::{suggest_candidates, AcquisitionKind, SuggestedCandidate};
pub use ehvi::{suggest_candidates_multi, MultiSuggestedCandidate};
pub use models::SurrogateModelKind;
pub use optimizers::OptimizerKind;
pub use validation::SurrogateValidationReport;

use crate::math::grid::linspace;
use validation::validate_surrogate;

/// サロゲート学習に必要な最小 trial 数。
pub const MIN_TRIALS_FOR_SURROGATE_OPT: usize = 10;

/// 自動モデル選択（Auto）の候補モデル。CV R² が最も高い候補を選ぶ。
///
/// 並び順は「単純・低コストなモデルを先頭」に揃えてある（Ridge → GP-FITC → GP-VFE →
/// LightGBM）。同点時はこの並びで先に来る候補を優先する（タイブレーク）。
///
/// GpMoe は候補から除外している:
///   - クラスタ数を CV で探索するため、候補ごとの検証コストが他より大幅に高い。
///   - 滑らか／線形なデータでは単一 GP に劣化し（クラスタが退化する）、Auto の
///     コスト対効果が悪い。MoE は不連続・多峰応答が分かっているときに手動で選ぶ想定。
pub const AUTO_CANDIDATES: [SurrogateModelKind; 4] = [
    SurrogateModelKind::Ridge,
    SurrogateModelKind::GpFitc,
    SurrogateModelKind::GpVfe,
    SurrogateModelKind::Lgbm,
];

/// 自動モデル選択（Auto）の結果。選ばれたモデルと候補ごとの CV R² を持つ。
#[derive(Debug, Clone)]
pub struct ModelSelectionReport {
    /// 選択されたモデル種別（最も高い CV R²、同点は AUTO_CANDIDATES の先頭優先）。
    pub chosen: SurrogateModelKind,
    /// 候補ごとの (モデル種別, スコア = cv_r2_mean)。`AUTO_CANDIDATES` と同順。
    /// フィット／検証に失敗した候補は f64::NEG_INFINITY を記録し、選択対象から外す。
    pub scores: Vec<(SurrogateModelKind, f64)>,
}

/// `AUTO_CANDIDATES` を交差検証し、CV R² が最も高いモデルを選ぶ。
///
/// 各候補について [`validate_surrogate`] を実行し、`cv_r2_mean` をスコアとする。
/// スコア差が 1e-3 未満の候補は「同点」とみなし、`AUTO_CANDIDATES` で先に来る候補
/// （より単純・低コスト）を優先する。全候補が失敗した場合のみ `Err` を返す。
pub fn select_best_model(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    seed: u64,
) -> Result<ModelSelectionReport, String> {
    validate_inputs(x_matrix, y)?;

    let mut scores: Vec<(SurrogateModelKind, f64)> = Vec::with_capacity(AUTO_CANDIDATES.len());
    for &kind in &AUTO_CANDIDATES {
        // フィット／検証に失敗した候補は NEG_INFINITY を記録し、選択対象から外す。
        let score = match validate_surrogate(kind, x_matrix, y, seed) {
            Ok(report) => report.cv_r2_mean,
            Err(_) => f64::NEG_INFINITY,
        };
        scores.push((kind, score));
    }

    // CV R² の差がこの値未満の候補は「同点」とみなし、AUTO_CANDIDATES で先に来る
    // （より単純・低コストな）候補を優先する。完全に線形なデータでは GP も Ridge も
    // ほぼ完璧に当てはまる（R² ≈ 1）ため、わずかな差で複雑な GP を選ばないようにする。
    const TIE_TOLERANCE: f64 = 1e-3;

    // 最大スコアの候補を選ぶ。AUTO_CANDIDATES の先頭から走査し、許容差を超えて
    // 大きいものだけ採用することで、同点は先（より単純）に来る候補が残る。
    let mut chosen: Option<(SurrogateModelKind, f64)> = None;
    for &(kind, score) in &scores {
        if !score.is_finite() {
            continue;
        }
        match chosen {
            Some((_, best)) if score <= best + TIE_TOLERANCE => {}
            _ => chosen = Some((kind, score)),
        }
    }

    let chosen = chosen
        .map(|(kind, _)| kind)
        .ok_or_else(|| "All candidate models failed validation".to_string())?;

    Ok(ModelSelectionReport { chosen, scores })
}

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
    /// 制約データ（空 = 制約なし）。
    pub constraints: Vec<ConstraintData>,
}

/// サロゲート学習に渡す 1 制約のデータ。
///
/// Optuna の制約規約: 値 ≤ 0 が実行可能（feasible）。
pub struct ConstraintData {
    /// 制約の名前（表示・ログ用）。
    pub name: String,
    /// 各 trial の制約値（`x_matrix` と同じ行順）。
    pub values: Vec<f64>,
}

/// サロゲートの学習＋検証の入力。
pub struct SurrogateFitRequest {
    pub x_matrix: Vec<Vec<f64>>,
    pub y: Vec<f64>,
    pub param_names: Vec<String>,
    pub objective_name: String,
    pub model: SurrogateModelKind,
    /// true のとき `model` を無視して `AUTO_CANDIDATES` を交差検証し、最良モデルを
    /// 自動選択して学習する（`TrainedSurrogate.model_selection` に経緯を残す）。
    pub auto_select: bool,
    /// 制約データ（空 = 制約なし）。各要素が 1 制約を表す。
    pub constraints: Vec<ConstraintData>,
    /// 誘導点として優先する行 index（`x_matrix` への index）。空 = 一様（既定）。
    /// 多目的でパレートフロント上の trial に GP の誘導点を集中させるために使う。
    /// N が GP の誘導点上限（100）以下のときは効果がない（Z = X で全点を使う）。
    pub priority_rows: Vec<usize>,
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
    /// ARD 長さスケールから算出した相対パラメータ重要度（`param_names` と同順、合計 1.0）。
    ///
    /// GP（単一 SGP: FITC / VFE）のみ Some。MoE / Ridge / LightGBM は None。
    /// 重要度はモデルの入力次元（= `x_matrix` の列）に対応し、その列順は `param_names`
    /// と一致する（`fit_surrogate` は列順を入れ替えないため）。
    pub param_importance: Option<Vec<f64>>,
    /// 制約名（`constraint_models` と同順。空 = 制約なし）。
    pub constraint_names: Vec<String>,
    /// 制約ごとの学習済みサロゲート（`constraint_names` と同順）。
    pub(crate) constraint_models: Vec<models::FittedSurrogate>,
    /// 各 trial の制約値（行 = trial、列 = 制約; `constraint_names` と同順）。
    /// 実行可能インカンバントの計算に使う。
    pub(crate) constraint_values: Vec<Vec<f64>>,
    /// 自動モデル選択（`auto_select = true`）の経緯。手動指定時は None。
    /// `model_kind` には選ばれた具体的なモデル種別が入る。
    pub model_selection: Option<ModelSelectionReport>,
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
    /// 予測標準偏差の格子（元の単位、`z_values` と同形状）。
    /// 事後分散を持つモデル（GP 系）のみ Some。Ridge / LightGBM は None。
    pub z_std: Option<Vec<Vec<f64>>>,
}

/// サロゲート最適化の結果。
#[derive(Debug, Clone)]
pub struct SurrogateOptResult {
    /// 推定最適点のパラメータ値（元の単位、`param_names` と同順）。
    pub best_params: Vec<f64>,
    /// 推定最適点でのサロゲート予測値（元の単位）。
    pub best_value: f64,
    /// 予測標準偏差（ガウス過程系のみ。Ridge は None）。
    pub predicted_std: Option<f64>,
    /// 訓練データに対するサロゲートの決定係数。
    pub r_squared: f64,
    /// 最適点を通る応答曲面スライス（`slice_params` 指定時のみ）。
    pub slice: Option<SurfaceSlice>,
    /// 観測データ中のベスト値（元の単位）。最小化なら最小値、最大化なら最大値。
    pub best_observed_value: f64,
    /// 推定最適点での各制約サロゲートの予測値（元の単位、`constraint_names` と同順）。
    /// 制約なし（`constraint_names` が空）のときは空。
    pub predicted_constraints: Vec<f64>,
    /// 推定最適点での実行可能性確率（0.0〜1.0）。制約なしのときは None。
    pub feasibility_probability: Option<f64>,
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
///
/// `constraint_models` が空でないとき、コスト関数に制約ペナルティを加えて探索する。
#[allow(clippy::too_many_arguments)]
fn run_optimize(
    surrogate: &models::FittedSurrogate,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    minimize: bool,
    optimizer: OptimizerKind,
    slice_params: Option<(usize, usize)>,
    n_grid: usize,
    constraint_models: &[models::FittedSurrogate],
) -> SurrogateOptResult {
    let n_dims = x_matrix.first().map(|r| r.len()).unwrap_or(0);

    // 観測ベスト点（最適化のスタート点に使う）。
    let best_observed_idx = best_observed_index(y, minimize);
    let start_norm = surrogate.to_norm_x(&x_matrix[best_observed_idx]);

    let t_best = optimizers::minimize_on_surrogate(
        surrogate,
        minimize,
        optimizer,
        &start_norm,
        constraint_models,
    );

    let best_value = surrogate.to_original_y(surrogate.predict_norm(&t_best));
    let predicted_std = surrogate
        .predict_var_norm(&t_best)
        .map(|v| v.max(0.0).sqrt() * surrogate.y_std);

    let slice = slice_params
        .and_then(|(px, py)| build_slice(surrogate, &t_best, px, py, n_grid.max(2), n_dims));

    let best_observed_value = y[best_observed_idx];

    // 制約の予測値と実行可能性確率を計算する。
    let (predicted_constraints, feasibility_probability) = if constraint_models.is_empty() {
        (vec![], None)
    } else {
        let preds: Vec<f64> = constraint_models
            .iter()
            .map(|cm| cm.to_original_y(cm.predict_norm(&t_best)))
            .collect();
        let p_feas = feasibility::feasibility_probability(constraint_models, &t_best);
        (preds, Some(p_feas))
    };

    SurrogateOptResult {
        best_params: surrogate.to_original_x(&t_best),
        best_value,
        predicted_std,
        r_squared: surrogate.r_squared,
        slice,
        best_observed_value,
        predicted_constraints,
        feasibility_probability,
    }
}

/// サロゲートを学習し、ホールドアウト＋k-fold CV で検証した結果を返す。
///
/// 検証シードは 42 を使用する。制約モデルは CV なしで全データ学習する。
pub fn fit_surrogate_with_validation(
    req: &SurrogateFitRequest,
) -> Result<TrainedSurrogate, String> {
    validate_inputs(&req.x_matrix, &req.y)?;

    // Auto 選択時は AUTO_CANDIDATES を交差検証して最良モデルを決める。
    // 以降の学習・検証・制約モデルは選ばれた具体的なモデル種別で行う
    //（SurrogateModelKind に "Auto" バリアントはないため、自動的に整合する）。
    let (model_kind, model_selection) = if req.auto_select {
        let report = select_best_model(&req.x_matrix, &req.y, 42)?;
        let chosen = report.chosen;
        (chosen, Some(report))
    } else {
        (req.model, None)
    };

    // CV・ホールドアウト検証を実施する。
    let mut report = validate_surrogate(model_kind, &req.x_matrix, &req.y, 42)?;

    // 全データで最終モデルを学習する。優先行（パレートフロント等）があれば GP の
    // 誘導点をそこに集中させる。CV/ホールドアウト検証側は汎化性能の推定のため一様
    // 誘導点のままにする（validate_surrogate は priority を受け取らない）。
    let surrogate =
        models::fit_surrogate_with_priority(model_kind, &req.x_matrix, &req.y, &req.priority_rows)?;

    // 全データ訓練 R² を最終モデルから設定する。
    report.train_r2 = surrogate.r_squared;

    // ARD 長さスケールによるパラメータ重要度（GP のみ Some、param_names と同順）。
    let param_importance = surrogate.param_importance();

    // 制約ごとにサロゲートを学習する（CV なし、全データ）。
    let mut constraint_names = Vec::with_capacity(req.constraints.len());
    let mut constraint_models = Vec::with_capacity(req.constraints.len());
    let mut constraint_values: Vec<Vec<f64>> = Vec::with_capacity(req.x_matrix.len());
    for _ in 0..req.x_matrix.len() {
        constraint_values.push(Vec::with_capacity(req.constraints.len()));
    }

    for cd in &req.constraints {
        // 制約モデルは目的関数と同じモデル種別で学習する。GP 系なら事後分散から
        // 平滑な実行可能性確率 P(c ≤ 0) が得られ（制約境界付近の不確実性を考慮した
        // 探索ができる）、Ridge / LightGBM ならハード指標へフォールバックする
        // （feasibility::single_prob 参照）。
        // Auto 選択時も目的モデルと同じ「選ばれた」種別を制約モデルに使う。
        let cm = models::fit_constraint_surrogate(model_kind, &req.x_matrix, &cd.values)
            .map_err(|e| format!("Constraint '{}' fit failed: {}", cd.name, e))?;
        constraint_names.push(cd.name.clone());
        constraint_models.push(cm);
        for (i, &v) in cd.values.iter().enumerate() {
            if let Some(row) = constraint_values.get_mut(i) {
                row.push(v);
            }
        }
    }

    Ok(TrainedSurrogate {
        surrogate,
        model_kind,
        param_names: req.param_names.clone(),
        objective_name: req.objective_name.clone(),
        x_matrix: req.x_matrix.clone(),
        y: req.y.clone(),
        validation: report,
        param_importance,
        constraint_names,
        constraint_models,
        constraint_values,
        model_selection,
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
        &trained.constraint_models,
    )
}

/// サロゲートモデルを学習し、その曲面上で最適化を実行する。
///
/// バックグラウンドスレッドから呼べるよう、スレッドローカルの DataFrame には依存しない。
pub fn run_surrogate_optimization(req: &SurrogateOptRequest) -> Result<SurrogateOptResult, String> {
    validate_inputs(&req.x_matrix, &req.y)?;

    let surrogate = models::fit_surrogate(req.model, &req.x_matrix, &req.y)?;

    // 制約サロゲートを学習する（制約なしの場合は空 vec）。目的関数と同じモデル種別を使い、
    // GP 系なら平滑な実行可能性確率を、Ridge / LightGBM ならハード指標を用いる。
    let constraint_models: Vec<models::FittedSurrogate> = req
        .constraints
        .iter()
        .map(|cd| {
            models::fit_constraint_surrogate(req.model, &req.x_matrix, &cd.values)
                .map_err(|e| format!("Constraint '{}' fit failed: {}", cd.name, e))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(run_optimize(
        &surrogate,
        &req.x_matrix,
        &req.y,
        req.minimize,
        req.optimizer,
        req.slice_params,
        req.n_grid,
        &constraint_models,
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

    // 各格子点で平均（元の単位）と、可能なら事後分散から元単位の標準偏差を評価する。
    // z_std はモデルが事後分散を持つ（GP 系）ときのみ Some を保持する。
    let mut z_values: Vec<Vec<f64>> = Vec::with_capacity(x_values.len());
    let mut z_std_grid: Vec<Vec<f64>> = Vec::with_capacity(x_values.len());
    let mut has_std = true;
    for &vx in &x_values {
        let mut z_row = Vec::with_capacity(y_values.len());
        let mut std_row = Vec::with_capacity(y_values.len());
        for &vy in &y_values {
            let mut pt = t_best.to_vec();
            pt[param_x_idx] = (vx - min_x) / range_x;
            pt[param_y_idx] = (vy - min_y) / range_y;
            z_row.push(surrogate.to_original_y(surrogate.predict_norm(&pt)));
            match surrogate.predict_var_norm(&pt) {
                // 正規化空間の分散 → 元の単位の標準偏差（y_std 倍）。
                Some(var) => std_row.push(var.max(0.0).sqrt() * surrogate.y_std),
                None => has_std = false,
            }
        }
        z_values.push(z_row);
        z_std_grid.push(std_row);
    }
    let z_std = has_std.then_some(z_std_grid);

    Some(SurfaceSlice {
        param_x_idx,
        param_y_idx,
        x_values,
        y_values,
        z_values,
        z_std,
    })
}

/// 多目的サロゲート最適化の入力。
pub struct SurrogateMultiOptRequest {
    /// 訓練データ（行 = trial、列 = パラメータ）。元の単位。
    pub x_matrix: Vec<Vec<f64>>,
    /// 目的ごとの値列。`ys[k][i]` = trial i の目的 k の値。
    pub ys: Vec<Vec<f64>>,
    /// 各パラメータ列の名前。
    pub param_names: Vec<String>,
    /// `ys` と同順の目的名。
    pub objective_names: Vec<String>,
    /// 目的ごとに true = 最小化。`ys` と同じ長さ。
    pub minimize: Vec<bool>,
    /// 使用するサロゲートモデル。
    pub model: SurrogateModelKind,
    /// 応答曲面スライスの 2 パラメータ列 index（表示用）。
    pub slice_params: Option<(usize, usize)>,
    /// スライス格子の一辺の点数。
    pub n_grid: usize,
}

/// 予測パレートフロント上の 1 点。
#[derive(Debug, Clone)]
pub struct ParetoFrontPoint {
    /// パラメータ値（元の単位、`param_names` と同順）。
    pub params: Vec<f64>,
    /// 各目的のサロゲート予測値（元の単位、`objective_names` と同順）。
    pub values: Vec<f64>,
}

/// 多目的サロゲート最適化の結果。
#[derive(Debug, Clone)]
pub struct SurrogateMultiOptResult {
    /// 予測パレートフロント（第 1 目的の値で昇順ソート済み）。
    pub front: Vec<ParetoFrontPoint>,
    /// 目的ごとの訓練データ決定係数（`objective_names` と同順）。
    pub r_squared: Vec<f64>,
    /// 目的ごとの応答曲面スライス（`slice_params` 指定時のみ、`objective_names` と同順。指定なし/無効時は空）。
    pub slices: Vec<SurfaceSlice>,
}

/// 多目的最適化ステージの設定（学習済みモデル群に対して実行する）。
pub struct SurrogateMultiOptimizeSpec {
    /// 目的ごとに true = 最小化。`trained` と同じ長さ。
    pub minimize: Vec<bool>,
    pub slice_params: Option<(usize, usize)>,
    pub n_grid: usize,
}

/// 多目的最適化の共通入力（目的 1 件ぶん）。
struct MultiObjectiveEntry<'a> {
    surrogate: &'a models::FittedSurrogate,
    /// 観測ベスト点（初期シード）の探索に使う学習データ。
    x_matrix: &'a [Vec<f64>],
    y: &'a [f64],
}

/// 学習済みサロゲート群に対する NSGA-II 実行＋フロント後処理の共通ロジック。
///
/// 全 entry のサロゲートは同一の正規化変換（col_stats）を持つ前提
/// （同じパラメータ空間の x_matrix から学習されていること）。
fn run_multi_optimize(
    entries: &[MultiObjectiveEntry<'_>],
    minimize: &[bool],
    slice_params: Option<(usize, usize)>,
    n_grid: usize,
) -> SurrogateMultiOptResult {
    let n_obj = entries.len();
    let surrogates: Vec<&models::FittedSurrogate> = entries.iter().map(|e| e.surrogate).collect();
    let ref_surrogate = surrogates[0];
    let n_dims = ref_surrogate.col_stats.len();

    let r_squared: Vec<f64> = surrogates.iter().map(|s| s.r_squared).collect();

    // ── 初期シード: 目的ごとの観測ベスト点を正規化 ─────────────────
    // col_stats は全サロゲートで共通のため、先頭サロゲートの to_norm_x を使う。
    let seeds: Vec<Vec<f64>> = entries
        .iter()
        .zip(minimize.iter())
        .map(|(e, &min_k)| {
            let best_idx = best_observed_index(e.y, min_k);
            ref_surrogate.to_norm_x(&e.x_matrix[best_idx])
        })
        .collect();

    // ── NSGA-II 実行 ─────────────────────────────────────────────────
    let signs: Vec<f64> = minimize
        .iter()
        .map(|&m| if m { 1.0 } else { -1.0 })
        .collect();
    let raw_front = optimizers::multi_objective_nsga2(&surrogates, &signs, &seeds);

    // ── フロント点の後処理 ──────────────────────────────────────────
    // 重複遺伝子の除去（全次元 1e-9 以内）。
    let mut deduped: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
    'outer: for (genome, fitness) in raw_front {
        for (existing, _) in &deduped {
            if genome
                .iter()
                .zip(existing.iter())
                .all(|(a, b)| (a - b).abs() < 1e-9)
            {
                continue 'outer;
            }
        }
        deduped.push((genome, fitness));
    }

    // 各点の遺伝子を [0,1] にクランプし、全目的のサロゲート予測値（元の単位）を計算。
    let mut front_points: Vec<ParetoFrontPoint> = deduped
        .into_iter()
        .map(|(genome, _)| {
            let clamped: Vec<f64> = genome.iter().map(|v| v.clamp(0.0, 1.0)).collect();
            let params = ref_surrogate.to_original_x(&clamped);
            let values: Vec<f64> = surrogates
                .iter()
                .map(|s| s.to_original_y(s.predict_norm(&clamped)))
                .collect();
            ParetoFrontPoint { params, values }
        })
        .collect();

    // 第 1 目的の値で昇順ソート。
    front_points.sort_by(|a, b| {
        a.values[0]
            .partial_cmp(&b.values[0])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // ── スライス ─────────────────────────────────────────────────────
    let slices = if let Some((px, py)) = slice_params {
        // バランス点: 正規化目的空間で理想点に最も近い点を選ぶ。
        // 理想点 = 各目的の sign 調整済み最小値（NSGA-II の最小化方向）。
        if front_points.is_empty() || px >= n_dims || py >= n_dims || px == py {
            Vec::new()
        } else {
            // 正規化目的空間での理想点。
            let ideal: Vec<f64> = (0..n_obj)
                .map(|k| {
                    front_points
                        .iter()
                        .map(|p| signs[k] * p.values[k])
                        .fold(f64::INFINITY, f64::min)
                })
                .collect();

            // バランス点（理想点に最近の点）の正規化パラメータを求める。
            let ideal_dist = |p: &ParetoFrontPoint| -> f64 {
                (0..n_obj)
                    .map(|k| (signs[k] * p.values[k] - ideal[k]).powi(2))
                    .sum()
            };
            let balance_norm = front_points
                .iter()
                .min_by(|a, b| {
                    ideal_dist(a)
                        .partial_cmp(&ideal_dist(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|p| ref_surrogate.to_norm_x(&p.params))
                .unwrap_or_else(|| vec![0.5; n_dims]);

            // 目的ごとにスライスを構築。
            surrogates
                .iter()
                .filter_map(|s| build_slice(s, &balance_norm, px, py, n_grid.max(2), n_dims))
                .collect()
        }
    } else {
        Vec::new()
    };

    SurrogateMultiOptResult {
        front: front_points,
        r_squared,
        slices,
    }
}

/// 検証済みの学習結果群に対して NSGA-II でパレートフロントを推定する。
/// `trained[k]` は目的 k のサロゲート。全要素が同一の param_names / 学習データ次元を持つこと。
pub fn optimize_multi_on_trained(
    trained: &[&TrainedSurrogate],
    spec: &SurrogateMultiOptimizeSpec,
) -> Result<SurrogateMultiOptResult, String> {
    let n_obj = trained.len();
    if n_obj < 2 {
        return Err(format!(
            "At least 2 objectives required (current: {})",
            n_obj
        ));
    }
    if spec.minimize.len() != n_obj {
        return Err("trained and minimize length mismatch".to_string());
    }
    let first = trained[0];
    if trained.iter().any(|t| t.param_names != first.param_names) {
        return Err("trained surrogates have inconsistent param_names".to_string());
    }
    let n_dims = first.surrogate.col_stats.len();
    if trained
        .iter()
        .any(|t| t.surrogate.col_stats.len() != n_dims)
    {
        return Err("trained surrogates have inconsistent dimensions".to_string());
    }

    let entries: Vec<MultiObjectiveEntry<'_>> = trained
        .iter()
        .map(|t| MultiObjectiveEntry {
            surrogate: &t.surrogate,
            x_matrix: &t.x_matrix,
            y: &t.y,
        })
        .collect();

    Ok(run_multi_optimize(
        &entries,
        &spec.minimize,
        spec.slice_params,
        spec.n_grid,
    ))
}

/// 多目的サロゲートモデルを学習し、NSGA-II でパレートフロントを推定する。
///
/// バックグラウンドスレッドから呼べるよう、スレッドローカルの DataFrame には依存しない。
pub fn run_surrogate_multi_optimization(
    req: &SurrogateMultiOptRequest,
) -> Result<SurrogateMultiOptResult, String> {
    // ── バリデーション ────────────────────────────────────────────────
    let n_obj = req.ys.len();
    if n_obj < 2 {
        return Err(format!(
            "At least 2 objectives required (current: {})",
            n_obj
        ));
    }
    if req.objective_names.len() != n_obj {
        return Err("ys and objective_names length mismatch".to_string());
    }
    if req.minimize.len() != n_obj {
        return Err("ys and minimize length mismatch".to_string());
    }

    let n = req.ys[0].len();
    if n < MIN_TRIALS_FOR_SURROGATE_OPT {
        return Err(format!(
            "At least {} trials required (current: {})",
            MIN_TRIALS_FOR_SURROGATE_OPT, n
        ));
    }
    for (k, yk) in req.ys.iter().enumerate() {
        if yk.len() != n {
            return Err(format!(
                "ys[{}] length {} does not match ys[0] length {}",
                k,
                yk.len(),
                n
            ));
        }
    }
    if req.x_matrix.len() != n {
        return Err("x_matrix and y length mismatch".to_string());
    }
    let n_dims = req.x_matrix.first().map(|r| r.len()).unwrap_or(0);
    if n_dims == 0 {
        return Err("No numeric parameters available".to_string());
    }
    if req.x_matrix.iter().any(|row| row.len() != n_dims) {
        return Err("x_matrix rows have inconsistent dimensions".to_string());
    }
    if req.x_matrix.iter().flatten().any(|v| !v.is_finite())
        || req.ys.iter().flatten().any(|v| !v.is_finite())
    {
        return Err("Input contains non-finite values".to_string());
    }

    // ── 目的ごとにサロゲートを学習 ──────────────────────────────────
    let surrogates: Vec<models::FittedSurrogate> = req
        .ys
        .iter()
        .map(|yk| models::fit_surrogate(req.model, &req.x_matrix, yk))
        .collect::<Result<Vec<_>, _>>()?;

    let entries: Vec<MultiObjectiveEntry<'_>> = surrogates
        .iter()
        .zip(req.ys.iter())
        .map(|(surrogate, yk)| MultiObjectiveEntry {
            surrogate,
            x_matrix: &req.x_matrix,
            y: yk,
        })
        .collect();

    Ok(run_multi_optimize(
        &entries,
        &req.minimize,
        req.slice_params,
        req.n_grid,
    ))
}

/// 多目的サロゲートを目的ごとに学習する（パレートフロント集中つき）。
///
/// `objective_values[k]` は目的 k の列（長さ N）、`minimize[k]` はその最適化方向。
/// 全目的を行ベクトルに組み替えて `nd_sort` で非劣（rank == 0）trial を求め、それらを
/// 各 GP の誘導点として優先する（`SurrogateFitRequest.priority_rows`）。
///
/// フロント集中は N が GP の誘導点上限（100）を超えるときのみモデルを変える。
/// N ≤ 100 では各 GP が Z = X（全点）を使うため、優先指定は結果に影響しない。
pub fn fit_multi_surrogates(
    x_matrix: &[Vec<f64>],
    objective_values: &[Vec<f64>],
    param_names: &[String],
    objective_names: &[String],
    model: SurrogateModelKind,
    minimize: &[bool],
) -> Result<Vec<TrainedSurrogate>, String> {
    let n_obj = objective_values.len();
    if n_obj != objective_names.len() || n_obj != minimize.len() {
        return Err(
            "objective_values, objective_names and minimize must have equal length".to_string(),
        );
    }
    if n_obj == 0 {
        return Err("At least 1 objective required".to_string());
    }
    let n = x_matrix.len();
    for (k, col) in objective_values.iter().enumerate() {
        if col.len() != n {
            return Err(format!(
                "objective_values[{}] length {} does not match x_matrix rows {}",
                k,
                col.len(),
                n
            ));
        }
    }

    // 行ごとの目的ベクトル rows[i][k] を組み、非劣 trial（rank == 0）を優先行にする。
    let rows: Vec<Vec<f64>> = (0..n)
        .map(|i| objective_values.iter().map(|col| col[i]).collect())
        .collect();
    let ranks = crate::multi_objective::pareto::nd_sort(&rows, minimize);
    let priority: Vec<usize> = ranks
        .iter()
        .enumerate()
        .filter(|(_, &r)| r == 0)
        .map(|(i, _)| i)
        .collect();

    let mut trained = Vec::with_capacity(n_obj);
    for k in 0..n_obj {
        let req = SurrogateFitRequest {
            x_matrix: x_matrix.to_vec(),
            y: objective_values[k].clone(),
            param_names: param_names.to_vec(),
            objective_name: objective_names[k].clone(),
            model,
            auto_select: false,
            constraints: vec![],
            priority_rows: priority.clone(),
        };
        let t = fit_surrogate_with_validation(&req).map_err(|e| {
            format!(
                "Fitting failed for objective '{}': {}",
                objective_names[k], e
            )
        })?;
        trained.push(t);
    }
    Ok(trained)
}

#[cfg(test)]
mod tests;
