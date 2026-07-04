use super::*;

// ---- TASK-2264: compute_cluster_stats_on_data 3 関数分割テスト ----

/// flat_data (row-major, n×p): 4 rows × 2 cols
/// cluster 0: rows 0,1 → centroid ≈ [1, 2]
/// cluster 1: rows 2,3 → centroid ≈ [11, 12]
fn make_two_cluster_data() -> (Vec<f64>, Vec<usize>, usize, usize, usize) {
    let flat_data = vec![0.0f64, 1.0, 2.0, 3.0, 10.0, 11.0, 12.0, 13.0];
    let labels = vec![0usize, 0, 1, 1];
    let n = 4;
    let p = 2;
    let k = 2;
    (flat_data, labels, n, p, k)
}

#[test]
fn tc_2264_01_compute_global_stats_mean() {
    let (flat_data, _, n, p, _) = make_two_cluster_data();
    // global mean of col0: (0+2+10+12)/4 = 6, col1: (1+3+11+13)/4 = 7
    let (mean, _std) = compute_global_stats(&flat_data, n, p);
    assert_eq!(mean.len(), p);
    assert!(
        (mean[0] - 6.0).abs() < 1e-10,
        "global_mean[0] should be 6.0, got {}",
        mean[0]
    );
    assert!(
        (mean[1] - 7.0).abs() < 1e-10,
        "global_mean[1] should be 7.0, got {}",
        mean[1]
    );
}

#[test]
fn tc_2264_02_compute_global_stats_std() {
    let (flat_data, _, n, p, _) = make_two_cluster_data();
    let (_mean, std) = compute_global_stats(&flat_data, n, p);
    assert_eq!(std.len(), p);
    // var col0 = ((0-6)²+(2-6)²+(10-6)²+(12-6)²)/3 = (36+16+16+36)/3 = 104/3
    let expected_std0 = (104.0f64 / 3.0).sqrt();
    assert!(
        (std[0] - expected_std0).abs() < 1e-6,
        "global_std[0]={} expected={}",
        std[0],
        expected_std0
    );
}

#[test]
fn tc_2264_03_compute_global_stats_empty() {
    let (mean, std) = compute_global_stats(&[], 0, 2);
    assert_eq!(mean, vec![0.0, 0.0]);
    assert_eq!(std, vec![0.0, 0.0]);
}

#[test]
fn tc_2264_04_compute_cluster_centroid_std_centroids() {
    let (flat_data, labels, n, p, k) = make_two_cluster_data();
    let (global_mean, _) = compute_global_stats(&flat_data, n, p);
    let stats = compute_cluster_centroid_std(&flat_data, &labels, n, p, k, &global_mean);
    assert_eq!(stats.len(), k);
    let s0 = stats.iter().find(|s| s.cluster_id == 0).unwrap();
    assert!(
        (s0.centroid[0] - 1.0).abs() < 1e-10,
        "cluster0 centroid[0]={}",
        s0.centroid[0]
    );
    assert!(
        (s0.centroid[1] - 2.0).abs() < 1e-10,
        "cluster0 centroid[1]={}",
        s0.centroid[1]
    );
    let s1 = stats.iter().find(|s| s.cluster_id == 1).unwrap();
    assert!(
        (s1.centroid[0] - 11.0).abs() < 1e-10,
        "cluster1 centroid[0]={}",
        s1.centroid[0]
    );
    assert!(
        (s1.centroid[1] - 12.0).abs() < 1e-10,
        "cluster1 centroid[1]={}",
        s1.centroid[1]
    );
}

#[test]
fn tc_2264_05_compute_cluster_centroid_std_empty_cluster_uses_global_mean() {
    let (flat_data, _, n, p, _) = make_two_cluster_data();
    // labels: cluster 0 has all points, cluster 1 is empty
    let labels = vec![0usize, 0, 0, 0];
    let (global_mean, _) = compute_global_stats(&flat_data, n, p);
    let stats = compute_cluster_centroid_std(&flat_data, &labels, n, p, 2, &global_mean);
    let empty = stats.iter().find(|s| s.cluster_id == 1).unwrap();
    assert_eq!(empty.size, 0);
    for (c, g) in empty.centroid.iter().zip(global_mean.iter()) {
        assert!(
            (c - g).abs() < 1e-10,
            "empty cluster centroid should be global_mean"
        );
    }
    assert!(empty.significant_features.iter().all(|&b| !b));
}

