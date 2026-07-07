//! 自己組織化マップ（SOM、バッチ学習）。
//!
//! 標準化した特徴空間で矩形グリッドのバッチ SOM を学習し、U-matrix・
//! ヒットカウント・成分プレーン（元単位）を提供する。初期化は PCA の
//! 第 1・第 2 主成分平面に沿った決定論的な線形初期化で、シードに依存しない
//! 再現可能な地図を得る。理論的背景は theory/{en,ja}/clustering/som.md。

use super::pca::run_pca_on_matrix_opts;

/// SOM の学習仕様。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SomSpec {
    /// グリッド幅（ノード列数）。
    pub grid_w: usize,
    /// グリッド高さ（ノード行数）。
    pub grid_h: usize,
    /// バッチエポック数。
    pub n_epochs: usize,
}

impl Default for SomSpec {
    fn default() -> Self {
        Self {
            grid_w: 8,
            grid_h: 8,
            n_epochs: 20,
        }
    }
}

/// SOM の学習結果。ノードは行優先（node = y * grid_w + x）で格納する。
#[derive(Debug, Clone)]
pub struct SomResult {
    pub grid_w: usize,
    pub grid_h: usize,
    /// ノード重み（標準化空間）。`weights[node][feature]`。
    pub weights: Vec<Vec<f64>>,
    /// U-matrix: 各ノードの隣接ノード（上下左右）との平均距離（標準化空間）。
    pub u_matrix: Vec<f64>,
    /// 各ノードが BMU になった行数。
    pub hits: Vec<usize>,
    /// 各データ行の BMU ノードインデックス。
    pub bmu: Vec<usize>,
    /// 標準化に使った列平均（成分プレーンの逆変換用）。
    pub feature_means: Vec<f64>,
    /// 標準化に使った列標準偏差（分散ゼロ列は 0）。
    pub feature_stds: Vec<f64>,
}

impl SomResult {
    /// 特徴 j の成分プレーンを元単位で返す（`weights` の逆標準化）。
    pub fn component_plane(&self, feature: usize) -> Vec<f64> {
        self.weights
            .iter()
            .map(|w| w[feature] * self.feature_stds[feature] + self.feature_means[feature])
            .collect()
    }
}

/// バッチ学習のエポック内で使う最大行数。超える場合は等間隔サブサンプルした
/// 行のみで重みを更新する（hierarchical の行数キャップと同じ慣行・決定論的）。
/// BMU・ヒット・U-matrix は全行に対して計算するため出力の形は変わらない。
pub const MAX_SOM_TRAINING_ROWS: usize = 800;

