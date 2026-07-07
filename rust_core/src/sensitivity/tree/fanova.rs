//! fANOVA (functional ANOVA) — Hutter et al. (2014) の厳密な分散分解。
//!
//! LightGBM は葉ノードの区間（box）をクリーンに取得する API を提供しないため、
//! この手法専用に純 Rust の CART 回帰木・ランダムフォレストを実装する。
//!
//! アルゴリズム概要:
//! 1. ブートストラップ標本（n-of-n 復元抽出）ごとに CART 回帰木を学習する。
//! 2. 各木の葉ノードは軸並行な区間（box）と葉内 y の平均値を持つ。
//! 3. 訓練データ全体の観測範囲を一様事前分布とみなし、葉の box をこの範囲で
//!    正規化した体積を重み w_ℓ として、目的関数の分散を葉ごとの寄与に分解する。
//! 4. 次元 j ごとに、他の全次元を周辺化した「主効果」の分散 V_j を区分求積で
//!    厳密に計算し、V_j / V を木ごとの重要度とする。
//! 5. フォレスト全体の重要度は木ごとの V_j/V の平均（正の全分散を持つ木のみ）。
//!
//! 分割候補は全特徴量を対象とする（特徴量サブサンプリングを行わない）。fANOVA の
//! 主効果は木の軸並行分割がパラメータ空間全体を厳密に被覆していることに依存するため、
//! Optuna の fanova（max_features 既定あり）と異なりこの実装は全特徴量を使うことで
//! 主効果分解が近似ではなく厳密になる。

use super::common::normalize;
use crate::math::rng::SeededRng;
use crate::math::stats::value_range;
use rayon::prelude::*;

/// 葉ノード: 各次元の軸並行区間 `[lo, hi]` と葉内 y の平均値。
#[derive(Debug, Clone)]
pub(super) struct FanovaLeaf {
    pub(super) bounds: Vec<(f64, f64)>,
    pub(super) mean: f64,
}

/// CART 回帰木 1 本。fANOVA 分解には葉ノードの box と平均値のみ必要なため、
/// 内部の分割ノードは保持せず葉のみを保持する。
#[derive(Debug, Clone)]
pub(super) struct FanovaTree {
    pub(super) leaves: Vec<FanovaLeaf>,
}

/// フォレスト学習パラメータ。
pub(super) struct FanovaConfig {
    pub(super) n_trees: usize,
    pub(super) max_depth: usize,
    pub(super) min_samples_leaf: usize,
    pub(super) seed: u64,
}

/// 木構築時にノード間で共有される読み取り専用コンテキスト。
struct BuildCtx<'a> {
    x: &'a [Vec<f64>],
    y: &'a [f64],
    max_depth: usize,
    min_samples_leaf: usize,
    p: usize,
}

/// 訓練データ全体（ブートストラップ前）の次元ごとの観測範囲 `[min, max]`。
/// 各木の根ノードの box はこの範囲で初期化される。
fn observed_ranges(x: &[Vec<f64>], p: usize) -> Vec<(f64, f64)> {
    (0..p)
        .map(|d| value_range(x.iter().map(|row| row[d])))
        .collect()
}

