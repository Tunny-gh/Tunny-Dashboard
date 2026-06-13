//! 期待ハイパーボリューム改善（EHVI: Expected Hypervolume Improvement）による
//! 多目的サロゲートからの次候補提案。
//!
//! 単目的の獲得関数（`acquisition.rs`）の多目的アナログ。各目的は独立した GP
//! サロゲートを持ち、その事後平均・分散から、観測パレートフロントへの
//! ハイパーボリューム改善期待値をモンテカルロ推定して最大化する。
//!
//! ## z-score 最小化フレーム
//!
//! すべての EHVI 計算は z-score 化した最小化フレームで行う。目的 k について
//! 正規化目的 `g_k(x) = sign_k * predict_norm_k(x)` を定義する。ここで
//! `sign_k = +1`（最小化）/ `-1`（最大化）であり、g は常に小さいほど良い。
//! 事後標準偏差 `s_k(x) = sqrt(predict_var_norm_k(x))`（符号は std に影響しない）。
//!
//! ## 決定性（共通乱数）
//!
//! 1 回の `suggest_candidates_multi` 呼び出しにつき、固定シード RNG から
//! `S × n_obj` の標準正規行列を **一度だけ** 引き、すべての x 評価で再利用する
//! （Common Random Numbers）。これにより MC-EHVI は x の決定的で滑らかな関数となり、
//! L-BFGS の数値勾配が有効になる。評価ごとに引き直さない。
//!
//! ## 参照点
//!
//! 観測フロント P から、次元ごとに `r_k = max_{p∈P} g_k(p) + REF_MARGIN`
//! （nadir + マージン、z-score 単位）。

use super::models::{fit_surrogate, FittedSurrogate};
use super::optimizers::minimize_scalar_fn;
use super::TrainedSurrogate;
use crate::math::rng::SeededRng;
use crate::multi_objective::pareto::hypervolume_nd;

/// 参照点の nadir からのマージン（z-score 単位）。
const REF_MARGIN: f64 = 0.1;
/// MC-EHVI のサンプル数（共通乱数行列の行数）。
const EHVI_SAMPLES: usize = 128;
/// 共通乱数行列の固定シード。
const EHVI_SEED: u64 = 42;

/// EHVI の最適化によって提案された 1 候補点。パラメータ値・予測値は元の単位。
#[derive(Debug, Clone)]
pub struct MultiSuggestedCandidate {
    /// パラメータ値（元の単位、`param_names` と同順）。
    pub params: Vec<f64>,
    /// 目的ごとのサロゲート予測値（元の単位、`objective_names` と同順）。
    pub predicted_values: Vec<f64>,
    /// 目的ごとの予測標準偏差（GP 系のみ Some、元の単位）。
    pub predicted_stds: Vec<Option<f64>>,
    /// EHVI スコア（最大化方向、大きいほど有望）。
    pub ehvi_score: f64,
}

/// EHVI 計算の作業コンテキスト（1 反復ぶん）。
struct EhviContext<'a> {
    /// 目的ごとのサロゲート（`predict_norm` / `predict_var_norm` を提供）。
    surrogates: Vec<&'a FittedSurrogate>,
    /// 目的ごとの符号（最小化 = +1.0、最大化 = -1.0）。
    signs: Vec<f64>,
    /// 観測パレートフロント P（z-score 最小化フレーム、非劣解のみ）。
    front: Vec<Vec<f64>>,
    /// 参照点 r（z-score 最小化フレーム）。
    ref_point: Vec<f64>,
    /// HV(P)（事前計算、P 固定）。
    hv_p: f64,
    /// 固定された S×n_obj 標準正規行列（共通乱数）。
    z_matrix: &'a [Vec<f64>],
}

