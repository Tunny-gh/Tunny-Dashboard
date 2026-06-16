//! サロゲートモデルのホールドアウト＋k-fold CV 検証。

use crate::pdp::utils::r_squared;

use super::models::{fit_surrogate, SurrogateModelKind};
use super::progress::FitProgress;
use crate::math::rng::SeededRng;

/// ホールドアウト + k-fold CV によるサロゲートモデルの検証レポート。
#[derive(Debug, Clone)]
pub struct SurrogateValidationReport {
    pub n_samples: usize,
    /// ホールドアウト分割の訓練側サンプル数（全体の 8 割）。
    pub n_train: usize,
    /// ホールドアウト分割のテスト側サンプル数（全体の 2 割）。
    pub n_test: usize,
    /// 全データで学習した最終モデルの訓練 R²（元の単位）。
    pub train_r2: f64,
    /// 8:2 ホールドアウト: 80% で学習し、残り 20% に対する R²。
    pub holdout_r2: f64,
    /// 同テストデータに対する RMSE（元の単位）。
    pub holdout_rmse: f64,
    /// CV の fold 数（データが少ない場合は 5 未満になりうる）。
    pub cv_folds: usize,
    /// fold ごとの検証 R² の平均と標準偏差（母標準偏差）。
    pub cv_r2_mean: f64,
    pub cv_r2_std: f64,
    /// fold ごとの検証 RMSE の平均と標準偏差。
    pub cv_rmse_mean: f64,
    pub cv_rmse_std: f64,
    /// out-of-fold の (実測値, 予測値) ペア（元の単位、予測 vs 実測プロット用）。
    pub oof_pairs: Vec<(f64, f64)>,
    /// `oof_pairs` と同順で、その点がパレートフロント（多目的 rank 0）の trial か。
    /// 多目的フィットのみ非空（単目的フィットや Auto 選択時の検証では全要素なし）。
    /// フロント近傍の近似度を散布図で色分けするために使う。
    pub oof_is_front: Vec<bool>,
    /// パレートフロント点のみで算出した OOF R²（フロント点が 2 点未満／分散ゼロなら None）。
    pub front_r2: Option<f64>,
    /// パレートフロント点のみで算出した OOF RMSE（フロント点が無ければ None）。
    pub front_rmse: Option<f64>,
}

#[cfg(test)]
impl SurrogateValidationReport {
    /// テスト用のプレースホルダ検証レポート（解析的モックサロゲートに添える）。
    /// 検証は行わない（曲面が既知のため）ので、R² 系は完全フィットを表す 1.0 を入れる。
    pub(crate) fn placeholder() -> Self {
        SurrogateValidationReport {
            n_samples: 0,
            n_train: 0,
            n_test: 0,
            train_r2: 1.0,
            holdout_r2: 1.0,
            holdout_rmse: 0.0,
            cv_folds: 0,
            cv_r2_mean: 1.0,
            cv_r2_std: 0.0,
            cv_rmse_mean: 0.0,
            cv_rmse_std: 0.0,
            oof_pairs: vec![],
            oof_is_front: vec![],
            front_r2: None,
            front_rmse: None,
        }
    }
}

/// RMSE を計算する（元の単位）。
fn rmse(actual: &[f64], pred: &[f64]) -> f64 {
    let n = actual.len();
    if n == 0 {
        return 0.0;
    }
    let mse = actual
        .iter()
        .zip(pred.iter())
        .map(|(&a, &p)| (a - p).powi(2))
        .sum::<f64>()
        / n as f64;
    mse.sqrt()
}

/// 母標準偏差を計算する。
fn population_std(values: &[f64]) -> f64 {
    let n = values.len();
    if n == 0 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let var = values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    var.sqrt()
}

/// 指定モデルに対してホールドアウト＋k-fold CV を実施し、検証レポートを返す。
///
/// - シャッフルは `seed` を用いた ChaCha8 RNG で決定論的に実施する。
/// - ホールドアウト: n_test = max(1, round(n × 0.2)) 点をテストに使用する。
/// - k-fold CV: k = min(5, n)。fold へのアサインはシャッフル後の round-robin。
///   縮退 fold（点数 < 2 または分散ゼロ）は R² 平均・標準偏差から除外するが、
///   OOF ペアと RMSE には含める。
///
/// `train_r2` は呼び出し元（`fit_surrogate_with_validation`）が全データモデルの
/// 値で上書きするため、ここでは 0.0 を返す。
#[cfg(test)]
pub(crate) fn validate_surrogate(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    seed: u64,
) -> Result<SurrogateValidationReport, String> {
    validate_surrogate_tracked(kind, x_matrix, y, seed, &FitProgress::default())
}

