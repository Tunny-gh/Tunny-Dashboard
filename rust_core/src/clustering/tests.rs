use super::*;

#[test]
fn tc_2262_05_kmeans_plusplus_correct_clusters_after_refactor() {
    let k = 3;
    let n_per = 50;
    let data = make_clustered_data(n_per, k);
    let n = n_per * k;
    let p = 2;
    let result = run_kmeans_on_data(&data, n, p, k, InitStrategy::KMeansPlusPlus);
    let mut counts = vec![0usize; k];
    for &label in &result.labels {
        counts[label] += 1;
    }
    for (c, &count) in counts.iter().enumerate() {
        assert_eq!(count, n_per, "cluster {} should have {} points", c, n_per);
    }
}

#[test]
fn tc_2262_06_deterministic_correct_clusters_after_refactor() {
    let k = 3;
    let n_per = 50;
    let data = make_clustered_data(n_per, k);
    let n = n_per * k;
    let p = 2;
    let result = run_kmeans_on_data(&data, n, p, k, InitStrategy::Deterministic);
    // centroids should be near the true cluster centers (0, 100, 200)
    let mut cx: Vec<f64> = result.centroids.iter().map(|c| c[0]).collect();
    cx.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    assert!(cx[0] < 1.0, "first centroid x < 1.0, got {}", cx[0]);
    assert!(
        cx[1] > 99.0 && cx[1] < 101.0,
        "second centroid x ~100, got {}",
        cx[1]
    );
    assert!(cx[2] > 199.0, "third centroid x > 199.0, got {}", cx[2]);
}

/// Generates 2-feature data where the 1st feature's variance greatly exceeds the 2nd feature's.
fn make_dominant_axis_data(n: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|i| {
            let x1 = i as f64 / n as f64 * 10.0;
            let x2 = 0.01 * (i as f64 / n as f64);
            vec![x1, x2]
        })
        .collect()
}

/// Generates flat, row-major data (2 features) with points arranged around
/// k cluster centers (0, 100, 200, ...).
fn make_clustered_data(n_per_cluster: usize, k: usize) -> Vec<f64> {
    let mut data = Vec::with_capacity(n_per_cluster * k * 2);
    for c in 0..k {
        let center = (c as f64) * 100.0;
        for i in 0..n_per_cluster {
            let x = center + (i as f64) * 0.01;
            let y = center + (i as f64) * 0.01;
            data.push(x);
            data.push(y);
        }
    }
    data
}

// ---- TASK-2267: k-means unnecessary clone reduction regression tests ----

#[test]
fn tc_2267_01_kmeans_plusplus_determinism_after_clone_reduction() {
    let k = 3;
    let n_per = 30;
    let data = make_clustered_data(n_per, k);
    let n = n_per * k;
    let p = 2;
    let r1 = run_kmeans_on_data(&data, n, p, k, InitStrategy::KMeansPlusPlus);
    let r2 = run_kmeans_on_data(&data, n, p, k, InitStrategy::KMeansPlusPlus);
    assert_eq!(r1.labels, r2.labels, "same seed must produce same labels");
    for (c1, c2) in r1.centroids.iter().zip(r2.centroids.iter()) {
        for (v1, v2) in c1.iter().zip(c2.iter()) {
            assert!((v1 - v2).abs() < 1e-12, "centroids must be identical");
        }
    }
}

#[test]
fn tc_2267_02_deterministic_init_matches_after_extend_from_slice() {
    let k = 3;
    let n_per = 30;
    let data = make_clustered_data(n_per, k);
    let n = n_per * k;
    let p = 2;
    let r1 = run_kmeans_on_data(&data, n, p, k, InitStrategy::Deterministic);
    let r2 = run_kmeans_on_data(&data, n, p, k, InitStrategy::Deterministic);
    assert_eq!(r1.labels, r2.labels, "deterministic init must be identical");
}

#[test]
fn tc_2267_03_empty_cluster_fallback_preserves_centroid() {
    // Single point per "cluster" in 1D → force empty-cluster fallback path
    let data = vec![0.0, 1.0, 100.0, 101.0];
    let n = 4;
    let p = 1;
    let k = 2;
    let result = run_kmeans_on_data(&data, n, p, k, InitStrategy::Deterministic);
    assert_eq!(result.centroids.len(), k);
    for c in &result.centroids {
        assert!(c[0].is_finite(), "centroid must be finite after fallback");
    }
}

