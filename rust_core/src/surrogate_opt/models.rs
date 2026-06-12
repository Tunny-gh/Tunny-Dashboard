//! サロゲートモデルの学習・予測ラッパ。
//!
//! Ridge（`sensitivity::ridge`）、ガウス過程 / スパースガウス過程（egobox-gp FITC バックエンド）、
//! LightGBM を統一インターフェースで包む。予測は正規化空間（X: min-max [0,1]、y: z-score）で行い、
//! 元の単位との変換は [`FittedSurrogate`] が担う。

use std::sync::Mutex;

use crate::gaussian_process::GpModel;
use crate::lgbm::{lgbm_predict, train_lgbm_rf, LgbmBooster, LgbmRfConfig};
use crate::pdp::utils::{normalize_x_minmax, normalize_y, r_squared};
use crate::sensitivity::compute_ridge_from_vecs;

/// 応答曲面の作成に使うサロゲートモデル種別。
/// 新しいモデルはここへバリアントを追加する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurrogateModelKind {
    /// Ridge 回帰（線形）。高速だが曲面は平面。
    Ridge,
    /// ガウス過程回帰（ARD Matérn 5/2）。100 点誘導点で学習。
    GaussianProcess,
    /// FITC 近似によるスパースガウス過程回帰。大規模データ向け。
    SparseGaussianProcess,
    /// LightGBM（RandomForest モード）。非線形・非平滑な応答に強いが、
    /// 予測は区分定数のため勾配法（L-BFGS）とは相性が悪い。
    Lgbm,
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
    /// egobox-gp FITC バックエンドによるガウス過程（通常 GP / スパース GP 共通）。
    Gp(Box<GpModel>),
    /// LightGBM RandomForest の Booster。
    /// FittedSurrogate / TrainedSurrogate は Arc 経由で複数スレッドから共有されうるが、
    /// LightGBM の predict は同一ハンドルに対して非スレッドセーフのため、
    /// Mutex で直列化して Sync を満たす（`LgbmBooster` は Send のみ実装）。
    Lgbm(Mutex<LgbmBooster>),
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
            FittedModel::Gp(model) => model.predict_mean(x_norm),
            FittedModel::Lgbm(booster) => {
                // poisoned lock は panic 連鎖を避けて内部値をそのまま使う
                // （Booster は predict で内部状態を変更しないため安全）。
                let booster = booster.lock().unwrap_or_else(|e| e.into_inner());
                lgbm_predict(&booster, &[x_norm.to_vec()])
                    .first()
                    .copied()
                    .unwrap_or(0.0)
            }
        }
    }

    /// 正規化空間での予測分散（事後分散を持つモデルのみ）。
    pub(crate) fn predict_var_norm(&self, x_norm: &[f64]) -> Option<f64> {
        match &self.model {
            FittedModel::Ridge { .. } | FittedModel::Lgbm(_) => None,
            FittedModel::Gp(model) => Some(model.predict_variance(x_norm)),
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
        SurrogateModelKind::GaussianProcess => FittedModel::Gp(Box::new(
            GpModel::fit(&x_norm, &y_norm, 100, 42).ok_or("Gaussian Process training failed")?,
        )),
        SurrogateModelKind::SparseGaussianProcess => FittedModel::Gp(Box::new(
            GpModel::fit(&x_norm, &y_norm, 20, 42)
                .ok_or("Sparse Gaussian Process training failed")?,
        )),
        SurrogateModelKind::Lgbm => FittedModel::Lgbm(Mutex::new(
            train_lgbm_rf(&x_norm, &y_norm, &LgbmRfConfig::default())
                .ok_or("LightGBM training failed")?,
        )),
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