/// ノード内サンプルを分割する最良の `(feature, threshold)` を探す。
/// 候補分割点は各特徴量についてノード内の相異なるソート済み値の中点。
/// SSE を改善する分割が存在しない場合（定数 y・定数 x・サンプル不足）は `None`。
fn best_split(
    indices: &[usize],
    x: &[Vec<f64>],
    y: &[f64],
    p: usize,
    min_samples_leaf: usize,
) -> Option<(usize, f64)> {
    let n = indices.len();
    if n < 2 * min_samples_leaf {
        return None;
    }
    let total_sum: f64 = indices.iter().map(|&i| y[i]).sum();
    let total_sum2: f64 = indices.iter().map(|&i| y[i] * y[i]).sum();
    let total_sse = total_sum2 - total_sum * total_sum / n as f64;
    if total_sse < 1e-12 {
        return None;
    }

    let mut best: Option<(usize, f64, f64)> = None;
    // p 個の特徴量それぞれについてノード内サンプルの (x[i][d], y[i]) 列を作るため、
    // x を直接 enumerate するイテレータには書き換えられない。
    #[allow(clippy::needless_range_loop)]
    for d in 0..p {
        let mut pairs: Vec<(f64, f64)> = indices.iter().map(|&i| (x[i][d], y[i])).collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if pairs[0].0 == pairs[n - 1].0 {
            continue; // 定数特徴量: 分割候補なし
        }

        let mut left_sum = 0.0;
        let mut left_sum2 = 0.0;
        let mut left_n = 0usize;
        let mut idx = 0usize;
        while idx < n {
            let cur_x = pairs[idx].0;
            while idx < n && pairs[idx].0 == cur_x {
                left_sum += pairs[idx].1;
                left_sum2 += pairs[idx].1 * pairs[idx].1;
                left_n += 1;
                idx += 1;
            }
            if idx == n {
                break; // 最後の値グループ: これ以降に右側を作れない
            }
            let right_n = n - left_n;
            if left_n < min_samples_leaf || right_n < min_samples_leaf {
                continue;
            }
            let right_sum = total_sum - left_sum;
            let right_sum2 = total_sum2 - left_sum2;
            let sse_left = (left_sum2 - left_sum * left_sum / left_n as f64).max(0.0);
            let sse_right = (right_sum2 - right_sum * right_sum / right_n as f64).max(0.0);
            let sse = sse_left + sse_right;
            let threshold = (cur_x + pairs[idx].0) / 2.0;
            // cur_x と次の値が隣接する f64 の場合、中点が cur_x に丸まり
            // `x < threshold` の分割が上の left_n/right_n カウントとずれる。
            // threshold > cur_x を保証できない候補はスキップする。
            if threshold <= cur_x {
                continue;
            }
            if best.as_ref().is_none_or(|b| sse < b.2) {
                best = Some((d, threshold, sse));
            }
        }
    }

    best.and_then(|(feat, thr, sse)| (sse < total_sse - 1e-12).then_some((feat, thr)))
}

/// 再帰的にノードを分割し、葉に到達したら `leaves` に追加する。
fn build_node(
    ctx: &BuildCtx,
    indices: Vec<usize>,
    bounds: Vec<(f64, f64)>,
    depth: usize,
    leaves: &mut Vec<FanovaLeaf>,
) {
    let n = indices.len();
    let mean = indices.iter().map(|&i| ctx.y[i]).sum::<f64>() / n as f64;

    if depth >= ctx.max_depth {
        leaves.push(FanovaLeaf { bounds, mean });
        return;
    }

    match best_split(&indices, ctx.x, ctx.y, ctx.p, ctx.min_samples_leaf) {
        Some((feat, threshold)) => {
            let (left, right): (Vec<usize>, Vec<usize>) = indices
                .into_iter()
                .partition(|&i| ctx.x[i][feat] < threshold);
            // best_split は両側が min_samples_leaf を満たす分割のみ返すため空にはならない
            debug_assert!(!left.is_empty() && !right.is_empty());

            let mut left_bounds = bounds.clone();
            left_bounds[feat].1 = threshold;
            let mut right_bounds = bounds;
            right_bounds[feat].0 = threshold;

            build_node(ctx, left, left_bounds, depth + 1, leaves);
            build_node(ctx, right, right_bounds, depth + 1, leaves);
        }
        None => leaves.push(FanovaLeaf { bounds, mean }),
    }
}