#[test]
fn tc_901_01_pca_dominant_axis() {
    let data = make_dominant_axis_data(200);
    let result = run_pca_on_matrix(&data, 2);

    assert_eq!(result.loadings.len(), 2, "loadings は 2 成分あるべき");
    assert_eq!(
        result.explained_variance.len(),
        2,
        "explained_variance は 2 成分あるべき"
    );

    assert!(
        result.explained_variance[0] > result.explained_variance[1],
        "第 1 主成分の分散 {} は第 2 主成分の分散 {} より大きいべき",
        result.explained_variance[0],
        result.explained_variance[1]
    );

    let loading0 = result.loadings[0][0].abs();
    let loading1 = result.loadings[0][1].abs();
    assert!(
        loading0 > loading1,
        "第 1 主成分は x1 の寄与 {} が x2 の寄与 {} より大きいべき",
        loading0,
        loading1
    );
}

#[test]
fn tc_901_02_pca_projection_shape() {
    let n = 100;
    let data = make_dominant_axis_data(n);
    let result = run_pca_on_matrix(&data, 2);

    assert_eq!(result.projections.len(), n, "射影は n 行あるべき");
    assert_eq!(result.projections[0].len(), 2, "射影は 2 成分あるべき");
}

#[test]
fn tc_901_03_pca_empty_data() {
    let result = run_pca_on_matrix(&[vec![1.0, 2.0]], 2);
    assert!(result.projections.is_empty(), "n<2 は空結果を返すべき");
}

#[test]
fn pca_ragged_rows_return_empty_without_panic() {
    // Ragged-length rows must return an empty result without an out-of-bounds access.
    let data = vec![vec![1.0, 2.0], vec![3.0], vec![4.0, 5.0, 6.0]];
    let result = run_pca_on_matrix(&data, 2);
    assert!(result.projections.is_empty());
    assert!(result.loadings.is_empty());
    let standardized = super::pca::run_pca_on_matrix_opts(&data, 2, true);
    assert!(standardized.projections.is_empty());
}

#[test]
fn pca_explained_ratio_sums_to_at_most_one() {
    let data = make_dominant_axis_data(100);
    let result = run_pca_on_matrix(&data, 2);
    assert_eq!(result.explained_ratio.len(), 2);
    let sum: f64 = result.explained_ratio.iter().sum();
    assert!(sum > 0.0 && sum <= 1.0 + 1e-9, "ratio sum = {sum}");
    assert!(result.explained_ratio[0] >= result.explained_ratio[1]);
}

#[test]
fn pca_standardized_is_column_scale_invariant() {
    // With standardized PCA, scaling a column by 1000x doesn't change the explained ratio (correlation-matrix PCA).
    let data = make_dominant_axis_data(100);
    let scaled: Vec<Vec<f64>> = data
        .iter()
        .map(|row| {
            let mut r = row.clone();
            r[1] *= 1000.0;
            r
        })
        .collect();
    let a = super::pca::run_pca_on_matrix_opts(&data, 2, true);
    let b = super::pca::run_pca_on_matrix_opts(&scaled, 2, true);
    for (ra, rb) in a.explained_ratio.iter().zip(&b.explained_ratio) {
        assert!((ra - rb).abs() < 1e-9, "{ra} vs {rb}");
    }
}

#[test]
fn pca_standardized_zero_variance_column_is_inert() {
    // A zero-variance column becomes 0 after standardization, keeping loadings aligned while contributing nothing.
    let data: Vec<Vec<f64>> = (0..50)
        .map(|i| vec![i as f64, 7.0, (i as f64 * 0.5).sin()])
        .collect();
    let result = super::pca::run_pca_on_matrix_opts(&data, 2, true);
    assert_eq!(result.loadings[0].len(), 3, "3 features stay aligned");
    assert!(result.projections.iter().all(|p| p.len() == 2));
}