impl EhviContext<'_> {
    /// 正規化空間の点 x における MC-EHVI を計算する（最大化方向、≥ 0）。
    fn ehvi(&self, x_norm: &[f64]) -> f64 {
        let n_obj = self.surrogates.len();
        // g_k(x), s_k(x) を事前評価する。
        let g: Vec<f64> = (0..n_obj)
            .map(|k| self.signs[k] * self.surrogates[k].predict_norm(x_norm))
            .collect();
        let s: Vec<f64> = (0..n_obj)
            .map(|k| {
                self.surrogates[k]
                    .predict_var_norm(x_norm)
                    .map(|v| v.max(0.0).sqrt())
                    .unwrap_or(0.0)
            })
            .collect();

        let s_samples = self.z_matrix.len();
        if s_samples == 0 {
            return 0.0;
        }

        // 各サンプルで v_s[k] = g_k + s_k * Z[s][k] を作り、HV(P ∪ {v_s}) − HV(P) を加算。
        let mut acc = 0.0;
        let mut augmented: Vec<Vec<f64>> = Vec::with_capacity(self.front.len() + 1);
        for z_row in self.z_matrix {
            let v_s: Vec<f64> = (0..n_obj).map(|k| g[k] + s[k] * z_row[k]).collect();
            augmented.clear();
            augmented.extend_from_slice(&self.front);
            augmented.push(v_s);
            let hv_aug = hypervolume_nd(&augmented, &self.ref_point);
            let improvement = hv_aug - self.hv_p;
            if improvement > 0.0 {
                acc += improvement;
            }
        }
        acc / s_samples as f64
    }
}

/// 観測値（目的ごとの raw y）から z-score 最小化フレームのパレートフロント P を作る。
///
/// 各 trial を `sign_k * (y_k - y_mean_k) / y_std_k` に変換し、最小化規約で
/// 非劣解（dominated でない点）のみを残す。
fn build_observed_front(
    surrogates: &[&FittedSurrogate],
    ys: &[&[f64]],
    signs: &[f64],
) -> Vec<Vec<f64>> {
    let n_obj = surrogates.len();
    let n = ys.first().map(|y| y.len()).unwrap_or(0);

    // z-score 最小化フレームへ変換。
    let points: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n_obj)
                .map(|k| {
                    let s = surrogates[k];
                    let z = if s.y_std > 1e-12 {
                        (ys[k][i] - s.y_mean) / s.y_std
                    } else {
                        0.0
                    };
                    signs[k] * z
                })
                .collect()
        })
        .collect();

    // 非劣解の抽出（最小化: a が b に支配される ⟺ 全次元 a ≥ b かつ 1 次元で a > b）。
    let mut front: Vec<Vec<f64>> = Vec::new();
    for p in &points {
        let dominated = points.iter().any(|q| dominates(q, p));
        if !dominated {
            // 重複点を除く。
            let dup = front
                .iter()
                .any(|f| f.iter().zip(p.iter()).all(|(a, b)| (a - b).abs() < 1e-12));
            if !dup {
                front.push(p.clone());
            }
        }
    }
    front
}

/// 最小化規約で q が p を支配するか（q ≤ p 全次元 かつ q < p いずれか）。
fn dominates(q: &[f64], p: &[f64]) -> bool {
    let mut strictly_better = false;
    for (qi, pi) in q.iter().zip(p.iter()) {
        if qi > pi {
            return false;
        }
        if qi < pi {
            strictly_better = true;
        }
    }
    strictly_better
}

/// 参照点 r を計算する: 次元ごとに `max_{p∈P} g_k(p) + REF_MARGIN`。
fn compute_ref_point(front: &[Vec<f64>], n_obj: usize) -> Vec<f64> {
    (0..n_obj)
        .map(|k| {
            let nadir = front.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max);
            // フロントが空の場合のフォールバック（呼び出し側で空は除外済みだが安全に）。
            let base = if nadir.is_finite() { nadir } else { 0.0 };
            base + REF_MARGIN
        })
        .collect()
}

