//! サロゲートモデルの学習・予測ラッパ。
//!
//! Ridge（`sensitivity::ridge`）、ガウス過程 3 方式（FITC / VFE / 混合エキスパート）、
//! LightGBM を統一インターフェースで包む。予測は正規化空間（X: min-max [0,1]、y: z-score）で行い、
//! 元の単位との変換は [`FittedSurrogate`] が担う。

use std::sync::Mutex;

use crate::gaussian_process::{GpMethod, GpModel};
use crate::lgbm::{lgbm_predict, train_lgbm_rf, LgbmBooster, LgbmRfConfig};
use crate::pdp::utils::{normalize_x_minmax, normalize_y, r_squared};
use crate::sensitivity::compute_ridge_from_vecs;

/// 応答曲面の作成に使うサロゲートモデル種別。
/// 新しいモデルはここへバリアントを追加する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurrogateModelKind {
    /// Ridge 回帰（線形）。高速だが曲面は平面。
    Ridge,
    /// FITC 近似（Fully Independent Training Conditional）によるスパースガウス過程回帰。
    /// M = min(N, 100) 誘導点を使用。N ≤ 100 では厳密 GP と等価。
    GpFitc,
    /// VFE 近似（Variational Free Energy）によるスパースガウス過程回帰。
    /// FITC よりノイズを保守的に見積もる傾向がある。M = min(N, 100)。
    GpVfe,
    /// 混合エキスパート（クラスタごとの FITC GP を滑らかに再結合）。
    /// 不連続・多峰応答向け。クラスタ数は交差検証で自動選択（最大 3）。
    GpMoe,
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
    /// egobox-gp バックエンドによるガウス過程（FITC / VFE / MoE 共通）。
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
    /// ガウス過程 3 方式（FITC / VFE / MoE）はすべて Some を返す。
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

    /// ARD 長さスケールから算出した相対パラメータ重要度（入力次元ごと、合計 1.0）。
    ///
    /// GP（単一 SGP）のみ Some を返す。各次元の θ_d を θ の総和で割って正規化する
    /// （egobox / SMT 規約では θ_d が大きいほど次元 d に敏感）。総和 ≤ 0 や非有限値が
    /// あれば None。MoE は θ が一意でないため、Ridge / LightGBM は ARD を持たないため None。
    /// 並びは学習時の入力列順（= `param_names` / `x_matrix` の列順）に一致する。
    pub(crate) fn param_importance(&self) -> Option<Vec<f64>> {
        let theta = match &self.model {
            FittedModel::Gp(model) => model.ard_theta()?,
            FittedModel::Ridge { .. } | FittedModel::Lgbm(_) => return None,
        };
        if theta.is_empty() || theta.iter().any(|t| !t.is_finite()) {
            return None;
        }
        let sum: f64 = theta.iter().sum();
        // theta は上で有限性を確認済みなので sum も有限。正でなければ正規化できない。
        if sum <= 0.0 {
            return None;
        }
        Some(theta.iter().map(|t| t / sum).collect())
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
        SurrogateModelKind::GpFitc => FittedModel::Gp(Box::new(
            GpModel::fit(&x_norm, &y_norm, GpMethod::Fitc, 100, 42)
                .ok_or("GP-FITC training failed")?,
        )),
        SurrogateModelKind::GpVfe => FittedModel::Gp(Box::new(
            GpModel::fit(&x_norm, &y_norm, GpMethod::Vfe, 100, 42)
                .ok_or("GP-VFE training failed")?,
        )),
        SurrogateModelKind::GpMoe => FittedModel::Gp(Box::new(
            GpModel::fit(&x_norm, &y_norm, GpMethod::Moe, 100, 42)
                .ok_or("GP-MOE training failed")?,
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

/// 制約サロゲートを学習する。基本は目的関数と同じ `kind` を使うが、GP 系の学習が
/// 失敗した場合は Ridge へフォールバックする。
///
/// 完全に線形・ノイズゼロな制約（例: `c = 0.5 - x`）では GP のハイパーパラメータ
/// 最適化が退化し（最適 lengthscale → ∞）学習に失敗しうる。その制約だけ Ridge に
/// 落とせば（実行可能性確率はハード指標になるが）機能全体は継続でき、他の制約は
/// GP の平滑な P(c ≤ 0) を保てる。
pub(crate) fn fit_constraint_surrogate(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    values: &[f64],
) -> Result<FittedSurrogate, String> {
    match fit_surrogate(kind, x_matrix, values) {
        Ok(m) => Ok(m),
        Err(e) if kind != SurrogateModelKind::Ridge => {
            fit_surrogate(SurrogateModelKind::Ridge, x_matrix, values).map_err(|ridge_err| {
                format!("{kind:?} failed ({e}); Ridge fallback also failed ({ridge_err})")
            })
        }
        Err(e) => Err(e),
    }
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