#[test]
fn tc_901_04_kmeans_convergence() {
    let k = 3;
    let n_per_cluster = 50;
    let data = make_clustered_data(n_per_cluster, k);
    let n = n_per_cluster * k;
    let p = 2;

    let result = run_kmeans_on_data(&data, n, p, k, InitStrategy::Deterministic);

    assert_eq!(result.labels.len(), n, "ラベルは n 個あるべき");
    assert_eq!(result.centroids.len(), k, "セントロイドは k 個あるべき");

    let mut counts = vec![0usize; k];
    for &label in &result.labels {
        counts[label] += 1;
    }
    for (cluster_id, &count) in counts.iter().enumerate() {
        assert_eq!(
            count, n_per_cluster,
            "クラスタ {} の点数は {} であるべき",
            cluster_id, n_per_cluster
        );
    }
}

#[test]
fn tc_901_05_kmeans_wcss_decreases_with_k() {
    let data = make_clustered_data(30, 4);
    let n = 120;
    let p = 2;

    let wcss_k2 = run_kmeans_on_data(&data, n, p, 2, InitStrategy::Deterministic).wcss;
    let wcss_k4 = run_kmeans_on_data(&data, n, p, 4, InitStrategy::Deterministic).wcss;

    assert!(
        wcss_k4 < wcss_k2,
        "k=4 の WCSS {} は k=2 の WCSS {} より小さいべき",
        wcss_k4,
        wcss_k2
    );
}

#[test]
fn tc_901_06_elbow_recommended_k_valid() {
    let k_true = 3;
    let data = make_clustered_data(50, k_true);
    let n = 50 * k_true;
    let p = 2;

    let result = estimate_k_elbow_on_data(&data, n, p, 8);

    assert_eq!(result.wcss_per_k.len(), 7, "WCSS は 7 個 (k=2..8) あるべき");

    assert!(
        result.recommended_k >= 2 && result.recommended_k <= 8,
        "推奨 k={} は範囲 [2, 8] に収まるべき",
        result.recommended_k
    );
}

#[test]
fn tc_901_p01_pca_performance() {
    #[cfg(debug_assertions)]
    let (n, p) = (2_000, 4);
    #[cfg(not(debug_assertions))]
    let (n, p) = (50_000, 10);

    let data: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..p)
                .map(|j| i as f64 / n as f64 + j as f64 * 0.1)
                .collect()
        })
        .collect();

    let result = run_pca_on_matrix(&data, 2);

    assert_eq!(result.projections.len(), n, "射影は n 行あるべき");
}

#[test]
fn tc_901_p02_kmeans_performance() {
    #[cfg(debug_assertions)]
    let (n, p) = (100, 4);
    #[cfg(not(debug_assertions))]
    let (n, p) = (50_000, 4);

    let flat_data: Vec<f64> = (0..n * p).map(|i| i as f64 / (n * p) as f64).collect();

    let result = run_kmeans_on_data(&flat_data, n, p, 4, InitStrategy::Deterministic);

    assert_eq!(result.labels.len(), n, "ラベルは n 個あるべき");
    assert!(
        result.labels.iter().all(|&c| c < 4),
        "every label must fall in one of the 4 clusters"
    );
}

#[test]
fn tc_101_01_pca_p20_works() {
    let p = 20;
    let n = 50;
    let data: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..p).map(|j| (i + j) as f64).collect())
        .collect();
    let result = run_pca_on_matrix(&data, 3);
    assert_eq!(result.projections.len(), n);
    assert_eq!(result.loadings.len(), 3);
    assert_eq!(result.loadings[0].len(), p);
    assert_eq!(result.explained_variance.len(), 3);
    assert!(result.explained_variance[0] >= result.explained_variance[1]);
}

#[test]
fn tc_101_b01_pca_n2_p2_minimum() {
    let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let result = run_pca_on_matrix(&data, 1);
    assert_eq!(result.projections.len(), 2);
    assert_eq!(result.loadings.len(), 1);
    assert_eq!(result.loadings[0].len(), 2);
}

#[test]
fn tc_101_b02_pca_n1_returns_empty() {
    let data = vec![vec![1.0, 2.0]];
    let result = run_pca_on_matrix(&data, 1);
    assert!(result.projections.is_empty());
}
