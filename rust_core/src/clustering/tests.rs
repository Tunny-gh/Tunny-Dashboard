use super::*;

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

    let start = std::time::Instant::now();
    let result = run_pca_on_matrix(&data, 2);
    let elapsed = start.elapsed();

    assert_eq!(result.projections.len(), n, "translated n translated");
    assert!(
        elapsed.as_millis() < 50,
        "PCA translated 50ms translated: translated {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn tc_901_p02_kmeans_performance() {
    #[cfg(debug_assertions)]
    let (n, p) = (2_000, 4);
    #[cfg(not(debug_assertions))]
    let (n, p) = (50_000, 4);

    let flat_data: Vec<f64> = (0..n * p).map(|i| i as f64 / (n * p) as f64).collect();

    let start = std::time::Instant::now();
    let result = run_kmeans_on_data(&flat_data, n, p, 4, InitStrategy::Deterministic);
    let elapsed = start.elapsed();

    assert_eq!(result.labels.len(), n, "translated n translated");
    assert!(
        elapsed.as_millis() < 200,
        "k-means translated 200ms translated: translated {}ms",
        elapsed.as_millis()
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

    let start = std::time::Instant::now();
    let stats = compute_cluster_stats_on_data(&flat_data, n, p, &labels, 4);
    let elapsed = start.elapsed();

    assert_eq!(stats.len(), 4, "translated 4 translated");
    assert!(
        elapsed.as_millis() < 150,
        "translated 150ms translated: translated {}ms",
        elapsed.as_millis()
    );
}
