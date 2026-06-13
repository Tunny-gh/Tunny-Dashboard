//! 獲得関数（Acquisition Function）によるサロゲートモデルからの次候補提案。
//!
//! ガウス過程（GP）サロゲートの事後平均・分散を使い、ベイズ最適化の 1 ステップ相当の
//! 候補点を提案する。バッチ候補は Constant Liar 戦略で生成する。
//!
//! 正規化空間 [0,1]^d・z-score 目的空間で全計算を行い、結果を元の単位へ変換して返す。

use super::feasibility::feasibility_probability;
use super::models::{fit_constraint_surrogate, fit_surrogate, FittedSurrogate};
use super::optimizers::minimize_scalar_fn;
use super::{best_observed_index, TrainedSurrogate};

/// 獲得関数の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquisitionKind {
    /// Expected Improvement（期待改善量）。
    ExpectedImprovement,
    /// Lower Confidence Bound（下限信頼境界）。
    LowerConfidenceBound,
}

/// EI の探索オフセット（z-score 単位）。
const XI: f64 = 0.01;
/// LCB の探索係数 κ。
const KAPPA: f64 = 2.0;
/// 制約付き LCB のペナルティ重み（z-score 単位）。
const CONSTRAINT_LCB_PENALTY: f64 = 10.0;

/// 獲得関数の最適化によって提案された 1 候補点。パラメータ値・予測値は元の単位。
#[derive(Debug, Clone)]
pub struct SuggestedCandidate {
    /// パラメータ値（元の単位、`param_names` と同順）。
    pub params: Vec<f64>,
    /// サロゲート予測値（元の単位）。
    pub predicted_value: f64,
    /// 予測標準偏差（GP 系のみ Some; 元の単位へスケール済み）。
    pub predicted_std: Option<f64>,
    /// 獲得スコア（最大化方向、値が大きいほど有望）。
    pub acq_score: f64,
    /// 制約サロゲートの予測値（元の単位、`constraint_names` と同順）。制約なしのときは空。
    pub predicted_constraints: Vec<f64>,
    /// 実行可能性確率（0.0〜1.0）。制約なしのときは None。
    pub feasibility_probability: Option<f64>,
}