#[test]
fn tc_2264_06_compute_significant_features_detects_significance() {
    // Use well-separated clusters (same as tc_901_08) so t >> 3.0
    let n_per = 50usize;
    let mut flat_data = Vec::new();
    let mut labels = Vec::new();
    for i in 0..n_per {
        flat_data.push(i as f64 / n_per as f64);
        flat_data.push(-1000.0 + i as f64 * 0.01);
        labels.push(0usize);
    }
    for i in 0..n_per {
        flat_data.push(i as f64 / n_per as f64);
        flat_data.push(1000.0 + i as f64 * 0.01);
        labels.push(1usize);
    }
    let n = 2 * n_per;
    let p = 2;
    let k = 2;
    let (global_mean, global_std) = compute_global_stats(&flat_data, n, p);
    let stats = compute_cluster_centroid_std(&flat_data, &labels, n, p, k, &global_mean);
    let stats = compute_significant_features(stats, &global_mean, &global_std, n);
    // y-feature (col 1) is -1000 vs +1000: definitely significant
    let s0 = stats.iter().find(|s| s.cluster_id == 0).unwrap();
    assert!(
        s0.significant_features[1],
        "y feature should be significant for well-separated clusters"
    );
}

#[test]
fn tc_2264_07_orchestrator_matches_original_behavior() {
    let (flat_data, labels, n, p, k) = make_two_cluster_data();
    let stats = compute_cluster_stats_on_data(&flat_data, n, p, &labels, k);
    // Verify same results as existing tests
    assert_eq!(stats.len(), k);
    let s0 = stats.iter().find(|s| s.cluster_id == 0).unwrap();
    assert!((s0.centroid[0] - 1.0).abs() < 1e-10);
    assert!((s0.centroid[1] - 2.0).abs() < 1e-10);
    assert_eq!(s0.size, 2);
}

#[test]
fn tc_2264_08_orchestrator_empty_data() {
    let stats = compute_cluster_stats_on_data(&[], 0, 2, &[], 2);
    assert!(stats.is_empty(), "empty data should return empty stats");
}

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

/// Documentation.
fn make_dominant_axis_data(n: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|i| {
            let x1 = i as f64 / n as f64 * 10.0;
            let x2 = 0.01 * (i as f64 / n as f64);
            vec![x1, x2]
        })
        .collect()
}

/// Documentation.
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

// ---- TASK-2267: k-means 不要クローン削減 回帰テスト ----

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

    assert_eq!(result.loadings.len(), 2, "translated 2 translated");
    assert_eq!(
        result.explained_variance.len(),
        2,
        "translated 2 translated"
    );

    assert!(
        result.explained_variance[0] > result.explained_variance[1],
        "translated1translated {} translated2translated {} translated",
        result.explained_variance[0],
        result.explained_variance[1]
    );

    let loading0 = result.loadings[0][0].abs();
    let loading1 = result.loadings[0][1].abs();
    assert!(
        loading0 > loading1,
        "translated1translated x1 translated {} translated x2 translated {} translated",
        loading0,
        loading1
    );
}

#[test]
fn tc_901_02_pca_projection_shape() {
    let n = 100;
    let data = make_dominant_axis_data(n);
    let result = run_pca_on_matrix(&data, 2);

    assert_eq!(result.projections.len(), n, "translated n translated");
    assert_eq!(result.projections[0].len(), 2, "translated 2 translated");
}

