use linfa::prelude::*;
use linfa_clustering::KMeans;
use ndarray::Array2;
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;

use super::types::{ElbowResult, InitStrategy, KmeansResult};

fn flat_to_array2(flat_data: &[f64], n: usize, p: usize) -> Array2<f64> {
    Array2::from_shape_vec((n, p), flat_data.to_vec())
        .unwrap_or_else(|_| Array2::zeros((0, p)))
}

fn make_seed(init: InitStrategy, n: usize, k: usize) -> u64 {
    match init {
        InitStrategy::KMeansPlusPlus => {
            let s = (n as u64)
                .wrapping_mul(0x9e3779b97f4a7c15)
                ^ ((k as u64).wrapping_mul(0x6c62272e07bb0142));
            s.max(1)
        }
        InitStrategy::Deterministic => 42,
    }
}

pub(crate) fn run_kmeans_on_data(
    flat_data: &[f64],
    n: usize,
    p: usize,
    k: usize,
    init: InitStrategy,
) -> KmeansResult {
    let empty =
        KmeansResult { labels: vec![0; n], centroids: vec![], wcss: 0.0, iterations: 0 };

    if n < k || k == 0 || p == 0 || flat_data.len() < n * p {
        return empty;
    }

    let arr = flat_to_array2(flat_data, n, p);
    let dataset = Dataset::from(arr.clone());
    let rng = Xoshiro256Plus::seed_from_u64(make_seed(init, n, k));

    let model = match KMeans::params_with_rng(k, rng)
        .max_n_iterations(300)
        .tolerance(1e-5)
        .fit(&dataset)
    {
        Ok(m) => m,
        Err(_) => return empty,
    };

    let labels: Vec<usize> = model.predict(arr.view()).targets().to_vec();
    let centroids_arr = model.centroids();
    let centroids: Vec<Vec<f64>> =
        (0..k).map(|i| (0..p).map(|j| centroids_arr[[i, j]]).collect()).collect();

    KmeansResult { labels, centroids, wcss: model.inertia(), iterations: 300 }
}

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
    ElbowResult { wcss_per_k, recommended_k }
}

pub fn run_kmeans(k: usize, flat_data: &[f64], n_cols: usize, init: InitStrategy) -> KmeansResult {
    if n_cols == 0 || flat_data.is_empty() {
        return KmeansResult { labels: vec![], centroids: vec![], wcss: 0.0, iterations: 0 };
    }
    let n = flat_data.len() / n_cols;
    run_kmeans_on_data(flat_data, n, n_cols, k, init)
}

pub fn estimate_k_elbow(flat_data: &[f64], n_cols: usize, max_k: usize) -> ElbowResult {
    if n_cols == 0 || flat_data.is_empty() {
        return ElbowResult { wcss_per_k: vec![], recommended_k: 2 };
    }
    let n = flat_data.len() / n_cols;
    estimate_k_elbow_on_data(flat_data, n, n_cols, max_k)
}