/// 固定シード RNG から S×n_obj の標準正規行列を引く（Box-Muller 変換）。
///
/// `next_f64` は [0,1) の一様乱数を返すため、対で標準正規を 2 個生成する。
fn draw_standard_normal_matrix(rows: usize, cols: usize) -> Vec<Vec<f64>> {
    let mut rng = SeededRng::from_seed(EHVI_SEED);
    let mut next_normal = move || -> f64 {
        // Box-Muller: u1 ∈ (0,1] を保証するため 0 を回避する。
        let u1 = (rng.next_f64()).max(f64::MIN_POSITIVE);
        let u2 = rng.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    };
    (0..rows)
        .map(|_| (0..cols).map(|_| next_normal()).collect())
        .collect()
}

/// 観測フロントのパラメータ平均（元の単位）を計算し、正規化空間の開始点を返す。
///
/// フロント点に対応する観測 trial を見つけられないため、全観測パラメータの平均
/// （元の単位）を堅牢な開始点とする（仕様の「a simple robust start is the mean of
/// observed param rows」）。
fn mean_param_start(surrogate: &FittedSurrogate, x_matrix: &[Vec<f64>], n_dims: usize) -> Vec<f64> {
    if x_matrix.is_empty() {
        return vec![0.5; n_dims];
    }
    let n = x_matrix.len() as f64;
    let mean: Vec<f64> = (0..n_dims)
        .map(|d| {
            x_matrix
                .iter()
                .map(|r| r.get(d).copied().unwrap_or(0.0))
                .sum::<f64>()
                / n
        })
        .collect();
    surrogate.to_norm_x(&mean)
}