#[test]
fn tc_901_03_pca_empty_data() {
    let result = run_pca_on_matrix(&[vec![1.0, 2.0]], 2);
    assert!(result.projections.is_empty(), "n<2 translated");
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
    // 標準化 PCA では、ある列を 1000 倍しても寄与率は変わらない（相関行列 PCA）。
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
    // 分散ゼロ列は標準化後 0 となり、loadings の整列は保たれつつ寄与しない。
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

    assert_eq!(result.labels.len(), n, "translated n translated");
    assert_eq!(result.centroids.len(), k, "translated k translated");

    let mut counts = vec![0usize; k];
    for &label in &result.labels {
        counts[label] += 1;
    }
    for (cluster_id, &count) in counts.iter().enumerate() {
        assert_eq!(
            count, n_per_cluster,
            "translated {} translated {} translated",
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
        "k=4 translated WCSS {} translated k=2 translated WCSS {} translated",
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

    assert_eq!(
        result.wcss_per_k.len(),
        7,
        "WCSS translated 7 translated (k=2..8) translated"
    );

    assert!(
        result.recommended_k >= 2 && result.recommended_k <= 8,
        "translated k={} translatedrange [2, 8] translated",
        result.recommended_k
    );
}

#[test]
fn tc_901_07_cluster_stats_centroid() {
    let flat_data = vec![0.0, 1.0, 2.0, 3.0, 10.0, 11.0, 12.0, 13.0];
    let labels = vec![0, 0, 1, 1];
    let stats = compute_cluster_stats_on_data(&flat_data, 4, 2, &labels, 2);

    let stat0 = stats.iter().find(|s| s.cluster_id == 0).unwrap();
    assert!(
        (stat0.centroid[0] - 1.0).abs() < 1e-9,
        "translated0translated x1 translated 1.0 translated"
    );
    assert!(
        (stat0.centroid[1] - 2.0).abs() < 1e-9,
        "translated0translated x2 translated 2.0 translated"
    );
    assert_eq!(stat0.size, 2, "translated0translated 2 translated");
}

#[test]
fn tc_901_08_cluster_stats_significant() {
    let n_per = 50;
    let mut flat_data = Vec::new();
    let mut labels = Vec::new();
    for i in 0..n_per {
        flat_data.push(i as f64 / n_per as f64);
        flat_data.push(-1000.0 + i as f64 * 0.01);
        labels.push(0usize);
    }
    for i in 0..n_per {
        flat_data.push(i as f64 / n_per as f64);
        flat_data.push(1000.0 + i as f64 * 0.01);
        labels.push(1usize);
    }

    let stats = compute_cluster_stats_on_data(&flat_data, 2 * n_per, 2, &labels, 2);

    let stat0 = stats.iter().find(|s| s.cluster_id == 0).unwrap();
    assert!(stat0.significant_features[1], "translated y translated");
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

    assert_eq!(result.projections.len(), n, "translated n translated");
}

#[test]
fn tc_901_p02_kmeans_performance() {
    #[cfg(debug_assertions)]
    let (n, p) = (100, 4);
    #[cfg(not(debug_assertions))]
    let (n, p) = (50_000, 4);

    let flat_data: Vec<f64> = (0..n * p).map(|i| i as f64 / (n * p) as f64).collect();

    let result = run_kmeans_on_data(&flat_data, n, p, 4, InitStrategy::Deterministic);

    assert_eq!(result.labels.len(), n, "translated n translated");
    assert!(
        result.labels.iter().all(|&c| c < 4),
        "every label must fall in one of the 4 clusters"
    );
}

#[test]
fn tc_901_p03_cluster_stats_performance() {
    #[cfg(debug_assertions)]
    let (n, p) = (2_000, 4);
    #[cfg(not(debug_assertions))]
    let (n, p) = (50_000, 4);

    let flat_data: Vec<f64> = (0..n * p).map(|i| i as f64 / (n * p) as f64).collect();
    let labels: Vec<usize> = (0..n).map(|i| i % 4).collect();

    let stats = compute_cluster_stats_on_data(&flat_data, n, p, &labels, 4);

    assert_eq!(stats.len(), 4, "translated 4 translated");
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
