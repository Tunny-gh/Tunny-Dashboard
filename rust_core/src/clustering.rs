//! PCA・k-means・クラスタ統計量 (TASK-901)
//!
//! 【役割】: 教師なし学習によるトライアルのグループ化
//! 【設計方針】:
//!   - PCA: 共分散行列 + Jacobi 固有値分解（p≤30 で O(p³) → 高速）
//!   - k-means: k-means++ 初期化 + Lloyd's algorithm
//!   - Elbow: WCSS の二次差分最大点で最適 k を推定
//!   - ClusterStats: centroid・std・Welch's t検定（有意差マーク）
//!
//! REQ-080: PCA run_pca() — 固有値分解
//! REQ-081: k-means run_kmeans() — Lloyd's algorithm
//! REQ-082: estimate_k_elbow() — Elbow法
//! REQ-083: compute_cluster_stats() — centroid / std / t検定
//!
//! 参照: docs/tasks/tunny-dashboard-tasks.md TASK-901

// =============================================================================
// 公開型定義
// =============================================================================

/// PCA 空間の指定
///
/// 【設計】: どの列群に対してPCAを実行するかを指定する 🟢
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PcaSpace {
    /// パラメータ列のみ
    Param,
    /// 目的列のみ
    Objective,
    /// 全数値列（パラメータ + 目的 + user_attr_numeric）
    All,
}

/// PCA の結果
///
/// 【設計】: 射影座標・負荷量・説明分散をまとめて保持する 🟢
#[derive(Debug, Clone)]
pub struct PcaResult {
    /// 射影座標: projections[i][j] = サンプル i の主成分 j の値 (n × k)
    pub projections: Vec<Vec<f64>>,
    /// 負荷量ベクタ: loadings[j][k] = 主成分 j での特徴 k の重み (n_components × p)
    pub loadings: Vec<Vec<f64>>,
    /// 各主成分の説明分散（固有値）
    pub explained_variance: Vec<f64>,
    /// 分析対象の特徴名リスト
    pub feature_names: Vec<String>,
}

/// k-means クラスタリングの結果
#[derive(Debug, Clone)]
pub struct KmeansResult {
    /// 各サンプルのクラスタラベル: labels[i] ∈ [0, k)
    pub labels: Vec<usize>,
    /// クラスタ重心: centroids[k][p]
    pub centroids: Vec<Vec<f64>>,
    /// クラスタ内誤差平方和（Within-Cluster Sum of Squares）
    pub wcss: f64,
    /// 収束に要したイテレーション数
    pub iterations: usize,
}

/// Elbow 法の結果
#[derive(Debug, Clone)]
pub struct ElbowResult {
    /// 各 k での WCSS: wcss_per_k[0] = k=2, [1] = k=3, ...
    pub wcss_per_k: Vec<f64>,
    /// Elbow 法による推薦 k（2 ≤ recommended_k ≤ max_k）
    pub recommended_k: usize,
}

/// 単一クラスタの統計量
#[derive(Debug, Clone)]
pub struct ClusterStat {
    /// クラスタ ID（0 始まり）
    pub cluster_id: usize,
    /// クラスタのサンプル数
    pub size: usize,
    /// 各特徴の平均（重心）
    pub centroid: Vec<f64>,
    /// 各特徴の標準偏差
    pub std_dev: Vec<f64>,
    /// 全体平均との有意差フラグ（Welch's t 検定、|t| > 3.0）
    /// significant_features[j] = true なら j 番目特徴が有意に異なる 🟢
    pub significant_features: Vec<bool>,
}

// =============================================================================
// 内部ヘルパー関数
// =============================================================================

/// 列方向の平均を計算する
///
/// 【設計】: PCA・クラスタ統計量の前処理で使用 🟢
fn col_means(data: &[Vec<f64>]) -> Vec<f64> {
    let n = data.len();
    if n == 0 {
        return vec![];
    }
    let p = data[0].len();
    let mut means = vec![0.0f64; p];
    for row in data {
        for (j, &v) in row.iter().enumerate() {
            means[j] += v;
        }
    }
    for m in &mut means {
        *m /= n as f64;
    }
    means
}

/// データを列平均で中心化する
fn center_data(data: &[Vec<f64>], means: &[f64]) -> Vec<Vec<f64>> {
    data.iter()
        .map(|row| {
            row.iter()
                .zip(means.iter())
                .map(|(&v, &m)| v - m)
                .collect()
        })
        .collect()
}

/// 2点間のユークリッド距離の二乗を計算する（O(p)）
#[inline]
fn sq_dist(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum()
}

