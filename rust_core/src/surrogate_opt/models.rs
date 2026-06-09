//! サロゲートモデルの学習・予測ラッパ。
//!
//! 既存の Ridge（`sensitivity::ridge`）と Kriging / Sparse Kriging（`kriging`）を
//! 統一インターフェースで包む。予測は正規化空間（X: min-max [0,1]、y: z-score）で行い、
//! 元の単位との変換は [`FittedSurrogate`] が担う。

use crate::kriging::{gaussian_process, sparse_fitc};
use crate::pdp::utils::{normalize_x_minmax, normalize_y, r_squared};
use crate::sensitivity::compute_ridge_from_vecs;

/// 応答曲面の作成に使うサロゲートモデル種別。
/// 新しいモデル（例: Random Forest / LightGBM）はここへバリアントを追加する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurrogateModelKind {
    /// Ridge 回帰（線形）。高速だが曲面は平面。
    Ridge,
    /// ガウス過程回帰（ARD Matérn 5/2）。100 点サブサンプルで学習。
    Kriging,
    /// FITC 近似によるスパースガウス過程回帰。大規模データ向け。
    SparseKriging,
}

/// 学習済みモデル本体（正規化空間で予測する）。
pub(crate) enum FittedModel {
    /// z-score 標準化済み列に対する Ridge 係数（`sensitivity::ridge` と同じ規約）。
    Ridge {
        beta: Vec<f64>,
        col_mean: Vec<f64>,
        col_std: Vec<f64>,
        y_norm_mean: f64,
    },
    Gp(gaussian_process::GpModel),
    Fitc(sparse_fitc::SparseFitcModel),
}

/// 学習済みサロゲートと正規化統計量。
pub(crate) struct FittedSurrogate {
    pub(crate) model: FittedModel,
    /// 各列の (min, range)（`normalize_x_minmax` と同じ）。
    pub(crate) col_stats: Vec<(f64, f64)>,
    pub(crate) y_mean: f64,
    pub(crate) y_std: f64,
    /// 訓練データに対する決定係数（元の単位で評価）。
    pub(crate) r_squared: f64,
}

impl FittedSurrogate {
    /// 正規化空間での予測（y は z-score 単位）。
    pub(crate) fn predict_norm(&self, x_norm: &[f64]) -> f64 {
        match &self.model {
            FittedModel::Ridge {
                beta,
                col_mean,
                col_std,
                y_norm_mean,
            } => {
                let mut acc = *y_norm_mean;
                for (d, &b) in beta.iter().enumerate() {
                    acc += b * (x_norm[d] - col_mean[d]) / col_std[d];
                }
                acc
            }
            FittedModel::Gp(model) => gaussian_process::predict_mean(model, x_norm),
            FittedModel::Fitc(model) => sparse_fitc::fitc_predict_mean(model, x_norm),
        }
    }

    /// 正規化空間での予測分散（事後分散を持つモデルのみ）。
    pub(crate) fn predict_var_norm(&self, x_norm: &[f64]) -> Option<f64> {
        match &self.model {
            FittedModel::Ridge { .. } => None,
            FittedModel::Gp(model) => {
                Some(gaussian_process::predict_variance(model, x_norm).max(0.0))
            }
            FittedModel::Fitc(model) => {
                Some(sparse_fitc::fitc_predict_variance(model, x_norm).max(0.0))
            }
        }
    }

    /// 元の単位の点を正規化空間 [0,1]^d へ写す。
    pub(crate) fn to_norm_x(&self, x_orig: &[f64]) -> Vec<f64> {
        x_orig
            .iter()
            .zip(self.col_stats.iter())
            .map(|(&v, &(min, range))| ((v - min) / range).clamp(0.0, 1.0))
            .collect()
    }

    /// 正規化空間の点を元の単位へ戻す。
    pub(crate) fn to_original_x(&self, x_norm: &[f64]) -> Vec<f64> {
        x_norm
            .iter()
            .zip(self.col_stats.iter())
            .map(|(&t, &(min, range))| min + t * range)
            .collect()
    }

    /// z-score 単位の予測値を元の単位へ戻す。
    pub(crate) fn to_original_y(&self, y_norm: f64) -> f64 {
        y_norm * self.y_std + self.y_mean
    }
}