/// ブートストラップ標本から木を 1 本学習する。乱数シードは `config.seed + tree_index` で
/// 木ごとに決定的に変える（既存の `SeededRng`（ChaCha8）を再利用し、新規依存は追加しない）。
fn train_tree(
    x: &[Vec<f64>],
    y: &[f64],
    p: usize,
    config: &FanovaConfig,
    tree_index: usize,
    ranges: &[(f64, f64)],
) -> FanovaTree {
    let n = x.len();
    let mut rng = SeededRng::from_seed(config.seed.wrapping_add(tree_index as u64));
    let boot_indices: Vec<usize> = (0..n).map(|_| rng.next_usize(n)).collect();

    let ctx = BuildCtx {
        x,
        y,
        max_depth: config.max_depth,
        min_samples_leaf: config.min_samples_leaf,
        p,
    };
    let mut leaves = Vec::new();
    build_node(&ctx, boot_indices, ranges.to_vec(), 0, &mut leaves);
    FanovaTree { leaves }
}

/// CART 回帰フォレストを学習する。戻り値は `(木の配列, 訓練データ全体の次元ごとの観測範囲)`。
/// 木の学習は互いに独立なので rayon で並列化する。
pub(super) fn train_forest(
    x: &[Vec<f64>],
    y: &[f64],
    config: &FanovaConfig,
) -> (Vec<FanovaTree>, Vec<(f64, f64)>) {
    let p = x.first().map_or(0, |row| row.len());
    if p == 0 {
        return (vec![], vec![]);
    }
    let ranges = observed_ranges(x, p);
    let trees: Vec<FanovaTree> = (0..config.n_trees)
        .into_par_iter()
        .map(|t| train_tree(x, y, p, config, t, &ranges))
        .collect();
    (trees, ranges)
}

/// 葉の box を訓練データ範囲で正規化した体積の重み。
/// `exclude` に次元を指定すると、その次元を除いた周辺重み（fANOVA の主効果計算用）になる。
/// 範囲が退化している（長さ ≈ 0）次元は比率 1 として扱う。
fn leaf_weight(
    bounds: &[(f64, f64)],
    ranges: &[(f64, f64)],
    p: usize,
    exclude: Option<usize>,
) -> f64 {
    let mut w = 1.0;
    for d in 0..p {
        if Some(d) == exclude {
            continue;
        }
        let (range_lo, range_hi) = ranges[d];
        let range_len = range_hi - range_lo;
        if range_len < 1e-12 {
            continue;
        }
        let (lo, hi) = bounds[d];
        let inter_len = (hi.min(range_hi) - lo.max(range_lo)).max(0.0);
        w *= inter_len / range_len;
    }
    w
}

/// 1 本の木に対する fANOVA 分散分解の結果。
struct TreeDecomposition {
    // 非テストビルドでは検証用途がなく未読になるため許可する（t1 の手計算検証で使用）。
    #[allow(dead_code)]
    f0: f64,
    total_variance: f64,
    /// V_j (次元ごとの主効果分散)
    dim_variance: Vec<f64>,
}

