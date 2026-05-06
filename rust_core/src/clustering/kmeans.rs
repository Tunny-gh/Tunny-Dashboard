use super::types::{ElbowResult, InitStrategy, KmeansResult};

#[inline]
fn sq_dist(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum()
}

/// xorshift64 PRNG（外部クレート不要、再現可能な固定シード）
#[inline]
fn xorshift64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// [0.0, 1.0) の一様乱数
#[inline]
fn uniform_f64(state: &mut u64) -> f64 {
    (xorshift64(state) >> 11) as f64 / (1u64 << 53) as f64
}

/// k-means++ 初期化: D² 比例確率でサンプリング
fn init_kmeans_plusplus(
    flat_data: &[f64],
    n: usize,
    p: usize,
    k: usize,
    rng: &mut u64,
) -> Vec<Vec<f64>> {
    let get_point = |i: usize| -> &[f64] { &flat_data[i * p..(i + 1) * p] };

    let mut centroids: Vec<Vec<f64>> = Vec::with_capacity(k);
    // 最初の重心: n/2 番目（決定論的なベースポイント）
    centroids.push(get_point(n / 2).to_vec());

    for _ in 1..k {
        // 各点から最近傍重心への D² を計算
        let d2: Vec<f64> = (0..n)
            .map(|i| {
                centroids
                    .iter()
                    .map(|c| sq_dist(get_point(i), c))
                    .fold(f64::INFINITY, f64::min)
            })
            .collect();

        let total: f64 = d2.iter().sum();
        if total < f64::EPSILON {
            // 全点が重なっている場合はフォールバック
            centroids.push(get_point(centroids.len() % n).to_vec());
            continue;
        }

        // D² 比例確率でサンプリング
        let threshold = uniform_f64(rng) * total;
        let mut cum = 0.0;
        let mut chosen = n - 1;
        for (i, &d) in d2.iter().enumerate() {
            cum += d;
            if cum >= threshold {
                chosen = i;
                break;
            }
        }
        centroids.push(get_point(chosen).to_vec());
    }

    centroids
}

/// 決定論的スプレッド初期化: 累積距離しきい値で等間隔選択
fn init_deterministic(flat_data: &[f64], n: usize, p: usize, k: usize) -> Vec<Vec<f64>> {
    let get_point = |i: usize| -> &[f64] { &flat_data[i * p..(i + 1) * p] };

    let mut centroids: Vec<Vec<f64>> = Vec::with_capacity(k);
    centroids.push(get_point(n / 2).to_vec());

    for _ in 1..k {
        let distances: Vec<f64> = (0..n)
            .map(|i| {
                centroids
                    .iter()
                    .map(|c| sq_dist(get_point(i), c))
                    .fold(f64::INFINITY, f64::min)
            })
            .collect();

        let total: f64 = distances.iter().sum();
        if total < f64::EPSILON {
            let idx = centroids.len() % n;
            centroids.push(get_point(idx).to_vec());
            continue;
        }

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
        centroids.push(get_point(chosen).to_vec());
    }

    centroids
}

