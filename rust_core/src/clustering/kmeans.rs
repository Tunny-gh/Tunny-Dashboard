use super::types::{ElbowResult, KmeansResult};

/// Documentation.
#[inline]
fn sq_dist(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum()
}

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
///
/// 【parameter】:
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
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

    let get_point = |i: usize| -> &[f64] { &flat_data[i * p..(i + 1) * p] };

    let mut centroids: Vec<Vec<f64>> = Vec::with_capacity(k);
    centroids.push(get_point(n / 2).to_vec());

    for _ in 1..k {
        let mut distances: Vec<f64> = (0..n)
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
        for (i, &distance) in distances.iter().enumerate() {
            cum += distance;
            if cum >= threshold {
                chosen = i;
                break;
            }
        }
        distances.clear();
        centroids.push(get_point(chosen).to_vec());
    }

    let mut labels = vec![0usize; n];
    let max_iter = 300;

    for iter in 0..max_iter {
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
        .map(|k| run_kmeans_on_data(flat_data, n, p, k).wcss)
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

/// Documentation.
///
/// Documentation.
pub fn run_kmeans(k: usize, flat_data: &[f64], n_cols: usize) -> KmeansResult {
    if n_cols == 0 || flat_data.is_empty() {
        return KmeansResult {
            labels: vec![],
            centroids: vec![],
            wcss: 0.0,
            iterations: 0,
        };
    }
    let n = flat_data.len() / n_cols;
    run_kmeans_on_data(flat_data, n, n_cols, k)
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