/// バッチ SOM を学習する。行数 3 未満・特徴 0・グリッド 2x2 未満は `None`。
///
/// データは内部で標準化される（分散ゼロ列は 0 に写像され地図に寄与しない）。
/// 近傍幅 σ はエポックに沿って `max(grid_w, grid_h)/2` から 0.5 へ指数減衰する。
/// 行数が [`MAX_SOM_TRAINING_ROWS`] を超える場合、エポック内の重み更新のみ
/// 等間隔サブサンプルで行う（結果は行数によらず決定論的）。
pub fn train_som(data: &[Vec<f64>], spec: &SomSpec) -> Option<SomResult> {
    let n = data.len();
    if n < 3 || data[0].is_empty() || spec.grid_w < 2 || spec.grid_h < 2 || spec.n_epochs == 0 {
        return None;
    }
    let p = data[0].len();
    let n_nodes = spec.grid_w * spec.grid_h;

    // ── 標準化（clustering 共通ヘルパ、母分散 n）─────────────────
    let mut x: Vec<Vec<f64>> = data.to_vec();
    let (means, stds) = super::standardize::standardize_columns(&mut x, 0);

    // ── PCA 平面に沿った決定論的線形初期化 ──────────────────────
    // 標準化済みデータの上位 2 主成分方向に ±2√λ の範囲でグリッドを張る。
    // 成分が縮退している場合は 0 ベクトル初期化（バッチ更新で即座に動く）。
    let pca = run_pca_on_matrix_opts(&x, 2, false);
    let axis = |comp: usize| -> (Vec<f64>, f64) {
        let dir = pca
            .loadings
            .get(comp)
            .cloned()
            .unwrap_or_else(|| vec![0.0; p]);
        let scale = pca
            .explained_variance
            .get(comp)
            .copied()
            .unwrap_or(0.0)
            .max(0.0)
            .sqrt();
        (dir, scale)
    };
    let (dir1, s1) = axis(0);
    let (dir2, s2) = axis(1);
    let mut weights: Vec<Vec<f64>> = Vec::with_capacity(n_nodes);
    for gy in 0..spec.grid_h {
        for gx in 0..spec.grid_w {
            let a = if spec.grid_w > 1 {
                (gx as f64 / (spec.grid_w - 1) as f64) * 4.0 - 2.0
            } else {
                0.0
            };
            let b = if spec.grid_h > 1 {
                (gy as f64 / (spec.grid_h - 1) as f64) * 4.0 - 2.0
            } else {
                0.0
            };
            weights.push(
                (0..p)
                    .map(|j| a * s1 * dir1[j] + b * s2 * dir2[j])
                    .collect(),
            );
        }
    }

    let d2 =
        |a: &[f64], b: &[f64]| -> f64 { a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum() };
    let node_xy =
        |node: usize| -> (f64, f64) { ((node % spec.grid_w) as f64, (node / spec.grid_w) as f64) };
    let find_bmu = |weights: &[Vec<f64>], row: &[f64]| -> usize {
        let mut best = 0;
        let mut best_d = f64::INFINITY;
        for (i, w) in weights.iter().enumerate() {
            let d = d2(w, row);
            if d < best_d {
                best_d = d;
                best = i;
            }
        }
        best
    };

    // ── 学習行のサブサンプル（等間隔・決定論的）──────────────────
    // 行数キャップ以下なら全行（従来と同一の挙動）。
    let train_indices: Vec<usize> = if n > MAX_SOM_TRAINING_ROWS {
        let step = n as f64 / MAX_SOM_TRAINING_ROWS as f64;
        (0..MAX_SOM_TRAINING_ROWS)
            .map(|i| ((i as f64 * step) as usize).min(n - 1))
            .collect()
    } else {
        (0..n).collect()
    };

    // ── バッチ学習 ──────────────────────────────────────────────
    let sigma0 = (spec.grid_w.max(spec.grid_h)) as f64 / 2.0;
    let sigma_end = 0.5f64;
    for epoch in 0..spec.n_epochs {
        let t = epoch as f64 / (spec.n_epochs.max(2) - 1) as f64;
        let sigma = sigma0 * (sigma_end / sigma0).powf(t);
        let two_sigma2 = 2.0 * sigma * sigma;

        // 各行の BMU を確定し、近傍カーネルで重み付き平均を取る。
        let mut num = vec![vec![0.0f64; p]; n_nodes];
        let mut den = vec![0.0f64; n_nodes];
        for row in train_indices.iter().map(|&ri| &x[ri]) {
            let bmu = find_bmu(&weights, row);
            let (bx, by) = node_xy(bmu);
            for node in 0..n_nodes {
                let (nx, ny) = node_xy(node);
                let g2 = (nx - bx).powi(2) + (ny - by).powi(2);
                let h = (-g2 / two_sigma2).exp();
                if h < 1e-6 {
                    continue;
                }
                for j in 0..p {
                    num[node][j] += h * row[j];
                }
                den[node] += h;
            }
        }
        for node in 0..n_nodes {
            if den[node] > 1e-12 {
                for j in 0..p {
                    weights[node][j] = num[node][j] / den[node];
                }
            }
        }
    }

    // ── BMU・ヒット・U-matrix ────────────────────────────────────
    let bmu: Vec<usize> = x.iter().map(|row| find_bmu(&weights, row)).collect();
    let mut hits = vec![0usize; n_nodes];
    for &b in &bmu {
        hits[b] += 1;
    }
    let mut u_matrix = vec![0.0f64; n_nodes];
    for node in 0..n_nodes {
        let (gx, gy) = (node % spec.grid_w, node / spec.grid_w);
        let mut sum = 0.0;
        let mut count = 0usize;
        let mut push = |ox: isize, oy: isize| {
            let (nx, ny) = (gx as isize + ox, gy as isize + oy);
            if nx >= 0 && ny >= 0 && (nx as usize) < spec.grid_w && (ny as usize) < spec.grid_h {
                let neighbor = ny as usize * spec.grid_w + nx as usize;
                sum += d2(&weights[node], &weights[neighbor]).sqrt();
                count += 1;
            }
        };
        push(-1, 0);
        push(1, 0);
        push(0, -1);
        push(0, 1);
        u_matrix[node] = if count > 0 { sum / count as f64 } else { 0.0 };
    }

    Some(SomResult {
        grid_w: spec.grid_w,
        grid_h: spec.grid_h,
        weights,
        u_matrix,
        hits,
        bmu,
        feature_means: means,
        feature_stds: stds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blobs() -> Vec<Vec<f64>> {
        let mut data = Vec::new();
        for i in 0..15 {
            data.push(vec![i as f64 * 0.01, 0.0]);
            data.push(vec![10.0 + i as f64 * 0.01, 5.0]);
        }
        data
    }

    #[test]
    fn shapes_are_consistent() {
        let r = train_som(&blobs(), &SomSpec::default()).unwrap();
        assert_eq!(r.weights.len(), 64);
        assert_eq!(r.u_matrix.len(), 64);
        assert_eq!(r.hits.len(), 64);
        assert_eq!(r.bmu.len(), 30);
        assert_eq!(r.hits.iter().sum::<usize>(), 30);
        assert_eq!(r.component_plane(0).len(), 64);
    }

    #[test]
    fn deterministic_without_seed() {
        // PCA 初期化 + バッチ更新は完全に決定論的。
        let a = train_som(&blobs(), &SomSpec::default()).unwrap();
        let b = train_som(&blobs(), &SomSpec::default()).unwrap();
        assert_eq!(a.bmu, b.bmu);
        assert_eq!(a.u_matrix, b.u_matrix);
    }

    #[test]
    fn separated_blobs_map_to_different_nodes() {
        let r = train_som(&blobs(), &SomSpec::default()).unwrap();
        // 2 つの塊の BMU 集合が交わらない（結線確認、地図品質は問わない）。
        let set_a: std::collections::HashSet<usize> = r.bmu.iter().step_by(2).copied().collect();
        let set_b: std::collections::HashSet<usize> =
            r.bmu.iter().skip(1).step_by(2).copied().collect();
        assert!(set_a.is_disjoint(&set_b), "{set_a:?} vs {set_b:?}");
    }

    #[test]
    fn component_plane_is_in_original_units() {
        let r = train_som(&blobs(), &SomSpec::default()).unwrap();
        let plane = r.component_plane(0);
        // 元単位: x0 は 0 付近と 10 付近の 2 群 → プレーンの範囲がそのスケールに乗る。
        let max = plane.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(max > 1.0, "destandardized plane should reach data scale");
    }

    #[test]
    fn caps_training_rows_but_outputs_full_shapes() {
        // 学習キャップを超えても BMU/ヒットは全行分返り、決定性が保たれる。
        let n = MAX_SOM_TRAINING_ROWS + 50;
        let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (i % 7) as f64]).collect();
        let spec = SomSpec {
            grid_w: 4,
            grid_h: 4,
            n_epochs: 3,
        };
        let a = train_som(&data, &spec).unwrap();
        assert_eq!(a.bmu.len(), n);
        assert_eq!(a.hits.iter().sum::<usize>(), n);
        let b = train_som(&data, &spec).unwrap();
        assert_eq!(a.bmu, b.bmu);
    }

    #[test]
    fn rejects_degenerate_input() {
        assert!(train_som(&[vec![1.0], vec![2.0]], &SomSpec::default()).is_none());
        let spec = SomSpec {
            grid_w: 1,
            ..Default::default()
        };
        assert!(train_som(&blobs(), &spec).is_none());
    }
}