pub(crate) fn run_kmeans_on_data(
    flat_data: &[f64],
    n: usize,
    p: usize,
    k: usize,
    init: InitStrategy,
) -> KmeansResult {
    let empty = KmeansResult {
        labels: vec![0; n],
        centroids: vec![],
        wcss: 0.0,
        iterations: 0,
    };

    if n < k || k == 0 || p == 0 || flat_data.len() < n * p {
        return empty;
    }

    let mut centroids = match init {
        InitStrategy::KMeansPlusPlus => {
            // シードを n と k から導出して再現性を保つ
            let mut rng: u64 = (n as u64).wrapping_mul(0x9e3779b97f4a7c15)
                ^ (k as u64).wrapping_mul(0x6c62272e07bb0142);
            if rng == 0 {
                rng = 1;
            }
            init_kmeans_plusplus(flat_data, n, p, k, &mut rng)
        }
        InitStrategy::Deterministic => init_deterministic(flat_data, n, p, k),
    };

    let get_point = |i: usize| -> &[f64] { &flat_data[i * p..(i + 1) * p] };
    let mut labels = vec![0usize; n];
    let max_iter = 300;

    for iter in 0..max_iter {
        let mut changed = false;
        for (i, label) in labels.iter_mut().enumerate().take(n) {
            let pt = get_point(i);
            let new_label = (0..k)
                .min_by(|&a, &b| {
                    sq_dist(pt, &centroids[a])
                        .partial_cmp(&sq_dist(pt, &centroids[b]))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0);
            if *label != new_label {
                *label = new_label;
                changed = true;
            }
        }

        if !changed {
            let wcss = compute_wcss(flat_data, n, p, &labels, &centroids);
            return KmeansResult {
                labels,
                centroids,
                wcss,
                iterations: iter + 1,
            };
        }

        let mut new_centroids = vec![vec![0.0f64; p]; k];
        let mut counts = vec![0usize; k];
        for (i, &lbl) in labels.iter().enumerate().take(n) {
            let pt = get_point(i);
            for j in 0..p {
                new_centroids[lbl][j] += pt[j];
            }
            counts[lbl] += 1;
        }
        for c in 0..k {
            if counts[c] > 0 {
                for val in new_centroids[c].iter_mut().take(p) {
                    *val /= counts[c] as f64;
                }
            } else {
                new_centroids[c] = centroids[c].clone();
            }
        }
        centroids = new_centroids;
    }

    let wcss = compute_wcss(flat_data, n, p, &labels, &centroids);
    KmeansResult {
        labels,
        centroids,
        wcss,
        iterations: max_iter,
    }
}

/// Documentation.
fn compute_wcss(
    flat_data: &[f64],
    n: usize,
    p: usize,
    labels: &[usize],
    centroids: &[Vec<f64>],
) -> f64 {
    (0..n)
        .map(|i| sq_dist(&flat_data[i * p..(i + 1) * p], &centroids[labels[i]]))
        .sum()
}

/// Documentation.
///
/// Documentation.
/// Documentation.
pub(crate) fn estimate_k_elbow_on_data(
    flat_data: &[f64],
    n: usize,
    p: usize,
    max_k: usize,
) -> ElbowResult {
    let effective_max_k = max_k.min(n);
    if effective_max_k < 2 {
        return ElbowResult {
            wcss_per_k: vec![],
            recommended_k: 2,
        };
    }

    let wcss_per_k: Vec<f64> = (2..=effective_max_k)
        .map(|k| run_kmeans_on_data(flat_data, n, p, k, InitStrategy::Deterministic).wcss)
        .collect();

    let recommended_k = if wcss_per_k.len() < 3 {
        wcss_per_k.len() + 1
    } else {
        let second_diffs: Vec<f64> = (0..wcss_per_k.len() - 2)
            .map(|i| wcss_per_k[i] - 2.0 * wcss_per_k[i + 1] + wcss_per_k[i + 2])
            .collect();

        let best_idx = second_diffs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        best_idx + 3
    };

    let recommended_k = recommended_k.clamp(2, effective_max_k);

    ElbowResult {
        wcss_per_k,
        recommended_k,
    }
}

pub fn run_kmeans(k: usize, flat_data: &[f64], n_cols: usize, init: InitStrategy) -> KmeansResult {
    if n_cols == 0 || flat_data.is_empty() {
        return KmeansResult {
            labels: vec![],
            centroids: vec![],
            wcss: 0.0,
            iterations: 0,
        };
    }
    let n = flat_data.len() / n_cols;
    run_kmeans_on_data(flat_data, n, n_cols, k, init)
}

/// Documentation.
pub fn estimate_k_elbow(flat_data: &[f64], n_cols: usize, max_k: usize) -> ElbowResult {
    if n_cols == 0 || flat_data.is_empty() {
        return ElbowResult {
            wcss_per_k: vec![],
            recommended_k: 2,
        };
    }
    let n = flat_data.len() / n_cols;
    estimate_k_elbow_on_data(flat_data, n, n_cols, max_k)
}