/// [`validate_surrogate`] と同じだが、各モデル学習の境界で `progress` を更新し、
/// キャンセル要求があれば早期に `Err` を返す。学習回数（ホールドアウト 1 + CV k 回）
/// だけ [`FitProgress::inc_done`] を呼ぶ。段階ラベルは呼び出し側が設定する。
pub(crate) fn validate_surrogate_tracked(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    seed: u64,
    progress: &FitProgress,
) -> Result<SurrogateValidationReport, String> {
    validate_surrogate_tracked_front(kind, x_matrix, y, seed, &[], progress)
}

/// [`validate_surrogate_tracked`] と同じだが、`front_rows`（パレートフロント = rank 0 の
/// 行 index、`x_matrix` への index）を受け取り、各 OOF 点がフロントかを記録して
/// フロント点のみの R²/RMSE も算出する。多目的フィットでフロント近傍の近似度を示すため。
pub(crate) fn validate_surrogate_tracked_front(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    seed: u64,
    front_rows: &[usize],
    progress: &FitProgress,
) -> Result<SurrogateValidationReport, String> {
    use std::collections::HashSet;
    let front_set: HashSet<usize> = front_rows.iter().copied().collect();
    let n = y.len();

    // シャッフル済みインデックスを生成する。
    let mut indices: Vec<usize> = (0..n).collect();
    let mut rng = SeededRng::from_seed(seed);
    rng.shuffle(&mut indices);

    // ---- ホールドアウト ----
    let n_test = ((n as f64 * 0.2).round() as usize).max(1);
    let n_train = n - n_test;

    let train_indices: Vec<usize> = indices[..n_train].to_vec();
    let test_indices: Vec<usize> = indices[n_train..].to_vec();

    let train_x: Vec<Vec<f64>> = train_indices.iter().map(|&i| x_matrix[i].clone()).collect();
    let train_y: Vec<f64> = train_indices.iter().map(|&i| y[i]).collect();
    let test_x: Vec<Vec<f64>> = test_indices.iter().map(|&i| x_matrix[i].clone()).collect();
    let test_y: Vec<f64> = test_indices.iter().map(|&i| y[i]).collect();

    progress.check()?;
    let holdout_model = fit_surrogate(kind, &train_x, &train_y)
        .map_err(|e| format!("ホールドアウト訓練失敗: {e}"))?;
    progress.inc_done();

    let holdout_pred: Vec<f64> = test_x
        .iter()
        .map(|row| {
            let x_norm = holdout_model.to_norm_x(row);
            holdout_model.to_original_y(holdout_model.predict_norm(&x_norm))
        })
        .collect();

    let holdout_r2 = r_squared(&test_y, &holdout_pred);
    let holdout_rmse = rmse(&test_y, &holdout_pred);

    // ---- k-fold CV ----
    let k = n.min(5);

    // シャッフル済みインデックスを round-robin で k fold に割り当てる。
    let mut fold_indices: Vec<Vec<usize>> = vec![Vec::new(); k];
    for (pos, &idx) in indices.iter().enumerate() {
        fold_indices[pos % k].push(idx);
    }

    let mut oof_pairs: Vec<(f64, f64)> = Vec::with_capacity(n);
    let mut oof_is_front: Vec<bool> = Vec::with_capacity(n);
    let mut cv_r2_values: Vec<f64> = Vec::with_capacity(k);
    let mut cv_rmse_values: Vec<f64> = Vec::with_capacity(k);

    for fold in 0..k {
        // fold 以外を訓練に使用する。
        let cv_train_indices: Vec<usize> = (0..k)
            .filter(|&f| f != fold)
            .flat_map(|f| fold_indices[f].iter().copied())
            .collect();
        let cv_val_indices: &[usize] = &fold_indices[fold];

        let cv_train_x: Vec<Vec<f64>> = cv_train_indices
            .iter()
            .map(|&i| x_matrix[i].clone())
            .collect();
        let cv_train_y: Vec<f64> = cv_train_indices.iter().map(|&i| y[i]).collect();
        let cv_val_x: Vec<Vec<f64>> = cv_val_indices
            .iter()
            .map(|&i| x_matrix[i].clone())
            .collect();
        let cv_val_y: Vec<f64> = cv_val_indices.iter().map(|&i| y[i]).collect();

        progress.check()?;
        let cv_model = fit_surrogate(kind, &cv_train_x, &cv_train_y)
            .map_err(|e| format!("CV fold {fold} 訓練失敗: {e}"))?;
        progress.inc_done();

        let cv_pred: Vec<f64> = cv_val_x
            .iter()
            .map(|row| {
                let x_norm = cv_model.to_norm_x(row);
                cv_model.to_original_y(cv_model.predict_norm(&x_norm))
            })
            .collect();

        // OOF ペアを収集する（元の行 index でフロント所属も記録）。
        for ((&idx, &actual), &predicted) in cv_val_indices
            .iter()
            .zip(cv_val_y.iter())
            .zip(cv_pred.iter())
        {
            oof_pairs.push((actual, predicted));
            oof_is_front.push(front_set.contains(&idx));
        }

        // fold RMSE（縮退 fold でも含める）。
        cv_rmse_values.push(rmse(&cv_val_y, &cv_pred));

        // 縮退 fold（点数 < 2 または分散ゼロ）は R² から除外する。
        if cv_val_y.len() < 2 {
            continue;
        }
        let y_mean = cv_val_y.iter().sum::<f64>() / cv_val_y.len() as f64;
        let ss_tot: f64 = cv_val_y.iter().map(|&v| (v - y_mean).powi(2)).sum();
        if ss_tot < f64::EPSILON {
            continue;
        }
        cv_r2_values.push(r_squared(&cv_val_y, &cv_pred));
    }

    // CV R² の平均・標準偏差（有効 fold のみ）。
    let cv_r2_mean = if cv_r2_values.is_empty() {
        0.0
    } else {
        cv_r2_values.iter().sum::<f64>() / cv_r2_values.len() as f64
    };
    let cv_r2_std = population_std(&cv_r2_values);

    // CV RMSE の平均・標準偏差（全 fold）。
    let cv_rmse_mean = if cv_rmse_values.is_empty() {
        0.0
    } else {
        cv_rmse_values.iter().sum::<f64>() / cv_rmse_values.len() as f64
    };
    let cv_rmse_std = population_std(&cv_rmse_values);

    // パレートフロント点のみの OOF R²/RMSE（フロント近傍の近似度）。
    let front_actual: Vec<f64> = oof_pairs
        .iter()
        .zip(oof_is_front.iter())
        .filter(|(_, &f)| f)
        .map(|(&(a, _), _)| a)
        .collect();
    let front_pred: Vec<f64> = oof_pairs
        .iter()
        .zip(oof_is_front.iter())
        .filter(|(_, &f)| f)
        .map(|(&(_, p), _)| p)
        .collect();
    let front_rmse = if front_actual.is_empty() {
        None
    } else {
        Some(rmse(&front_actual, &front_pred))
    };
    let front_r2 = if front_actual.len() < 2 {
        None
    } else {
        let mean = front_actual.iter().sum::<f64>() / front_actual.len() as f64;
        let ss_tot: f64 = front_actual.iter().map(|&v| (v - mean).powi(2)).sum();
        if ss_tot < f64::EPSILON {
            None
        } else {
            Some(r_squared(&front_actual, &front_pred))
        }
    };

    Ok(SurrogateValidationReport {
        n_samples: n,
        n_train,
        n_test,
        train_r2: 0.0, // 呼び出し元が全データモデルの値で上書きする。
        holdout_r2,
        holdout_rmse,
        cv_folds: k,
        cv_r2_mean,
        cv_r2_std,
        cv_rmse_mean,
        cv_rmse_std,
        oof_pairs,
        oof_is_front,
        front_r2,
        front_rmse,
    })
}