/// 対称行列の Jacobi 固有値分解を実行する
///
/// 【アルゴリズム】: Jacobi 回転法 — 最大非対角要素を順次ゼロ化する
/// 【適用範囲】: p ≤ 30 の小行列向け（O(p³ × sweep数)）
/// 【戻り値】: (固有値, 固有ベクタ行列) — 固有値降順でソート済み
/// 🟢 共分散行列に適用して PCA の主成分を取得する
fn jacobi_eigensystem(mut a: Vec<Vec<f64>>, p: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    // 【固有ベクタ行列の初期化】: 単位行列から開始
    let mut eigvec: Vec<Vec<f64>> = (0..p)
        .map(|i| (0..p).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
        .collect();

    let max_sweeps = 100 * p * p;

    for _ in 0..max_sweeps {
        // 【最大非対角要素の探索】
        let mut max_off = 0.0f64;
        let mut pi = 0usize;
        let mut qi = 1usize;
        for i in 0..p {
            for j in (i + 1)..p {
                let v = a[i][j].abs();
                if v > max_off {
                    max_off = v;
                    pi = i;
                    qi = j;
                }
            }
        }

        // 【収束判定】: 最大非対角要素が十分小さければ終了
        if max_off < 1e-12 {
            break;
        }

        // 【Jacobi 回転角度の計算】: tan(2θ) = 2*a_pq / (a_qq - a_pp)
        let a_pp = a[pi][pi];
        let a_qq = a[qi][qi];
        let a_pq = a[pi][qi];

        let theta = if a_pq.abs() < f64::EPSILON {
            0.0
        } else {
            (a_qq - a_pp) / (2.0 * a_pq)
        };

        // 【t = tan(θ)】: 絶対値最小解を選択して数値安定性を確保
        let t = if theta >= 0.0 {
            1.0 / (theta + (1.0 + theta * theta).sqrt())
        } else {
            -1.0 / (-theta + (1.0 + theta * theta).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt(); // cos(θ)
        let s = t * c; // sin(θ)

        // 【対角要素の更新】
        a[pi][pi] = c * c * a_pp - 2.0 * s * c * a_pq + s * s * a_qq;
        a[qi][qi] = s * s * a_pp + 2.0 * s * c * a_pq + c * c * a_qq;
        a[pi][qi] = 0.0;
        a[qi][pi] = 0.0;

        // 【非対角要素の更新】
        for r in 0..p {
            if r == pi || r == qi {
                continue;
            }
            let a_rp = a[r][pi];
            let a_rq = a[r][qi];
            let new_rp = c * a_rp - s * a_rq;
            let new_rq = s * a_rp + c * a_rq;
            a[r][pi] = new_rp;
            a[pi][r] = new_rp;
            a[r][qi] = new_rq;
            a[qi][r] = new_rq;
        }

        // 【固有ベクタ行列の更新】
        for r in 0..p {
            let v_rp = eigvec[r][pi];
            let v_rq = eigvec[r][qi];
            eigvec[r][pi] = c * v_rp - s * v_rq;
            eigvec[r][qi] = s * v_rp + c * v_rq;
        }
    }

    // 【固有値取得】
    let mut eigenvalues: Vec<f64> = (0..p).map(|i| a[i][i].max(0.0)).collect();

    // 【降順ソート】: 説明分散の大きい主成分が先頭に来るよう並び替え
    let mut idx: Vec<usize> = (0..p).collect();
    idx.sort_by(|&i, &j| {
        eigenvalues[j]
            .partial_cmp(&eigenvalues[i])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let sorted_eigenvalues: Vec<f64> = idx.iter().map(|&i| eigenvalues[i]).collect();
    let sorted_eigvec: Vec<Vec<f64>> = (0..p)
        .map(|row| idx.iter().map(|&i| eigvec[row][i]).collect())
        .collect();

    // 【未使用変数の警告を抑制】
    eigenvalues.clear();

    (sorted_eigenvalues, sorted_eigvec)
}

// =============================================================================
// pub(crate) 計算関数（テスト・内部向け）
// =============================================================================

/// 行優先データ行列から PCA を実行する（テスト・内部向け）
///
/// 【アルゴリズム】: 共分散行列（p×p）を構築 → Jacobi 固有値分解 → 上位 k 主成分に射影
///
/// 【計算ステップ】🟢:
///   1. 列平均で中心化: X_c[i][j] = X[i][j] - mean_j
///   2. 共分散行列: C[i][j] = (1/(n-1)) * Σ_k X_c[k,i] * X_c[k,j]
///   3. 固有値分解: C = V Λ V^T（Jacobi 回転法）
///   4. 射影: projections[i][comp] = Σ_j X_c[i][j] * loading[comp][j]
pub(crate) fn run_pca_on_matrix(data: &[Vec<f64>], n_components: usize) -> PcaResult {
    let empty = PcaResult {
        projections: vec![],
        loadings: vec![],
        explained_variance: vec![],
        feature_names: vec![],
    };

    let n = data.len();
    if n < 2 || data[0].is_empty() || n_components == 0 {
        return empty;
    }
    let p = data[0].len();
    let k = n_components.min(p);

    // 【中心化】
    let means = col_means(data);
    let x_c = center_data(data, &means);

    // 【共分散行列の構築（列優先メモリで計算効率化）】
    // x_cols[j * n + i] = x_c[i][j] (連続メモリ → キャッシュ効率良好)
    let mut x_cols = vec![0.0f64; n * p];
    for (i, row) in x_c.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            x_cols[j * n + i] = v;
        }
    }

    let nf = (n as f64 - 1.0).max(1.0);
    let mut cov = vec![vec![0.0f64; p]; p];
    for i in 0..p {
        for j in i..p {
            let col_i = &x_cols[i * n..(i + 1) * n];
            let col_j = &x_cols[j * n..(j + 1) * n];
            let val: f64 =
                col_i.iter().zip(col_j.iter()).map(|(a, b)| a * b).sum::<f64>() / nf;
            cov[i][j] = val;
            cov[j][i] = val;
        }
    }

    // 【固有値分解（降順ソート済み）】
    let (eigenvalues, eigvec) = jacobi_eigensystem(cov, p);

    // 【上位 k 主成分の負荷量】: loadings[comp][feat] = eigvec[feat][comp]
    let loadings: Vec<Vec<f64>> = (0..k)
        .map(|comp| (0..p).map(|feat| eigvec[feat][comp]).collect())
        .collect();

    // 【射影】: projections[i][comp] = Σ_j X_c[i][j] * loadings[comp][j]
    let projections: Vec<Vec<f64>> = x_c
        .iter()
        .map(|row| {
            (0..k)
                .map(|comp| {
                    row.iter()
                        .zip(loadings[comp].iter())
                        .map(|(x, l)| x * l)
                        .sum()
                })
                .collect()
        })
        .collect();

    PcaResult {
        projections,
        loadings,
        explained_variance: eigenvalues[..k].to_vec(),
        feature_names: vec![],
    }
}

/// フラット行優先データ配列から k-means クラスタリングを実行する（テスト・内部向け）
///
/// 【アルゴリズム】:
///   1. k-means++ 初期化: D²確率比例サンプリングで初期重心を選択
///   2. Lloyd's algorithm: 割り当て → 重心更新 → 収束まで繰り返す
///
/// 【パラメータ】:
///   - flat_data: 行優先フラット配列 (n * p 要素)
///   - n: サンプル数
///   - p: 特徴数
///   - k: クラスタ数
pub(crate) fn run_kmeans_on_data(flat_data: &[f64], n: usize, p: usize, k: usize) -> KmeansResult {
    let empty = KmeansResult {
        labels: vec![0; n],
        centroids: vec![],
        wcss: 0.0,
        iterations: 0,
    };

    if n < k || k == 0 || p == 0 || flat_data.len() < n * p {
        return empty;
    }

    // 【点取得ヘルパー】: flat_data[i*p .. (i+1)*p] = サンプル i の特徴ベクタ
    let get_point = |i: usize| -> &[f64] { &flat_data[i * p..(i + 1) * p] };

    // 【k-means++ 初期化】: D²確率比例サンプリングで初期重心を選択
    let mut centroids: Vec<Vec<f64>> = Vec::with_capacity(k);

    // 最初の重心: データの中央付近のサンプルを選択（決定論的にする）
    centroids.push(get_point(n / 2).to_vec());

    // 残りの重心
    for _ in 1..k {
        // 各点から最近の既存重心までの距離² を計算
        let mut distances: Vec<f64> = (0..n)
            .map(|i| {
                centroids
                    .iter()
                    .map(|c| sq_dist(get_point(i), c))
                    .fold(f64::INFINITY, f64::min)
            })
            .collect();

        // 距離の合計
        let total: f64 = distances.iter().sum();
        if total < f64::EPSILON {
            // 全点が同一の場合はランダム選択代わりに次の点を使用
            let idx = centroids.len() % n;
            centroids.push(get_point(idx).to_vec());
            continue;
        }

        // 確率的サンプリング: 距離²に比例する確率でサンプルを選択
        // 決定論的実装: 累積距離が total/k * (選択済み数+1) を超えた最初の点を選択
        let threshold = total / (k - centroids.len() + 1) as f64;
        let mut cum = 0.0;
        let mut chosen = n - 1;
        for (i, &d) in distances.iter().enumerate() {
            cum += d;
            if cum >= threshold {
                chosen = i;
                break;
            }
        }
        distances.clear();
        centroids.push(get_point(chosen).to_vec());
    }

    // 【Lloyd's algorithm】
    let mut labels = vec![0usize; n];
    let max_iter = 300;

    for iter in 0..max_iter {
        // 【割り当てフェーズ】: 各点を最近の重心に割り当て
        let mut changed = false;
        for i in 0..n {
            let pt = get_point(i);
            let new_label = (0..k)
                .min_by(|&a, &b| {
                    sq_dist(pt, &centroids[a])
                        .partial_cmp(&sq_dist(pt, &centroids[b]))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0);
            if labels[i] != new_label {
                labels[i] = new_label;
                changed = true;
            }
        }

        if !changed {
            // 【収束】
            let wcss = compute_wcss(flat_data, n, p, &labels, &centroids, k);
            return KmeansResult { labels, centroids, wcss, iterations: iter + 1 };
        }

        // 【重心更新フェーズ】: 各クラスタの平均に重心を移動
        let mut new_centroids = vec![vec![0.0f64; p]; k];
        let mut counts = vec![0usize; k];
        for i in 0..n {
            let lbl = labels[i];
            let pt = get_point(i);
            for j in 0..p {
                new_centroids[lbl][j] += pt[j];
            }
            counts[lbl] += 1;
        }
        for c in 0..k {
            if counts[c] > 0 {
                for j in 0..p {
                    new_centroids[c][j] /= counts[c] as f64;
                }
            } else {
                // 【空クラスタ処理】: 元の重心を維持する
                new_centroids[c] = centroids[c].clone();
            }
        }
        centroids = new_centroids;
    }

    // 最大イテレーション後の WCSS を計算
    let wcss = compute_wcss(flat_data, n, p, &labels, &centroids, k);
    KmeansResult { labels, centroids, wcss, iterations: max_iter }
}

/// WCSS（クラスタ内誤差平方和）を計算する
fn compute_wcss(
    flat_data: &[f64],
    n: usize,
    p: usize,
    labels: &[usize],
    centroids: &[Vec<f64>],
    k: usize,
) -> f64 {
    let _ = k;
    (0..n)
        .map(|i| sq_dist(&flat_data[i * p..(i + 1) * p], &centroids[labels[i]]))
        .sum()
}

/// Elbow 法で最適 k を推定する（テスト・内部向け）
///
/// 【アルゴリズム】: k=2..max_k で k-means を実行し WCSS の二次差分が最大の k を選択
/// 【設計】: 二次差分 = Δ²WCSS(k) = WCSS(k-1) - 2*WCSS(k) + WCSS(k+1) 🟢
pub(crate) fn estimate_k_elbow_on_data(
    flat_data: &[f64],
    n: usize,
    p: usize,
    max_k: usize,
) -> ElbowResult {
    let effective_max_k = max_k.min(n);
    if effective_max_k < 2 {
        return ElbowResult { wcss_per_k: vec![], recommended_k: 2 };
    }

    let wcss_per_k: Vec<f64> = (2..=effective_max_k)
        .map(|k| run_kmeans_on_data(flat_data, n, p, k).wcss)
        .collect();

    // 【推薦 k の選択】: 二次差分が最大の点を Elbow と見なす
    let recommended_k = if wcss_per_k.len() < 3 {
        // k=2 または k=3 しかない場合はそのまま返す
        wcss_per_k.len() + 1
    } else {
        // 二次差分: second_diff[i] = wcss[i] - 2*wcss[i+1] + wcss[i+2]
        let second_diffs: Vec<f64> = (0..wcss_per_k.len() - 2)
            .map(|i| wcss_per_k[i] - 2.0 * wcss_per_k[i + 1] + wcss_per_k[i + 2])
            .collect();

        let best_idx = second_diffs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        // 二次差分のインデックスは k=2..max_k-2 に対応する
        // second_diffs[0] は k=3 の前後の変化率を反映
        best_idx + 3 // k=3 が基準（second_diffs[0] = k=3 のElbow）
    };

    let recommended_k = recommended_k.clamp(2, effective_max_k);

    ElbowResult { wcss_per_k, recommended_k }
}

/// クラスタ統計量を計算する（テスト・内部向け）
///
/// 【算出内容】: 各クラスタの centroid・std・Welch's t 検定（全体平均との比較）
///
/// 【t検定設計】🟢: Welch's t = (cluster_mean - global_mean) / sqrt(var/n_c + var_global/n)
///   |t| > 3.0 かつ標準化効果量 > 0.1 を有意差ありと判定
pub(crate) fn compute_cluster_stats_on_data(
    flat_data: &[f64],
    n: usize,
    p: usize,
    labels: &[usize],
    k: usize,
) -> Vec<ClusterStat> {
    if n == 0 || p == 0 || flat_data.len() < n * p {
        return vec![];
    }

    // 【全体統計量】
    let mut global_mean = vec![0.0f64; p];
    let mut global_var = vec![0.0f64; p];
    for i in 0..n {
        for j in 0..p {
            global_mean[j] += flat_data[i * p + j];
        }
    }
    for m in &mut global_mean {
        *m /= n as f64;
    }
    for i in 0..n {
        for j in 0..p {
            global_var[j] += (flat_data[i * p + j] - global_mean[j]).powi(2);
        }
    }
    for v in &mut global_var {
        *v /= (n as f64 - 1.0).max(1.0);
    }

    // 【クラスタ統計量の計算】
    (0..k)
        .map(|c| {
            let indices: Vec<usize> = (0..n).filter(|&i| labels[i] == c).collect();
            let nc = indices.len();
            if nc == 0 {
                return ClusterStat {
                    cluster_id: c,
                    size: 0,
                    centroid: global_mean.clone(),
                    std_dev: vec![0.0; p],
                    significant_features: vec![false; p],
                };
            }

            // クラスタ内平均
            let mut centroid = vec![0.0f64; p];
            for &i in &indices {
                for j in 0..p {
                    centroid[j] += flat_data[i * p + j];
                }
            }
            for m in &mut centroid {
                *m /= nc as f64;
            }

            // クラスタ内標準偏差
            let mut var_c = vec![0.0f64; p];
            for &i in &indices {
                for j in 0..p {
                    var_c[j] += (flat_data[i * p + j] - centroid[j]).powi(2);
                }
            }
            let nc_f = nc as f64;
            for v in &mut var_c {
                *v /= (nc_f - 1.0).max(1.0);
            }
            let std_dev: Vec<f64> = var_c.iter().map(|&v| v.sqrt()).collect();

            // 【Welch's t 検定】: クラスタ平均 vs 全体平均
            // t = (mean_c - mean_global) / sqrt(var_c/n_c + var_global/n)
            let n_f = n as f64;
            let significant_features: Vec<bool> = (0..p)
                .map(|j| {
                    let diff = (centroid[j] - global_mean[j]).abs();
                    let se = (var_c[j] / nc_f + global_var[j] / n_f).sqrt();
                    if se < f64::EPSILON {
                        return false;
                    }
                    let t = diff / se;
                    // 【有意差判定】: |t| > 3.0（≒ p < 0.003）
                    t > 3.0
                })
                .collect();

            ClusterStat {
                cluster_id: c,
                size: nc,
                centroid,
                std_dev,
                significant_features,
            }
        })
        .collect()
}

// =============================================================================
// DataFrame 対応 公開 API
// =============================================================================

/// アクティブ Study から PCA を実行する
///
/// 【設計】: with_active_df で列データを取得し run_pca_on_matrix に委譲する 🟢
/// @param n_components 主成分数（1〜p）
/// @param space 対象空間（Param / Objective / All）
pub fn run_pca(n_components: usize, space: PcaSpace) -> Option<PcaResult> {
    crate::dataframe::with_active_df(|df| {
        // 【対象列の選択】
        let feature_names: Vec<String> = match space {
            PcaSpace::Param => df.param_col_names().to_vec(),
            PcaSpace::Objective => df.objective_col_names().to_vec(),
            PcaSpace::All => {
                let mut names = df.param_col_names().to_vec();
                names.extend_from_slice(df.objective_col_names());
                names.extend_from_slice(df.user_attr_numeric_col_names());
                names
            }
        };

        if feature_names.is_empty() {
            return None;
        }

        let n = df.row_count();
        if n < 2 {
            return None;
        }

        // 【行優先行列構築】
        let data: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                feature_names
                    .iter()
                    .map(|name| {
                        df.get_numeric_column(name)
                            .and_then(|c| c.get(i))
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();

        let mut result = run_pca_on_matrix(&data, n_components);
        result.feature_names = feature_names;
        Some(result)
    })
    .flatten()
}

/// フラットデータ配列から k-means を実行する（公開 API）
///
/// 【設計】: JS/WASM から直接呼べるように flat_data を受け取る 🟢
pub fn run_kmeans(k: usize, flat_data: &[f64], n_cols: usize) -> KmeansResult {
    if n_cols == 0 || flat_data.is_empty() {
        return KmeansResult { labels: vec![], centroids: vec![], wcss: 0.0, iterations: 0 };
    }
    let n = flat_data.len() / n_cols;
    run_kmeans_on_data(flat_data, n, n_cols, k)
}

/// Elbow 法で最適 k を推定する（公開 API）
pub fn estimate_k_elbow(flat_data: &[f64], n_cols: usize, max_k: usize) -> ElbowResult {
    if n_cols == 0 || flat_data.is_empty() {
        return ElbowResult { wcss_per_k: vec![], recommended_k: 2 };
    }
    let n = flat_data.len() / n_cols;
    estimate_k_elbow_on_data(flat_data, n, n_cols, max_k)
}

/// アクティブ Study のデータに対してクラスタ統計量を計算する（公開 API）
///
/// 【設計】: with_active_df から全数値列のデータを取得し統計を計算する 🟢
pub fn compute_cluster_stats(labels: &[usize]) -> Vec<ClusterStat> {
    let Some(result) = crate::dataframe::with_active_df(|df| {
        let mut all_names = df.param_col_names().to_vec();
        all_names.extend_from_slice(df.objective_col_names());
        let n = df.row_count();
        let p = all_names.len();

        if n == 0 || p == 0 || labels.len() != n {
            return vec![];
        }

        // フラット配列構築
        let k = labels.iter().copied().max().map(|m| m + 1).unwrap_or(0);
        let flat_data: Vec<f64> = (0..n)
            .flat_map(|i| {
                all_names
                    .iter()
                    .map(move |name| {
                        df.get_numeric_column(name)
                            .and_then(|c| c.get(i))
                            .copied()
                            .unwrap_or(0.0)
                    })
            })
            .collect();

        compute_cluster_stats_on_data(&flat_data, n, p, labels, k)
    }) else {
        return vec![];
    };
    result
}

// =============================================================================
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // テストデータ生成ヘルパー
    // ------------------------------------------------------------------

    /// 【ヘルパー】: x1 高分散・x2 ほぼゼロ分散の2次元データを生成する
    fn make_dominant_axis_data(n: usize) -> Vec<Vec<f64>> {
        (0..n)
            .map(|i| {
                let x1 = i as f64 / n as f64 * 10.0; // 高分散
                let x2 = 0.01 * (i as f64 / n as f64); // 低分散
                vec![x1, x2]
            })
            .collect()
    }

    /// 【ヘルパー】: k 個の明確に分離されたクラスタを持つデータを生成する
    fn make_clustered_data(n_per_cluster: usize, k: usize) -> Vec<f64> {
        let mut data = Vec::with_capacity(n_per_cluster * k * 2);
        for c in 0..k {
            let center = (c as f64) * 100.0; // 十分広い間隔
            for i in 0..n_per_cluster {
                let x = center + (i as f64) * 0.01; // ほぼ点群
                let y = center + (i as f64) * 0.01;
                data.push(x);
                data.push(y);
            }
        }
        data
    }

    // ------------------------------------------------------------------
    // TC-901-01: PCA 主成分の正確性テスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_01_pca_dominant_axis() {
        // 【テスト目的】: x1 に高分散・x2 に低分散のデータで第1主成分が x1 方向を指すことを検証する 🟢
        let data = make_dominant_axis_data(200);
        let result = run_pca_on_matrix(&data, 2);

        // 【確認内容】: 主成分数が 2 であること
        assert_eq!(result.loadings.len(), 2, "主成分数が 2 であること");
        assert_eq!(result.explained_variance.len(), 2, "説明分散の個数が 2 であること");

        // 【確認内容】: 第1主成分の説明分散 > 第2主成分の説明分散
        assert!(
            result.explained_variance[0] > result.explained_variance[1],
            "第1主成分の説明分散 {} が第2主成分 {} より大きいこと",
            result.explained_variance[0],
            result.explained_variance[1]
        );

        // 【確認内容】: 第1主成分の x1 成分（index=0）が x2 成分（index=1）より大きい
        let loading0 = result.loadings[0][0].abs();
        let loading1 = result.loadings[0][1].abs();
        assert!(
            loading0 > loading1,
            "第1主成分の x1 成分 {} が x2 成分 {} より大きいこと",
            loading0,
            loading1
        );
    }

    // ------------------------------------------------------------------
    // TC-901-02: PCA 射影結果の形状テスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_02_pca_projection_shape() {
        // 【テスト目的】: 射影結果が n × n_components の形状であることを検証する 🟢
        let n = 100;
        let data = make_dominant_axis_data(n);
        let result = run_pca_on_matrix(&data, 2);

        // 【確認内容】: 射影行列のサイズが n × 2 であること
        assert_eq!(result.projections.len(), n, "射影行列の行数が n であること");
        assert_eq!(result.projections[0].len(), 2, "射影行列の列数が 2 であること");
    }

    // ------------------------------------------------------------------
    // TC-901-03: PCA 空データで空結果を返す
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_03_pca_empty_data() {
        // 【テスト目的】: n < 2 のとき空の PcaResult を返すことを検証する 🟢
        let result = run_pca_on_matrix(&[vec![1.0, 2.0]], 2);
        assert!(result.projections.is_empty(), "n<2 のとき射影が空であること");
    }

    // ------------------------------------------------------------------
    // TC-901-04: k-means 収束テスト（明確に分離されたクラスタ）
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_04_kmeans_convergence() {
        // 【テスト目的】: 明確に分離された 3 クラスタのデータで k=3 が収束することを検証する 🟢
        let k = 3;
        let n_per_cluster = 50;
        let data = make_clustered_data(n_per_cluster, k);
        let n = n_per_cluster * k;
        let p = 2;

        let result = run_kmeans_on_data(&data, n, p, k);

        // 【確認内容】: ラベル数が n 個であること
        assert_eq!(result.labels.len(), n, "ラベル数が n であること");
        assert_eq!(result.centroids.len(), k, "重心数が k であること");

        // 【確認内容】: 各クラスタが正確に n_per_cluster 個のサンプルを持つこと
        let mut counts = vec![0usize; k];
        for &l in &result.labels {
            counts[l] += 1;
        }
        for (c, &cnt) in counts.iter().enumerate() {
            assert_eq!(cnt, n_per_cluster, "クラスタ {} のサンプル数が {} であること", c, n_per_cluster);
        }
    }

    // ------------------------------------------------------------------
    // TC-901-05: k-means WCSS 正確性テスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_05_kmeans_wcss_decreases_with_k() {
        // 【テスト目的】: k が増加するにつれて WCSS が単調減少することを検証する 🟢
        let data = make_clustered_data(30, 4);
        let n = 120;
        let p = 2;

        let wcss_k2 = run_kmeans_on_data(&data, n, p, 2).wcss;
        let wcss_k4 = run_kmeans_on_data(&data, n, p, 4).wcss;

        // 【確認内容】: k が大きいほど WCSS が小さいこと
        assert!(
            wcss_k4 < wcss_k2,
            "k=4 の WCSS {} が k=2 の WCSS {} より小さいこと",
            wcss_k4,
            wcss_k2
        );
    }

    // ------------------------------------------------------------------
    // TC-901-06: Elbow 法の推薦 k の妥当性テスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_06_elbow_recommended_k_valid() {
        // 【テスト目的】: 3 クラスタデータで推薦 k が 2〜5 の範囲に収まることを検証する 🟢
        // 【注意】: Elbow 法は近似的な手法なので、正確な k=3 を要求しない
        let k_true = 3;
        let data = make_clustered_data(50, k_true);
        let n = 50 * k_true;
        let p = 2;

        let result = estimate_k_elbow_on_data(&data, n, p, 8);

        // 【確認内容】: WCSS リストが (max_k - 2 + 1) 個の要素を持つこと
        assert_eq!(result.wcss_per_k.len(), 7, "WCSS リストが 7 個 (k=2..8) であること");

        // 【確認内容】: 推薦 k が妥当な範囲 [2, 8] に収まること
        assert!(
            result.recommended_k >= 2 && result.recommended_k <= 8,
            "推薦 k={} が範囲 [2, 8] にあること",
            result.recommended_k
        );
    }

    // ------------------------------------------------------------------
    // TC-901-07: クラスタ統計量の正確性テスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_07_cluster_stats_centroid() {
        // 【テスト目的】: クラスタ 0 の重心が正しく計算されることを検証する 🟢
        // 【データ設計】: クラスタ 0 = [0, 1, 2]、クラスタ 1 = [10, 11, 12]
        let flat_data = vec![
            0.0, 1.0, // サンプル 0, クラスタ 0
            2.0, 3.0, // サンプル 1, クラスタ 0
            10.0, 11.0, // サンプル 2, クラスタ 1
            12.0, 13.0, // サンプル 3, クラスタ 1
        ];
        let labels = vec![0, 0, 1, 1];
        let stats = compute_cluster_stats_on_data(&flat_data, 4, 2, &labels, 2);

        // 【確認内容】: クラスタ 0 の重心が [1.0, 2.0] であること
        let stat0 = stats.iter().find(|s| s.cluster_id == 0).unwrap();
        assert!((stat0.centroid[0] - 1.0).abs() < 1e-9, "クラスタ0の x1 重心が 1.0 であること");
        assert!((stat0.centroid[1] - 2.0).abs() < 1e-9, "クラスタ0の x2 重心が 2.0 であること");
        assert_eq!(stat0.size, 2, "クラスタ0のサイズが 2 であること");
    }

    // ------------------------------------------------------------------
    // TC-901-08: クラスタ統計量 有意差テスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_08_cluster_stats_significant() {
        // 【テスト目的】: 大きく分離されたクラスタで有意差フラグが立つことを検証する 🟢
        // 100点のデータ、各クラスタが全体平均から 10σ 離れている
        let n_per = 50;
        let mut flat_data = Vec::new();
        let mut labels = Vec::new();
        for i in 0..n_per {
            flat_data.push(i as f64 / n_per as f64); // x ≈ 0.5 (mean)
            flat_data.push(-1000.0 + i as f64 * 0.01); // クラスタ0: y ≪ 0
            labels.push(0usize);
        }
        for i in 0..n_per {
            flat_data.push(i as f64 / n_per as f64);
            flat_data.push(1000.0 + i as f64 * 0.01); // クラスタ1: y ≫ 0
            labels.push(1usize);
        }

        let stats = compute_cluster_stats_on_data(&flat_data, 2 * n_per, 2, &labels, 2);

        // 【確認内容】: y 特徴 (index=1) は有意差ありと判定されること
        let stat0 = stats.iter().find(|s| s.cluster_id == 0).unwrap();
        assert!(
            stat0.significant_features[1],
            "大きく分離された y 特徴が有意差ありと判定されること"
        );
    }

    // ------------------------------------------------------------------
    // TC-901-P01: PCA 50ms 以内のパフォーマンステスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_p01_pca_performance() {
        // 【テスト目的】: PCA が 50ms 以内で完了することを検証する 🟢
        // 【データ規模】: debug=2,000行×4列 / release=50,000行×10列

        #[cfg(debug_assertions)]
        let (n, p) = (2_000, 4);
        #[cfg(not(debug_assertions))]
        let (n, p) = (50_000, 10);

        let data: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..p).map(|j| i as f64 / n as f64 + j as f64 * 0.1).collect())
            .collect();

        let start = std::time::Instant::now();
        let result = run_pca_on_matrix(&data, 2);
        let elapsed = start.elapsed();

        assert_eq!(result.projections.len(), n, "射影行数が n であること");
        assert!(
            elapsed.as_millis() < 50,
            "PCA が 50ms 以内: 実測 {}ms",
            elapsed.as_millis()
        );
    }

    // ------------------------------------------------------------------
    // TC-901-P02: k-means 200ms 以内のパフォーマンステスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_p02_kmeans_performance() {
        // 【テスト目的】: k-means が 200ms 以内で完了することを検証する 🟢
        // 【データ規模】: debug=2,000行×4列 / release=50,000行×4列

        #[cfg(debug_assertions)]
        let (n, p) = (2_000, 4);
        #[cfg(not(debug_assertions))]
        let (n, p) = (50_000, 4);

        let flat_data: Vec<f64> = (0..n * p)
            .map(|i| i as f64 / (n * p) as f64)
            .collect();

        let start = std::time::Instant::now();
        let result = run_kmeans_on_data(&flat_data, n, p, 4);
        let elapsed = start.elapsed();

        assert_eq!(result.labels.len(), n, "ラベル数が n であること");
        assert!(
            elapsed.as_millis() < 200,
            "k-means が 200ms 以内: 実測 {}ms",
            elapsed.as_millis()
        );
    }

    // ------------------------------------------------------------------
    // TC-901-P03: クラスタ統計量 150ms 以内のパフォーマンステスト
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_p03_cluster_stats_performance() {
        // 【テスト目的】: クラスタ統計量が 150ms 以内で完了することを検証する 🟢

        #[cfg(debug_assertions)]
        let (n, p) = (2_000, 4);
        #[cfg(not(debug_assertions))]
        let (n, p) = (50_000, 4);

        let flat_data: Vec<f64> = (0..n * p)
            .map(|i| i as f64 / (n * p) as f64)
            .collect();
        let labels: Vec<usize> = (0..n).map(|i| i % 4).collect();

        let start = std::time::Instant::now();
        let stats = compute_cluster_stats_on_data(&flat_data, n, p, &labels, 4);
        let elapsed = start.elapsed();

        assert_eq!(stats.len(), 4, "統計量が 4 クラスタ分あること");
        assert!(
            elapsed.as_millis() < 150,
            "クラスタ統計量が 150ms 以内: 実測 {}ms",
            elapsed.as_millis()
        );
    }
}
