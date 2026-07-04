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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// テスト用解析的モックサロゲートが保持する closed-form クロージャ型（平均・分散共通）。
#[cfg(test)]
pub(crate) type AnalyticFn = Box<dyn Fn(&[f64]) -> f64 + Send + Sync>;

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
    /// テスト専用: 既知の closed-form 関数を返す解析的モック。
    ///
    /// GP フィットの代わりに同じインターフェースで応答曲面を注入するためのもの。
    /// 曲面が解析的に既知なので、最適化・獲得関数・実行可能性などの「曲面を使う処理」を
    /// 緩い許容ではなく厳密に検証できる。`var` が Some なら GP 系（事後分散あり）として
    /// 振る舞い、None なら Ridge / LightGBM 同様に事後分散を持たないモデルを表す。
    #[cfg(test)]
    Analytic {
        mean: AnalyticFn,
        var: Option<AnalyticFn>,
    },
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
            #[cfg(test)]
            FittedModel::Analytic { mean, .. } => mean(x_norm),
        }
    }

    /// 正規化空間での予測分散（事後分散を持つモデルのみ）。
    /// ガウス過程 3 方式（FITC / VFE / MoE）はすべて Some を返す。
    pub(crate) fn predict_var_norm(&self, x_norm: &[f64]) -> Option<f64> {
        match &self.model {
            FittedModel::Ridge { .. } | FittedModel::Lgbm(_) => None,
            FittedModel::Gp(model) => Some(model.predict_variance(x_norm)),
            #[cfg(test)]
            FittedModel::Analytic { var, .. } => var.as_ref().map(|f| f(x_norm)),
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
            #[cfg(test)]
            FittedModel::Analytic { .. } => return None,
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

/// 各列を [0,1] へ正規化する。`bounds[d] = Some((lo, hi))`（lo<hi・有限）の列は宣言
/// レンジで、それ以外は観測 min/max で正規化する。宣言レンジを使うと最適化の探索箱
/// （正規化空間 [0,1]^d）が log 由来の真の変数範囲に一致し、観測データの外（未観測だが
/// 有効な領域）も探索できる一方、`to_original_x` のクランプで範囲外へは出ない。
fn normalize_x_box(
    x_matrix: &[Vec<f64>],
    bounds: Option<&[Option<(f64, f64)>]>,
) -> (Vec<(f64, f64)>, Vec<Vec<f64>>) {
    let (observed_stats, _) = normalize_x_minmax(x_matrix);
    let col_stats: Vec<(f64, f64)> = observed_stats
        .iter()
        .enumerate()
        .map(
            |(d, &obs)| match bounds.and_then(|b| b.get(d)).copied().flatten() {
                Some((lo, hi)) if lo.is_finite() && hi.is_finite() && hi > lo => {
                    (lo, (hi - lo).max(f64::EPSILON))
                }
                _ => obs,
            },
        )
        .collect();
    let x_norm = x_matrix
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(d, &v)| {
                    let (min, range) = col_stats[d];
                    (v - min) / range
                })
                .collect()
        })
        .collect();
    (col_stats, x_norm)
}

/// 指定モデルでサロゲートを学習する。
pub(crate) fn fit_surrogate(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
) -> Result<FittedSurrogate, String> {
    // 優先行なし・観測レンジ正規化（従来動作）にデリゲートする。
    fit_surrogate_with_priority(kind, x_matrix, y, &[])
}

/// `fit_surrogate` と同じだが、GP 系では `priority`（誘導点として優先する行 index）を
/// パレートフロント等に集中させて学習する。N > GP の誘導点上限のときのみ効果がある。
/// Ridge / LightGBM は `priority` を無視する。
pub(crate) fn fit_surrogate_with_priority(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    priority: &[usize],
) -> Result<FittedSurrogate, String> {
    fit_surrogate_with_priority_bounds(kind, x_matrix, y, priority, None)
}

/// [`fit_surrogate_with_priority`] と同じだが、`bounds` で各列の宣言レンジを指定できる。
/// 与えた列はその範囲で正規化し（= 探索箱が真の変数範囲に一致）、無い列は観測レンジに
/// フォールバックする。
pub(crate) fn fit_surrogate_with_priority_bounds(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    priority: &[usize],
    bounds: Option<&[Option<(f64, f64)>]>,
) -> Result<FittedSurrogate, String> {
    let (col_stats, x_norm) = normalize_x_box(x_matrix, bounds);
    let (y_mean, y_std, y_norm) = normalize_y(y);

    let model = match kind {
        SurrogateModelKind::Ridge => fit_ridge(&x_norm, &y_norm)?,
        SurrogateModelKind::GpFitc => FittedModel::Gp(Box::new(
            GpModel::fit_front_focused(&x_norm, &y_norm, GpMethod::Fitc, 100, 42, priority)
                .ok_or("GP-FITC training failed")?,
        )),
        SurrogateModelKind::GpVfe => FittedModel::Gp(Box::new(
            GpModel::fit_front_focused(&x_norm, &y_norm, GpMethod::Vfe, 100, 42, priority)
                .ok_or("GP-VFE training failed")?,
        )),
        SurrogateModelKind::GpMoe => FittedModel::Gp(Box::new(
            GpModel::fit_front_focused(&x_norm, &y_norm, GpMethod::Moe, 100, 42, priority)
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
    fit_constraint_surrogate_bounds(kind, x_matrix, values, None)
}

/// [`fit_constraint_surrogate`] と同じだが、`bounds` で各列の宣言レンジを指定できる。
/// 制約サロゲートは最適化中に目的サロゲートと同じ正規化空間で評価されるため、目的と
/// 同一の `bounds` を渡して正規化箱を一致させる必要がある。
pub(crate) fn fit_constraint_surrogate_bounds(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    values: &[f64],
    bounds: Option<&[Option<(f64, f64)>]>,
) -> Result<FittedSurrogate, String> {
    match fit_surrogate_with_priority_bounds(kind, x_matrix, values, &[], bounds) {
        Ok(m) => Ok(m),
        Err(e) if kind != SurrogateModelKind::Ridge => fit_surrogate_with_priority_bounds(
            SurrogateModelKind::Ridge,
            x_matrix,
            values,
            &[],
            bounds,
        )
        .map_err(|ridge_err| {
            format!("{kind:?} failed ({e}); Ridge fallback also failed ({ridge_err})")
        }),
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

#[cfg(test)]
impl FittedSurrogate {
    /// テスト用の解析的モックサロゲートを作る。
    ///
    /// 正規化を恒等（`col_stats = (0, 1)`、`y_mean = 0`、`y_std = 1`）に固定するため、
    /// 正規化空間 [0,1]^d がそのまま元の単位空間に一致する。したがって `mean` / `var`
    /// クロージャの出力がそのまま元単位の予測平均・予測分散となり、既知の closed-form
    /// 応答曲面を GP フィットなしで注入できる。`var` が `Some` なら GP 系（事後分散あり）
    /// として、`None` なら Ridge / LightGBM 同様に事後分散を持たないモデルとして振る舞う。
    pub(crate) fn analytic(
        n_dims: usize,
        mean: impl Fn(&[f64]) -> f64 + Send + Sync + 'static,
        var: Option<AnalyticFn>,
    ) -> Self {
        FittedSurrogate {
            model: FittedModel::Analytic {
                mean: Box::new(mean),
                var,
            },
            col_stats: vec![(0.0, 1.0); n_dims],
            y_mean: 0.0,
            y_std: 1.0,
            r_squared: 1.0,
        }
    }
}