/// 訓練済みサロゲート群から EHVI で次試行の候補点を提案する。
///
/// - `trained[k]`: 目的 k のサロゲート（全目的が GP 系であること）。
/// - `minimize[k]`: true = 目的 k を最小化。
/// - `n_candidates`: 提案候補点数（≥ 1）。
///
/// バッチ（n > 1）は Constant Liar 戦略: 候補を 1 つ選ぶごとに、その候補の
/// パラメータと目的ごとの予測平均（raw 単位）を各目的の (x, y) 作業コピーへ追加し、
/// 各目的サロゲートを再フィットして P・r を再計算し、次候補を再探索する。
pub fn suggest_candidates_multi(
    trained: &[TrainedSurrogate],
    minimize: &[bool],
    n_candidates: usize,
) -> Result<Vec<MultiSuggestedCandidate>, String> {
    if trained.is_empty() {
        return Err("EHVI requires at least one objective surrogate".to_string());
    }
    if trained.len() != minimize.len() {
        return Err("trained and minimize length mismatch".to_string());
    }
    if n_candidates == 0 {
        return Err("n_candidates must be ≥ 1".to_string());
    }

    let n_obj = trained.len();
    let n_dims = trained[0].surrogate.col_stats.len();
    // 全サロゲートが同一次元（同じ正規化変換）を持つことを確認する。
    if trained
        .iter()
        .any(|t| t.surrogate.col_stats.len() != n_dims)
    {
        return Err("trained surrogates have inconsistent dimensions".to_string());
    }

    // GP 系（事後分散あり）かどうかを各目的で確認する。
    for t in trained {
        let probe = t.x_matrix.first().map(|row| t.surrogate.to_norm_x(row));
        let has_var = probe
            .as_deref()
            .and_then(|xn| t.surrogate.predict_var_norm(xn))
            .is_some();
        if !has_var {
            return Err("EHVI requires Gaussian Process models for all objectives".to_string());
        }
    }

    let signs: Vec<f64> = minimize
        .iter()
        .map(|&m| if m { 1.0 } else { -1.0 })
        .collect();

    // 共通乱数行列を一度だけ引く（決定性・滑らかさのため）。
    let z_matrix = draw_standard_normal_matrix(EHVI_SAMPLES, n_obj);

    let model_kinds: Vec<_> = trained.iter().map(|t| t.model_kind).collect();
    // Constant Liar 用の作業コピー（目的ごとの x, y）。x は全目的共通。
    let mut work_x = trained[0].x_matrix.clone();
    let mut work_ys: Vec<Vec<f64>> = trained.iter().map(|t| t.y.clone()).collect();

    // 各反復で使う再フィット済みサロゲート（i=0 は trained を直接使う）。
    let mut refitted: Vec<Vec<FittedSurrogate>> = Vec::new();

    let mut candidates: Vec<MultiSuggestedCandidate> = Vec::with_capacity(n_candidates);

    for i in 0..n_candidates {
        // 今回の反復で使うサロゲート参照を取得する。
        let surrogates: Vec<&FittedSurrogate> = if i == 0 {
            trained.iter().map(|t| &t.surrogate).collect()
        } else {
            refitted[i - 1].iter().collect()
        };
        let ref_surrogate = surrogates[0];

        // 観測フロント P と参照点 r を（作業データから）再計算する。
        let ys_refs: Vec<&[f64]> = work_ys.iter().map(|y| y.as_slice()).collect();
        let front = build_observed_front(&surrogates, &ys_refs, &signs);
        let ref_point = compute_ref_point(&front, n_obj);
        let hv_p = hypervolume_nd(&front, &ref_point);

        let ctx = EhviContext {
            surrogates: surrogates.clone(),
            signs: signs.clone(),
            front,
            ref_point,
            hv_p,
            z_matrix: &z_matrix,
        };

        // 開始点: 観測パラメータ平均（正規化空間）。
        let start_norm = mean_param_start(ref_surrogate, &work_x, n_dims);

        // EHVI を最大化（= -ehvi を最小化）する。
        let neg_ehvi = |x: &[f64]| -ctx.ehvi(x);
        let mut best_norm = minimize_scalar_fn(&neg_ehvi, n_dims, &start_norm);

        // 重複ガード: 前の候補との正規化 L2 距離が 1e-6 未満なら別シードで再試行。
        let is_dup = candidates.iter().any(|prev| {
            let prev_norm = ref_surrogate.to_norm_x(&prev.params);
            let dist2: f64 = best_norm
                .iter()
                .zip(prev_norm.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            dist2 < 1e-12
        });
        if is_dup {
            let mut rng = SeededRng::from_seed(EHVI_SEED + i as u64 + 1);
            let alt_start: Vec<f64> = (0..n_dims).map(|_| rng.next_f64()).collect();
            best_norm = minimize_scalar_fn(&neg_ehvi, n_dims, &alt_start);
        }

        // 候補を元の単位へ変換する。
        let params = ref_surrogate.to_original_x(&best_norm);
        let predicted_values: Vec<f64> = surrogates
            .iter()
            .map(|s| s.to_original_y(s.predict_norm(&best_norm)))
            .collect();
        let predicted_stds: Vec<Option<f64>> = surrogates
            .iter()
            .map(|s| {
                s.predict_var_norm(&best_norm)
                    .map(|v| v.max(0.0).sqrt() * s.y_std)
            })
            .collect();
        let ehvi_score = ctx.ehvi(&best_norm);

        candidates.push(MultiSuggestedCandidate {
            params: params.clone(),
            predicted_values: predicted_values.clone(),
            predicted_stds,
            ehvi_score,
        });

        // Constant Liar: 次の候補のために作業データへ追加して各目的を再フィット。
        if i + 1 < n_candidates {
            work_x.push(params);
            for (k, yk) in work_ys.iter_mut().enumerate() {
                yk.push(predicted_values[k]);
            }
            let mut new_surrogates = Vec::with_capacity(n_obj);
            let mut refit_ok = true;
            for (k, yk) in work_ys.iter().enumerate() {
                match fit_surrogate(model_kinds[k], &work_x, yk) {
                    Ok(s) => new_surrogates.push(s),
                    Err(_) => {
                        refit_ok = false;
                        break;
                    }
                }
            }
            if refit_ok {
                refitted.push(new_surrogates);
            } else {
                // 再フィット失敗 → これまでの候補を Ok で返す（≥ 1 件）。
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

    /// 2 目的の競合問題のデータを生成する。
    /// f1 = x0² + x1²（最小化）、f2 = (x0−1)² + (x1−1)²（最小化）。
    fn conflicting_samples(n: usize) -> (Vec<Vec<f64>>, Vec<f64>, Vec<f64>) {
        let mut rng = SeededRng::from_seed(42);
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|_| vec![rng.next_f64(), rng.next_f64()])
            .collect();
        let f1: Vec<f64> = x_matrix
            .iter()
            .map(|r| r[0].powi(2) + r[1].powi(2))
            .collect();
        let f2: Vec<f64> = x_matrix
            .iter()
            .map(|r| (r[0] - 1.0).powi(2) + (r[1] - 1.0).powi(2))
            .collect();
        (x_matrix, f1, f2)
    }

    /// 2 目的の GP-FITC サロゲートを学習する。
    fn fit_two_objectives(
        x_matrix: Vec<Vec<f64>>,
        f1: Vec<f64>,
        f2: Vec<f64>,
    ) -> Vec<TrainedSurrogate> {
        let names = vec!["x0".to_string(), "x1".to_string()];
        let t1 = fit_surrogate_with_validation(&SurrogateFitRequest {
            x_matrix: x_matrix.clone(),
            y: f1,
            param_names: names.clone(),
            objective_name: "f1".to_string(),
            model: SurrogateModelKind::GpFitc,
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
        })
        .expect("fit f1 should succeed");
        let t2 = fit_surrogate_with_validation(&SurrogateFitRequest {
            x_matrix,
            y: f2,
            param_names: names,
            objective_name: "f2".to_string(),
            model: SurrogateModelKind::GpFitc,
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
        })
        .expect("fit f2 should succeed");
        vec![t1, t2]
    }

    #[test]
    fn single_candidate_in_box_with_stds() {
        let (x, f1, f2) = conflicting_samples(60);
        let trained = fit_two_objectives(x, f1, f2);
        let candidates = suggest_candidates_multi(&trained, &[true, true], 1)
            .expect("EHVI suggest should succeed");

        assert_eq!(candidates.len(), 1);
        let c = &candidates[0];
        assert!(
            c.params.iter().all(|&v| (0.0..=1.0).contains(&v)),
            "params out of [0,1]: {:?}",
            c.params
        );
        assert!(
            c.ehvi_score >= 0.0,
            "EHVI should be ≥ 0, got {}",
            c.ehvi_score
        );
        assert_eq!(c.predicted_values.len(), 2);
        assert_eq!(c.predicted_stds.len(), 2);
        assert!(
            c.predicted_stds.iter().all(|s| s.is_some()),
            "all GP stds should be Some"
        );
    }

    #[test]
    fn deterministic_across_two_runs() {
        let (x, f1, f2) = conflicting_samples(60);
        let trained = fit_two_objectives(x, f1, f2);
        let c1 = suggest_candidates_multi(&trained, &[true, true], 1).expect("run 1");
        let c2 = suggest_candidates_multi(&trained, &[true, true], 1).expect("run 2");
        assert_eq!(c1.len(), c2.len());
        for (a, b) in c1.iter().zip(c2.iter()) {
            for (pa, pb) in a.params.iter().zip(b.params.iter()) {
                assert!(
                    (pa - pb).abs() < 1e-9,
                    "EHVI suggest must be deterministic: {pa} vs {pb}"
                );
            }
            assert!((a.ehvi_score - b.ehvi_score).abs() < 1e-9);
        }
    }

    #[test]
    fn batch_3_candidates_pairwise_diverse() {
        let (x, f1, f2) = conflicting_samples(60);
        let trained = fit_two_objectives(x, f1, f2);
        let candidates = suggest_candidates_multi(&trained, &[true, true], 3)
            .expect("batch EHVI suggest should succeed");
        assert_eq!(candidates.len(), 3);

        let ref_s = &trained[0].surrogate;
        for i in 0..3 {
            for j in (i + 1)..3 {
                let ni = ref_s.to_norm_x(&candidates[i].params);
                let nj = ref_s.to_norm_x(&candidates[j].params);
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
    fn suggested_beats_worst_observed_point() {
        // 提案候補の EHVI が、最悪観測点（z-score 最小化フレームで nadir に近い点）の
        // EHVI 以上であること（オプティマイザが改善していることのサニティチェック）。
        let (x, f1, f2) = conflicting_samples(60);
        let trained = fit_two_objectives(x, f1, f2);
        let candidates = suggest_candidates_multi(&trained, &[true, true], 1).expect("suggest");
        let suggested = &candidates[0];

        // 同じコンテキストを再構築して最悪観測点の EHVI を測る。
        let surrogates: Vec<&FittedSurrogate> = trained.iter().map(|t| &t.surrogate).collect();
        let signs = vec![1.0, 1.0];
        let ys_refs: Vec<&[f64]> = trained.iter().map(|t| t.y.as_slice()).collect();
        let front = build_observed_front(&surrogates, &ys_refs, &signs);
        let ref_point = compute_ref_point(&front, 2);
        let hv_p = hypervolume_nd(&front, &ref_point);
        let z_matrix = draw_standard_normal_matrix(EHVI_SAMPLES, 2);
        let ctx = EhviContext {
            surrogates: surrogates.clone(),
            signs,
            front,
            ref_point,
            hv_p,
            z_matrix: &z_matrix,
        };

        // 最悪観測点: 第 1 目的が最大（最小化フレームで最悪）の trial の正規化パラメータ。
        let worst_idx = (0..trained[0].y.len())
            .max_by(|&a, &b| {
                trained[0].y[a]
                    .partial_cmp(&trained[0].y[b])
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        let worst_norm = trained[0]
            .surrogate
            .to_norm_x(&trained[0].x_matrix[worst_idx]);
        let worst_ehvi = ctx.ehvi(&worst_norm);

        assert!(
            suggested.ehvi_score >= worst_ehvi - 1e-9,
            "suggested EHVI {} should be >= worst-point EHVI {}",
            suggested.ehvi_score,
            worst_ehvi
        );
    }

    #[test]
    fn ridge_models_return_error() {
        let (x, f1, f2) = conflicting_samples(40);
        let names = vec!["x0".to_string(), "x1".to_string()];
        let make = |y: Vec<f64>, name: &str| {
            fit_surrogate_with_validation(&SurrogateFitRequest {
                x_matrix: x.clone(),
                y,
                param_names: names.clone(),
                objective_name: name.to_string(),
                model: SurrogateModelKind::Ridge,
                auto_select: false,
                constraints: vec![],
                priority_rows: vec![],
            })
            .expect("ridge fit should succeed")
        };
        let trained = vec![make(f1, "f1"), make(f2, "f2")];
        let err = suggest_candidates_multi(&trained, &[true, true], 1).unwrap_err();
        assert!(
            err.contains("Gaussian Process"),
            "expected GP error, got: {err}"
        );
    }

    #[test]
    fn mixed_min_max_valid() {
        // f2 を最大化する混合方向でも候補が有効（EHVI 有限、箱内）。
        let (x, f1, f2) = conflicting_samples(60);
        let trained = fit_two_objectives(x, f1, f2);
        let candidates = suggest_candidates_multi(&trained, &[true, false], 2)
            .expect("mixed min/max suggest should succeed");
        assert_eq!(candidates.len(), 2);
        for c in &candidates {
            assert!(c.ehvi_score.is_finite(), "EHVI should be finite");
            assert!(c.ehvi_score >= 0.0);
            assert!(
                c.params.iter().all(|&v| (0.0..=1.0).contains(&v)),
                "params out of [0,1]: {:?}",
                c.params
            );
        }
    }

    #[test]
    fn errors_on_empty_and_mismatch() {
        assert!(suggest_candidates_multi(&[], &[], 1).is_err());

        let (x, f1, f2) = conflicting_samples(40);
        let trained = fit_two_objectives(x, f1, f2);
        // n_candidates == 0
        assert!(suggest_candidates_multi(&trained, &[true, true], 0).is_err());
        // length mismatch
        assert!(suggest_candidates_multi(&trained, &[true], 1).is_err());
    }
}