/// 1 本の木を fANOVA 分解する。全分散 V が実質ゼロ（ほぼ定数出力）の木は `None` を返し、
/// 呼び出し元はこれをスキップする。
fn decompose_tree(tree: &FanovaTree, ranges: &[(f64, f64)], p: usize) -> Option<TreeDecomposition> {
    if tree.leaves.is_empty() {
        return None;
    }
    let weights: Vec<f64> = tree
        .leaves
        .iter()
        .map(|l| leaf_weight(&l.bounds, ranges, p, None))
        .collect();

    let f0: f64 = weights
        .iter()
        .zip(&tree.leaves)
        .map(|(&w, l)| w * l.mean)
        .sum();
    let ey2: f64 = weights
        .iter()
        .zip(&tree.leaves)
        .map(|(&w, l)| w * l.mean * l.mean)
        .sum();
    let total_variance = ey2 - f0 * f0;
    if total_variance < 1e-12 {
        return None;
    }

    let mut dim_variance = vec![0.0; p];
    for (d, &(range_lo, range_hi)) in ranges.iter().enumerate() {
        let range_len = range_hi - range_lo;
        if range_len < 1e-12 {
            continue;
        }

        // 次元 d を除いた周辺重み × 葉平均は区間に依存しないため、(leaf, d) ごとに
        // 1 度だけ事前計算する（以前は区間ループ内で毎回再計算しており O(葉数×区間数)）。
        let marginal_terms: Vec<f64> = tree
            .leaves
            .iter()
            .map(|leaf| leaf_weight(&leaf.bounds, ranges, p, Some(d)) * leaf.mean)
            .collect();

        // 次元 d について全葉の区間端点を集め、相異なる端点で区切られた区分求積用の
        // 「基本区間」を作る。基本区間内では、それを含む葉の集合が変化しない。
        let mut endpoints: Vec<f64> = Vec::with_capacity(tree.leaves.len() * 2 + 2);
        endpoints.push(range_lo);
        endpoints.push(range_hi);
        for leaf in &tree.leaves {
            let (lo, hi) = leaf.bounds[d];
            endpoints.push(lo.clamp(range_lo, range_hi));
            endpoints.push(hi.clamp(range_lo, range_hi));
        }
        endpoints.sort_by(|a, b| a.partial_cmp(b).unwrap());
        endpoints.dedup_by(|a, b| (*a - *b).abs() < 1e-12);

        for w in endpoints.windows(2) {
            let (i_lo, i_hi) = (w[0], w[1]);
            let i_len = i_hi - i_lo;
            if i_len < 1e-12 {
                continue;
            }
            // 基本区間の中点は端点として使われないため、どの葉の境界とも一致しない。
            let mid = (i_lo + i_hi) / 2.0;

            let f_j: f64 = tree
                .leaves
                .iter()
                .zip(&marginal_terms)
                .filter(|(leaf, _)| {
                    let (lo, hi) = leaf.bounds[d];
                    mid >= lo && mid <= hi
                })
                .map(|(_, &term)| term)
                .sum();

            let diff = f_j - f0;
            dim_variance[d] += (i_len / range_len) * diff * diff;
        }
    }

    Some(TreeDecomposition {
        f0,
        total_variance,
        dim_variance,
    })
}

/// フォレスト全体の主効果重要度。木ごとの `V_j / V` を（全分散が正の木についてのみ）平均し、
/// 合計 1 になるよう正規化する。有効な木が 1 本もない場合はすべて 0.0。
pub(super) fn forest_importances(
    trees: &[FanovaTree],
    ranges: &[(f64, f64)],
    p: usize,
) -> Vec<f64> {
    let decomps: Vec<Option<TreeDecomposition>> = trees
        .par_iter()
        .map(|t| decompose_tree(t, ranges, p))
        .collect();

    let mut sum = vec![0.0; p];
    let mut count = 0usize;
    for d in decomps.into_iter().flatten() {
        for (s, dv) in sum.iter_mut().zip(d.dim_variance.iter()) {
            *s += dv / d.total_variance;
        }
        count += 1;
    }
    if count == 0 {
        return sum;
    }
    for v in sum.iter_mut() {
        *v /= count as f64;
    }
    normalize(&mut sum);
    sum
}

/// 1 本の木で予測値（葉の平均値）を求める。範囲外の入力は訓練データ範囲にクランプしてから
/// box に一致する葉を探す（浮動小数誤差対策で境界に微小な許容誤差を持たせる）。
fn predict_tree(tree: &FanovaTree, row: &[f64], ranges: &[(f64, f64)]) -> f64 {
    let clamped: Vec<f64> = row
        .iter()
        .zip(ranges)
        .map(|(&v, &(lo, hi))| v.clamp(lo, hi))
        .collect();

    for leaf in &tree.leaves {
        let contains = clamped
            .iter()
            .zip(leaf.bounds.iter())
            .all(|(&v, &(lo, hi))| v >= lo - 1e-9 && v <= hi + 1e-9);
        if contains {
            return leaf.mean;
        }
    }
    // 到達しないはずだが、浮動小数誤差で一致しない場合のフォールバック
    tree.leaves.first().map_or(0.0, |l| l.mean)
}