/// 標準正規分布の CDF Φ(z)。
///
/// Abramowitz & Stegun 式 7.1.26 の erf 近似を使用する。
/// 誤差は |ε| < 1.5 × 10⁻⁷。
pub(crate) fn normal_cdf(z: f64) -> f64 {
    // erf(x) ≈ 1 - (a1·t + a2·t² + a3·t³ + a4·t⁴ + a5·t⁵)·exp(-x²)  (t = 1/(1+0.3275911·x))
    // Φ(z) = 0.5 · (1 + erf(z / √2))
    let x = z / std::f64::consts::SQRT_2;
    let sign = if x < 0.0 { -1.0f64 } else { 1.0f64 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let poly = t
        * (0.254829592
            + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    let erf_abs = 1.0 - poly * (-x * x).exp();
    0.5 * (1.0 + sign * erf_abs)
}

/// 標準正規分布の PDF φ(z)。
fn normal_pdf(z: f64) -> f64 {
    (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// 正規化空間における EI（期待改善量）を計算する。最大化方向。
///
/// `f_best`: 訓練データ中の最良 z-score 値（最小化なら最小値、最大化なら最小値（符号反転済み））。
/// `mu`: サロゲートの事後平均（z-score）。
/// `sigma`: 事後標準偏差（z-score）。
/// `minimize`: true なら最小化問題。
fn ei_norm(f_best: f64, mu: f64, sigma: f64) -> f64 {
    // 最小化として扱う（f_best は minimize/maximize の符号変換済み）。
    if sigma < 1e-12 {
        return (f_best - mu - XI).max(0.0);
    }
    let i = f_best - mu - XI;
    let z = i / sigma;
    i * normal_cdf(z) + sigma * normal_pdf(z)
}

/// 正規化空間における LCB を計算する。最小化スコア（小さいほど有望）。
///
/// `mu` / `sigma` は z-score 単位。
fn lcb_norm(mu: f64, sigma: f64) -> f64 {
    mu - KAPPA * sigma
}

/// インカンバント（z-score 空間の最良値）を取得する。
///
/// 制約がある場合は実行可能な trial のみを対象とする（全 trial が実行不可能なら全体で計算）。
/// 最小化ならば z-score の最小値、最大化ならば z-score の最大値の符号反転（= −最大値）。
/// 内部的には常に「最小化」の世界で動くため、maximize は負の符号を使う。
fn incumbent(
    surrogate: &FittedSurrogate,
    y: &[f64],
    minimize: bool,
    constraint_values: &[Vec<f64>],
) -> f64 {
    // y は元の単位なので z-score へ変換する。
    let y_norm: Vec<f64> = y
        .iter()
        .map(|&v| (v - surrogate.y_mean) / surrogate.y_std)
        .collect();

    // 実行可能 trial のインデックス: すべての制約値 ≤ 0。
    let feasible_indices: Vec<usize> = if constraint_values.is_empty() {
        (0..y_norm.len()).collect()
    } else {
        (0..y_norm.len())
            .filter(|&i| {
                constraint_values
                    .get(i)
                    .is_none_or(|cv| cv.iter().all(|&c| c <= 0.0))
            })
            .collect()
    };

    // 実行可能な trial が存在するならその中から選ぶ、なければ全体から選ぶ。
    let indices = if feasible_indices.is_empty() {
        (0..y_norm.len()).collect::<Vec<_>>()
    } else {
        feasible_indices
    };

    if minimize {
        indices
            .iter()
            .map(|&i| y_norm[i])
            .fold(f64::INFINITY, f64::min)
    } else {
        // maximize → 符号反転して最小化問題として扱う。
        indices
            .iter()
            .map(|&i| -y_norm[i])
            .fold(f64::INFINITY, f64::min)
    }
}

/// 訓練済みサロゲートから次試行の候補点を提案する。
///
/// - `trained`: 検証済みの学習結果（GP 系のみ対応）。
/// - `n_candidates`: 提案候補点数（≥ 1）。
/// - `acquisition`: 使用する獲得関数。
/// - `minimize`: true = 最小化問題、false = 最大化問題。
///
/// バッチ（n > 1）は Constant Liar 戦略: 1 候補ずつ追加後に「嘘の」観測値
/// （最良観測値）を付加して GP を再フィットし、次候補を探索する。
/// 制約モデルも同時に再フィットし、候補の予測制約平均値を嘘値として付加する。
pub fn suggest_candidates(
    trained: &TrainedSurrogate,
    n_candidates: usize,
    acquisition: AcquisitionKind,
    minimize: bool,
) -> Result<Vec<SuggestedCandidate>, String> {
    if n_candidates == 0 {
        return Err("n_candidates must be ≥ 1".to_string());
    }

    // GP 系かどうかを確認する（事後分散が必要）。
    let probe = trained
        .x_matrix
        .first()
        .map(|row| trained.surrogate.to_norm_x(row));
    let has_variance = probe
        .as_deref()
        .and_then(|xn| trained.surrogate.predict_var_norm(xn))
        .is_some();
    if !has_variance {
        return Err(
            "acquisition requires a Gaussian Process model (GP-FITC, GP-VFE, or GP-MOE)"
                .to_string(),
        );
    }

    let has_constraints = !trained.constraint_models.is_empty();
    let n_dims = trained.surrogate.col_stats.len();
    let mut candidates: Vec<SuggestedCandidate> = Vec::with_capacity(n_candidates);

    // Constant Liar 用の作業コピー。
    let mut work_x = trained.x_matrix.clone();
    let mut work_y = trained.y.clone();
    // 制約の Constant Liar 用作業コピー（制約ごとの列）。
    let mut work_c: Vec<Vec<f64>> = trained
        .constraint_models
        .iter()
        .enumerate()
        .map(|(ci, _)| {
            trained
                .constraint_values
                .iter()
                .map(|row| row.get(ci).copied().unwrap_or(0.0))
                .collect()
        })
        .collect();

    // Constant Liar の「嘘」は現在の最良観測値（minimize なら最小、maximize なら最大）。
    let lie_y = {
        let best_idx = best_observed_index(&trained.y, minimize);
        trained.y[best_idx]
    };

    // 各反復で使うサロゲートを所有権付きで保持する Vec。
    // 最初の要素はダミー（i=0 では trained.surrogate を直接使う）。
    // i >= 1 では refitted[i-1] を参照する。
    let mut refitted: Vec<FittedSurrogate> = Vec::new();
    // 制約の再フィット済みモデル群（制約 × 反復）。
    // refitted_constraints[i-1][ci] が i 番目の反復で使う制約 ci のサロゲート。
    let mut refitted_constraints: Vec<Vec<FittedSurrogate>> = Vec::new();

    for i in 0..n_candidates {
        // 今回の反復で使うサロゲートへの参照を取得する。
        let (params_orig, predicted_value, predicted_std, acq_score, pred_constraints, p_feas) = {
            let surrogate: &FittedSurrogate = if i == 0 {
                &trained.surrogate
            } else {
                &refitted[i - 1]
            };
            // 制約サロゲートへの参照。
            let c_models: &[FittedSurrogate] = if i == 0 {
                &trained.constraint_models
            } else {
                &refitted_constraints[i - 1]
            };

            // work_y と work_c から制約値行列を再構築する（Constant Liar 追記分を含む）。
            let work_constraint_values: Vec<Vec<f64>> = (0..work_y.len())
                .map(|row| {
                    (0..c_models.len())
                        .map(|ci| {
                            work_c
                                .get(ci)
                                .and_then(|col| col.get(row))
                                .copied()
                                .unwrap_or(0.0)
                        })
                        .collect()
                })
                .collect();

            let f_best = incumbent(surrogate, &work_y, minimize, &work_constraint_values);

            // 獲得関数（最小化方向）を構築する。
            // 制約付き EI: EI(x) * P_feas(x)
            // 制約付き LCB: LCB(x) + CONSTRAINT_LCB_PENALTY * (1 - P_feas(x))
            let eval_acq = |x_norm: &[f64]| -> f64 {
                let mu = if minimize {
                    surrogate.predict_norm(x_norm)
                } else {
                    -surrogate.predict_norm(x_norm)
                };
                let sigma = surrogate
                    .predict_var_norm(x_norm)
                    .map(|v| v.max(0.0).sqrt())
                    .unwrap_or(0.0);
                let p = if has_constraints {
                    feasibility_probability(c_models, x_norm)
                } else {
                    1.0
                };
                match acquisition {
                    // 制約付き EI: -EI * P_feas（最小化方向）
                    AcquisitionKind::ExpectedImprovement => -ei_norm(f_best, mu, sigma) * p,
                    // 制約付き LCB: LCB + penalty * (1 - P_feas)
                    AcquisitionKind::LowerConfidenceBound => {
                        lcb_norm(mu, sigma) + CONSTRAINT_LCB_PENALTY * (1.0 - p)
                    }
                }
            };

            // 開始点: 現在の観測ベスト点（正規化空間）。
            let best_idx = best_observed_index(&work_y, minimize);
            let start_norm = surrogate.to_norm_x(&work_x[best_idx]);

            // 重複ガード: 前の候補との L2 距離が 1e-6 以下なら再試行。
            let best_norm = {
                let cand = minimize_scalar_fn(&eval_acq, n_dims, &start_norm);
                let is_dup = candidates.iter().any(|prev| {
                    let prev_norm = surrogate.to_norm_x(&prev.params);
                    let dist2: f64 = cand
                        .iter()
                        .zip(prev_norm.iter())
                        .map(|(a, b)| (a - b).powi(2))
                        .sum();
                    dist2 < 1e-12
                });
                if is_dup {
                    // 再試行: 別シードのランダムスタートで再探索。
                    let mut rng = crate::math::rng::SeededRng::from_seed(42 + i as u64 + 1);
                    let alt_start: Vec<f64> = (0..n_dims).map(|_| rng.next_f64()).collect();
                    minimize_scalar_fn(&eval_acq, n_dims, &alt_start)
                } else {
                    cand
                }
            };

            // 候補を元の単位へ変換する。
            let params_orig = surrogate.to_original_x(&best_norm);
            let mu_norm = surrogate.predict_norm(&best_norm);
            let predicted_value = surrogate.to_original_y(mu_norm);
            let predicted_std = surrogate
                .predict_var_norm(&best_norm)
                .map(|v| v.max(0.0).sqrt() * surrogate.y_std);

            // 獲得スコアは最大化方向（大きいほど有望）。
            let acq_score = {
                let mu = if minimize { mu_norm } else { -mu_norm };
                let sigma = surrogate
                    .predict_var_norm(&best_norm)
                    .map(|v| v.max(0.0).sqrt())
                    .unwrap_or(0.0);
                let p = if has_constraints {
                    feasibility_probability(c_models, &best_norm)
                } else {
                    1.0
                };
                match acquisition {
                    AcquisitionKind::ExpectedImprovement => ei_norm(f_best, mu, sigma) * p,
                    AcquisitionKind::LowerConfidenceBound => {
                        -lcb_norm(mu, sigma) - CONSTRAINT_LCB_PENALTY * (1.0 - p)
                    }
                }
            };

            // 制約予測値と実行可能性確率を計算する。
            let (pred_constraints, p_feas) = if has_constraints {
                let preds: Vec<f64> = c_models
                    .iter()
                    .map(|cm| cm.to_original_y(cm.predict_norm(&best_norm)))
                    .collect();
                let p = feasibility_probability(c_models, &best_norm);
                (preds, Some(p))
            } else {
                (vec![], None)
            };

            (
                params_orig,
                predicted_value,
                predicted_std,
                acq_score,
                pred_constraints,
                p_feas,
            )
        };
        // surrogate への借用はここで終了する。

        candidates.push(SuggestedCandidate {
            params: params_orig.clone(),
            predicted_value,
            predicted_std,
            acq_score,
            predicted_constraints: pred_constraints.clone(),
            feasibility_probability: p_feas,
        });

        // Constant Liar: 次の候補のために作業データへ追加して再フィット。
        if i + 1 < n_candidates {
            work_x.push(params_orig);
            work_y.push(lie_y);
            // 制約の嘘値: 候補の制約予測平均値を使う。
            for (ci, col) in work_c.iter_mut().enumerate() {
                let lie_c = pred_constraints.get(ci).copied().unwrap_or(0.0);
                col.push(lie_c);
            }
            match fit_surrogate(trained.model_kind, &work_x, &work_y) {
                Ok(new_surrogate) => {
                    refitted.push(new_surrogate);
                }
                Err(_) => {
                    // 再フィット失敗 → これまでの候補を Ok で返す。
                    return Ok(candidates);
                }
            }
            // 制約モデルも再フィットする。
            let mut new_c_models = Vec::with_capacity(work_c.len());
            let mut constraint_refit_ok = true;
            for col in &work_c {
                match fit_constraint_surrogate(trained.model_kind, &work_x, col) {
                    Ok(cm) => new_c_models.push(cm),
                    Err(_) => {
                        constraint_refit_ok = false;
                        break;
                    }
                }
            }
            if constraint_refit_ok {
                refitted_constraints.push(new_c_models);
            } else {
                // 制約モデル再フィット失敗 → これまでの候補を Ok で返す。
                return Ok(candidates);
            }
        }
    }

    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::rng::SeededRng;
    use crate::surrogate_opt::models::SurrogateModelKind;
    use crate::surrogate_opt::{fit_surrogate_with_validation, SurrogateFitRequest};

    // ── normal_cdf のユニットテスト ─────────────────────────────────

    #[test]
    fn normal_cdf_at_zero() {
        let v = normal_cdf(0.0);
        assert!((v - 0.5).abs() < 1e-4, "Φ(0) = 0.5, got {v}");
    }

    #[test]
    fn normal_cdf_at_1_96() {
        let v = normal_cdf(1.96);
        assert!((v - 0.975).abs() < 1e-4, "Φ(1.96) ≈ 0.975, got {v}");
    }

    #[test]
    fn normal_cdf_negative_1() {
        let v = normal_cdf(-1.0);
        assert!((v - 0.1587).abs() < 1e-4, "Φ(-1) ≈ 0.1587, got {v}");
    }

    // ── EI のプロパティテスト ────────────────────────────────────────

    #[test]
    fn ei_zero_when_sigma_tiny_and_mu_above_fbest() {
        // σ → 0, μ > f_best → EI ≈ 0。
        let f_best = 0.0;
        let mu = 1.0; // μ > f_best なので改善なし
        let sigma = 1e-15;
        let ei = ei_norm(f_best, mu, sigma);
        assert!(
            ei < 1e-6,
            "EI should be near 0 when σ→0 and μ > f_best, got {ei}"
        );
    }

    #[test]
    fn ei_grows_with_sigma_at_fixed_mu() {
        // σ が大きいほど EI が大きい。
        let f_best = 0.0;
        let mu = 0.0;
        let ei_small = ei_norm(f_best, mu, 0.01);
        let ei_large = ei_norm(f_best, mu, 1.0);
        assert!(
            ei_large > ei_small,
            "EI should grow with σ: ei(σ=0.01)={ei_small}, ei(σ=1.0)={ei_large}"
        );
    }

    // ── GP-FITC 2D 二次関数上の候補提案テスト ─────────────────────

    fn quadratic_trained_fitc(n: usize) -> TrainedSurrogate {
        let mut rng = SeededRng::from_seed(7);
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|_| vec![rng.next_f64(), rng.next_f64()])
            .collect();
        let y: Vec<f64> = x_matrix
            .iter()
            .map(|r| (r[0] - 0.3).powi(2) + (r[1] - 0.7).powi(2))
            .collect();
        fit_surrogate_with_validation(&SurrogateFitRequest {
            x_matrix,
            y,
            param_names: vec!["x".to_string(), "y".to_string()],
            objective_name: "obj".to_string(),
            model: SurrogateModelKind::GpFitc,
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
        })
        .expect("fit should succeed")
    }

    #[test]
    fn single_ei_candidate_in_unit_box_with_std() {
        let trained = quadratic_trained_fitc(50);
        let candidates =
            suggest_candidates(&trained, 1, AcquisitionKind::ExpectedImprovement, true)
                .expect("suggest should succeed");
        assert_eq!(candidates.len(), 1);
        let c = &candidates[0];
        // 提案パラメータは元の単位で [0,1] 内にあるはず。
        assert!(
            c.params.iter().all(|&v| (0.0..=1.0).contains(&v)),
            "params out of [0,1]: {:?}",
            c.params
        );
        // acq_score >= 0（EI は非負）。
        assert!(c.acq_score >= 0.0, "EI should be ≥ 0, got {}", c.acq_score);
        // GP なので predicted_std は Some。
        assert!(c.predicted_std.is_some(), "GP should have predicted_std");
    }

    #[test]
    fn batch_3_candidates_pairwise_diverse() {
        let trained = quadratic_trained_fitc(50);
        let candidates =
            suggest_candidates(&trained, 3, AcquisitionKind::ExpectedImprovement, true)
                .expect("batch suggest should succeed");
        assert_eq!(candidates.len(), 3);

        // ペアワイズ正規化 L2 距離 > 1e-4。
        let surrogate = &trained.surrogate;
        for i in 0..3 {
            for j in (i + 1)..3 {
                let ni = surrogate.to_norm_x(&candidates[i].params);
                let nj = surrogate.to_norm_x(&candidates[j].params);
                let dist: f64 = ni
                    .iter()
                    .zip(nj.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(
                    dist > 1e-4,
                    "candidates {i} and {j} too close (dist={dist:.2e})"
                );
            }
        }
    }

    #[test]
    fn batch_3_deterministic_across_two_runs() {
        let trained = quadratic_trained_fitc(50);
        let c1 = suggest_candidates(&trained, 3, AcquisitionKind::ExpectedImprovement, true)
            .expect("first run");
        let c2 = suggest_candidates(&trained, 3, AcquisitionKind::ExpectedImprovement, true)
            .expect("second run");
        assert_eq!(c1.len(), c2.len());
        for (a, b) in c1.iter().zip(c2.iter()) {
            for (pa, pb) in a.params.iter().zip(b.params.iter()) {
                assert!(
                    (pa - pb).abs() < 1e-9,
                    "results differ between runs: {pa} vs {pb}"
                );
            }
        }
    }

    #[test]
    fn maximize_steers_away_from_minimum() {
        // maximize=true では、二次関数の最大値（隅）に向かうはず。
        let trained = quadratic_trained_fitc(50);
        let y_median = {
            let mut ys = trained.y.clone();
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            ys[ys.len() / 2]
        };
        let candidates =
            suggest_candidates(&trained, 1, AcquisitionKind::ExpectedImprovement, false)
                .expect("maximize suggest");
        // 予測値がメジアン以上（最大化方向に動いている）。
        assert!(
            candidates[0].predicted_value >= y_median - 0.1,
            "maximize should steer toward higher values, got {}",
            candidates[0].predicted_value
        );
    }

    #[test]
    fn ridge_model_returns_error() {
        let mut rng = SeededRng::from_seed(7);
        let x_matrix: Vec<Vec<f64>> = (0..20)
            .map(|_| vec![rng.next_f64(), rng.next_f64()])
            .collect();
        let y: Vec<f64> = x_matrix.iter().map(|r| r[0] + r[1]).collect();
        let trained = fit_surrogate_with_validation(&SurrogateFitRequest {
            x_matrix,
            y,
            param_names: vec!["x".to_string(), "y".to_string()],
            objective_name: "obj".to_string(),
            model: SurrogateModelKind::Ridge,
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
        })
        .expect("ridge fit should succeed");

        let err = suggest_candidates(&trained, 1, AcquisitionKind::ExpectedImprovement, true)
            .unwrap_err();
        assert!(
            err.contains("Gaussian Process"),
            "expected GP error, got: {err}"
        );
    }
}