/// 指定モデルでサロゲートを学習する。
pub(crate) fn fit_surrogate(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
) -> Result<FittedSurrogate, String> {
    let (col_stats, x_norm) = normalize_x_minmax(x_matrix);
    let (y_mean, y_std, y_norm) = normalize_y(y);

    let model = match kind {
        SurrogateModelKind::Ridge => fit_ridge(&x_norm, &y_norm)?,
        SurrogateModelKind::Kriging => FittedModel::Gp(
            gaussian_process::train_gp(x_norm.clone(), y_norm.clone(), 100, 42)
                .ok_or("Kriging training failed")?,
        ),
        SurrogateModelKind::SparseKriging => fit_sparse_kriging(&x_norm, &y_norm)?,
    };

    let mut surrogate = FittedSurrogate {
        model,
        col_stats,
        y_mean,
        y_std,
        r_squared: 0.0,
    };
    let y_pred: Vec<f64> = x_norm
        .iter()
        .map(|row| surrogate.to_original_y(surrogate.predict_norm(row)))
        .collect();
    surrogate.r_squared = r_squared(y, &y_pred);
    Ok(surrogate)
}

fn fit_ridge(x_norm: &[Vec<f64>], y_norm: &[f64]) -> Result<FittedModel, String> {
    let ridge = compute_ridge_from_vecs(x_norm, y_norm, 1.0);
    if ridge.beta.is_empty() {
        return Err("Ridge training failed".to_string());
    }
    let n = y_norm.len() as f64;
    let n_dims = x_norm[0].len();
    let col_mean: Vec<f64> = (0..n_dims)
        .map(|d| x_norm.iter().map(|r| r[d]).sum::<f64>() / n)
        .collect();
    let col_std: Vec<f64> = (0..n_dims)
        .map(|d| {
            let var = x_norm
                .iter()
                .map(|r| (r[d] - col_mean[d]).powi(2))
                .sum::<f64>()
                / n;
            var.sqrt().max(f64::EPSILON)
        })
        .collect();
    let y_norm_mean = y_norm.iter().sum::<f64>() / n;
    Ok(FittedModel::Ridge {
        beta: ridge.beta,
        col_mean,
        col_std,
        y_norm_mean,
    })
}

/// FITC スパース Kriging を学習する。
/// `pdp::kriging` の sparse 経路と同じく、100 点サブサンプルの標準 GP から
/// ハイパーパラメータを借り、誘導点は GP サブサンプルの k-means で選ぶ。
/// FITC 学習が数値的に失敗した場合は標準 GP へフォールバックする。
fn fit_sparse_kriging(x_norm: &[Vec<f64>], y_norm: &[f64]) -> Result<FittedModel, String> {
    let n = y_norm.len();
    let n_dims = x_norm[0].len();

    let gp_model = gaussian_process::train_gp(x_norm.to_vec(), y_norm.to_vec(), 100, 42)
        .ok_or("Sparse Kriging training failed (hyperparameter GP)")?;

    let mut fitc_params: Vec<f64> = gp_model.kernel.log_ls.clone();
    fitc_params.push(gp_model.kernel.log_sf);
    fitc_params.push(gp_model.kernel.log_sn);

    const M: usize = 20;
    let gp_n = gp_model.x_train.len();
    let m = M.min(gp_n);

    // GP サブサンプル（正規化済み）から column-major flat 配列を作る。
    let mut gp_x_flat = vec![0.0_f64; n_dims * gp_n];
    for i in 0..gp_n {
        for d in 0..n_dims {
            gp_x_flat[d * gp_n + i] = gp_model.x_train[i][d];
        }
    }
    let z = sparse_fitc::select_inducing_points_kmeans(&gp_x_flat, gp_n, n_dims, m, 42);

    let mut x_flat = vec![0.0_f64; n_dims * n];
    for (i, row) in x_norm.iter().enumerate() {
        for d in 0..n_dims {
            x_flat[d * n + i] = row[d];
        }
    }

    match sparse_fitc::fitc_train(&x_flat, &z, y_norm, &fitc_params, n, m) {
        Some(model) if model.w.iter().all(|v| v.is_finite()) => Ok(FittedModel::Fitc(model)),
        _ => Ok(FittedModel::Gp(gp_model)),
    }
}