/// フォレストの予測値（木ごとの葉平均値の平均）。
fn predict_forest(trees: &[FanovaTree], row: &[f64], ranges: &[(f64, f64)]) -> f64 {
    if trees.is_empty() {
        return 0.0;
    }
    let sum: f64 = trees.iter().map(|t| predict_tree(t, row, ranges)).sum();
    sum / trees.len() as f64
}

/// 前処理済み訓練/評価データから fANOVA フォレストを学習し `(重要度, R²)` を返す。
/// R² はフォレスト予測（木ごとの葉平均値の平均）でホールドアウト評価データを予測して計算する。
pub(super) fn compute_fanova(
    x_train: &[Vec<f64>],
    y_train: &[f64],
    x_eval: &[Vec<f64>],
    y_eval: &[f64],
    config: &FanovaConfig,
) -> Option<(Vec<f64>, f64)> {
    let p = x_train.first()?.len();
    if p == 0 {
        return None;
    }
    let (trees, ranges) = train_forest(x_train, y_train, config);
    if trees.is_empty() {
        return None;
    }

    let importances = forest_importances(&trees, &ranges, p);

    let mse = if y_eval.is_empty() {
        0.0
    } else {
        x_eval
            .iter()
            .zip(y_eval)
            .map(|(row, &y)| (predict_forest(&trees, row, &ranges) - y).powi(2))
            .sum::<f64>()
            / y_eval.len() as f64
    };
    let r_squared = crate::lgbm::mse_to_r_squared(mse, y_eval).max(0.0);

    Some((importances, r_squared))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(b0: (f64, f64), b1: (f64, f64), mean: f64) -> FanovaLeaf {
        FanovaLeaf {
            bounds: vec![b0, b1],
            mean,
        }
    }

    /// t1: 手計算で検証済みの木構造 (theory/ja/sensitivity-analysis/rfanova.md 参照)。
    /// ルート x0<0.5 → 左葉 y=0。右側 (x0>=0.5) はさらに x1<0.25 で y=1 / y=3 に分割。
    #[test]
    fn analytic_two_split_tree_matches_hand_computation() {
        let tree = FanovaTree {
            leaves: vec![
                leaf((0.0, 0.5), (0.0, 1.0), 0.0),
                leaf((0.5, 1.0), (0.0, 0.25), 1.0),
                leaf((0.5, 1.0), (0.25, 1.0), 3.0),
            ],
        };
        let ranges = vec![(0.0, 1.0), (0.0, 1.0)];

        // 葉の重み: w = ([0,0.5]x[0,1], 0.5), ([0.5,1]x[0,0.25], 0.125), ([0.5,1]x[0.25,1], 0.375)
        let weights: Vec<f64> = tree
            .leaves
            .iter()
            .map(|l| leaf_weight(&l.bounds, &ranges, 2, None))
            .collect();
        assert!((weights[0] - 0.5).abs() < 1e-12);
        assert!((weights[1] - 0.125).abs() < 1e-12);
        assert!((weights[2] - 0.375).abs() < 1e-12);

        let decomp = decompose_tree(&tree, &ranges, 2).expect("全分散は正のはず");

        assert!((decomp.f0 - 1.25).abs() < 1e-12, "f0={}", decomp.f0);
        assert!(
            (decomp.total_variance - 1.9375).abs() < 1e-12,
            "V={}",
            decomp.total_variance
        );
        assert!(
            (decomp.dim_variance[0] - 1.5625).abs() < 1e-12,
            "V0={}",
            decomp.dim_variance[0]
        );
        assert!(
            (decomp.dim_variance[1] - 0.1875).abs() < 1e-12,
            "V1={}",
            decomp.dim_variance[1]
        );

        let frac0 = decomp.dim_variance[0] / decomp.total_variance;
        let frac1 = decomp.dim_variance[1] / decomp.total_variance;
        assert!(
            (frac0 - 0.806_451_612_903_225_8).abs() < 1e-9,
            "frac0={frac0}"
        );
        assert!(
            (frac1 - 0.096_774_193_548_387_1).abs() < 1e-9,
            "frac1={frac1}"
        );
    }

    /// 周辺重みの (leaf, d) キャッシュ化（区間ループ外への吊り上げ）が、実データから
    /// 学習したフォレストの分解結果を一切変えないことを、区間ループ内で毎回
    /// `leaf_weight` を再計算する素朴な実装と突き合わせて確認する。
    #[test]
    fn cached_marginal_weights_match_naive_recomputation() {
        // 素朴な実装（キャッシュなし）: decompose_tree の旧ロジックを踏襲。
        fn decompose_naive(tree: &FanovaTree, ranges: &[(f64, f64)], p: usize) -> Vec<f64> {
            let weights: Vec<f64> = tree
                .leaves
                .iter()
                .map(|l| leaf_weight(&l.bounds, ranges, p, None))
                .collect();
            let f0: f64 = weights
                .iter()
                .zip(&tree.leaves)
                .map(|(&w, l)| w * l.mean)
                .sum();
            let mut dim_variance = vec![0.0; p];
            for (d, &(range_lo, range_hi)) in ranges.iter().enumerate() {
                let range_len = range_hi - range_lo;
                if range_len < 1e-12 {
                    continue;
                }
                let mut endpoints: Vec<f64> = vec![range_lo, range_hi];
                for leaf in &tree.leaves {
                    let (lo, hi) = leaf.bounds[d];
                    endpoints.push(lo.clamp(range_lo, range_hi));
                    endpoints.push(hi.clamp(range_lo, range_hi));
                }
                endpoints.sort_by(|a, b| a.partial_cmp(b).unwrap());
                endpoints.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
                for w in endpoints.windows(2) {
                    let (i_lo, i_hi) = (w[0], w[1]);
                    let i_len = i_hi - i_lo;
                    if i_len < 1e-12 {
                        continue;
                    }
                    let mid = (i_lo + i_hi) / 2.0;
                    let f_j: f64 = tree
                        .leaves
                        .iter()
                        .filter(|leaf| {
                            let (lo, hi) = leaf.bounds[d];
                            mid >= lo && mid <= hi
                        })
                        .map(|leaf| leaf_weight(&leaf.bounds, ranges, p, Some(d)) * leaf.mean)
                        .sum();
                    let diff = f_j - f0;
                    dim_variance[d] += (i_len / range_len) * diff * diff;
                }
            }
            dim_variance
        }

        // 実データからフォレストを学習して両実装を突き合わせる。
        let mut rng = SeededRng::from_seed(12345);
        let x: Vec<Vec<f64>> = (0..50)
            .map(|_| {
                (0..3)
                    .map(|_| rng.next_usize(1000) as f64 / 1000.0)
                    .collect()
            })
            .collect();
        let y: Vec<f64> = x.iter().map(|r| 3.0 * r[0] + r[1] * r[1]).collect();
        let config = FanovaConfig {
            n_trees: 8,
            max_depth: 6,
            min_samples_leaf: 2,
            seed: 7,
        };
        let (trees, ranges) = train_forest(&x, &y, &config);
        assert!(!trees.is_empty());

        for tree in &trees {
            let Some(decomp) = decompose_tree(tree, &ranges, 3) else {
                continue;
            };
            let naive = decompose_naive(tree, &ranges, 3);
            for (a, b) in decomp.dim_variance.iter().zip(&naive) {
                assert_eq!(a, b, "cached vs naive dim_variance must be bit-identical");
            }
        }
    }
}
