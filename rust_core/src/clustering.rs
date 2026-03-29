//! Module documentation.
//!
//! Module documentation.
//! Design:
//! Module documentation.
//! Module documentation.
//! Module documentation.
//! Module documentation.
//!
//! Module documentation.
//! REQ-081: k-means run_kmeans() — Lloyd's algorithm
//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/tasks/tunny-dashboard-tasks.md TASK-901

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PcaSpace {
    /// Documentation.
    Param,
    /// Documentation.
    Objective,
    /// Documentation.
    All,
}

/// Documentation.
///
/// Documentation.
#[derive(Debug, Clone)]
pub struct PcaResult {
    /// Documentation.
    pub projections: Vec<Vec<f64>>,
    /// Documentation.
    pub loadings: Vec<Vec<f64>>,
    /// Documentation.
    pub explained_variance: Vec<f64>,
    /// Documentation.
    pub feature_names: Vec<String>,
}

/// Documentation.
#[derive(Debug, Clone)]
pub struct KmeansResult {
    /// Documentation.
    pub labels: Vec<usize>,
    /// Documentation.
    pub centroids: Vec<Vec<f64>>,
    /// Documentation.
    pub wcss: f64,
    /// Documentation.
    pub iterations: usize,
}

/// Documentation.
#[derive(Debug, Clone)]
pub struct ElbowResult {
    /// Documentation.
    pub wcss_per_k: Vec<f64>,
    /// Documentation.
    pub recommended_k: usize,
}

/// Documentation.
#[derive(Debug, Clone)]
pub struct ClusterStat {
    /// Documentation.
    pub cluster_id: usize,
    /// Documentation.
    pub size: usize,
    /// Documentation.
    pub centroid: Vec<f64>,
    /// Documentation.
    pub std_dev: Vec<f64>,
    /// Documentation.
    /// Documentation.
    pub significant_features: Vec<bool>,
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
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

/// Documentation.
fn center_data(data: &[Vec<f64>], means: &[f64]) -> Vec<Vec<f64>> {
    data.iter()
        .map(|row| row.iter().zip(means.iter()).map(|(&v, &m)| v - m).collect())
        .collect()
}

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
fn jacobi_eigensystem(mut a: Vec<Vec<f64>>, p: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    // Documentation.
    let mut eigvec: Vec<Vec<f64>> = (0..p)
        .map(|i| (0..p).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
        .collect();

    let max_sweeps = 100 * p * p;

    for _ in 0..max_sweeps {
        // Documentation.
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

        // Documentation.
        if max_off < 1e-12 {
            break;
        }

        // Documentation.
        let a_pp = a[pi][pi];
        let a_qq = a[qi][qi];
        let a_pq = a[pi][qi];

        let theta = if a_pq.abs() < f64::EPSILON {
            0.0
        } else {
            (a_qq - a_pp) / (2.0 * a_pq)
        };

        // Documentation.
        let t = if theta >= 0.0 {
            1.0 / (theta + (1.0 + theta * theta).sqrt())
        } else {
            -1.0 / (-theta + (1.0 + theta * theta).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt(); // cos(θ)
        let s = t * c; // sin(θ)

        // Documentation.
        a[pi][pi] = c * c * a_pp - 2.0 * s * c * a_pq + s * s * a_qq;
        a[qi][qi] = s * s * a_pp + 2.0 * s * c * a_pq + c * c * a_qq;
        a[pi][qi] = 0.0;
        a[qi][pi] = 0.0;

        // Documentation.
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

        // Documentation.
        for r in 0..p {
            let v_rp = eigvec[r][pi];
            let v_rq = eigvec[r][qi];
            eigvec[r][pi] = c * v_rp - s * v_rq;
            eigvec[r][qi] = s * v_rp + c * v_rq;
        }
    }

    // Documentation.
    let mut eigenvalues: Vec<f64> = (0..p).map(|i| a[i][i].max(0.0)).collect();

    // Documentation.
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

    // Documentation.
    eigenvalues.clear();

    (sorted_eigenvalues, sorted_eigvec)
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
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

    // Documentation.
    let means = col_means(data);
    let x_c = center_data(data, &means);

    // Documentation.
    // Documentation.
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
            let val: f64 = col_i
                .iter()
                .zip(col_j.iter())
                .map(|(a, b)| a * b)
                .sum::<f64>()
                / nf;
            cov[i][j] = val;
            cov[j][i] = val;
        }
    }

    // Documentation.
    let (eigenvalues, eigvec) = jacobi_eigensystem(cov, p);

    // Documentation.
    let loadings: Vec<Vec<f64>> = (0..k)
        .map(|comp| (0..p).map(|feat| eigvec[feat][comp]).collect())
        .collect();

    // Documentation.
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

/// Documentation.
///
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

    // Documentation.
    let get_point = |i: usize| -> &[f64] { &flat_data[i * p..(i + 1) * p] };

    // Documentation.
    let mut centroids: Vec<Vec<f64>> = Vec::with_capacity(k);

    // Documentation.
    centroids.push(get_point(n / 2).to_vec());

    // Documentation.
    for _ in 1..k {
        // Documentation.
        let mut distances: Vec<f64> = (0..n)
            .map(|i| {
                centroids
                    .iter()
                    .map(|c| sq_dist(get_point(i), c))
                    .fold(f64::INFINITY, f64::min)
            })
            .collect();

        // Documentation.
        let total: f64 = distances.iter().sum();
        if total < f64::EPSILON {
            // Documentation.
            let idx = centroids.len() % n;
            centroids.push(get_point(idx).to_vec());
            continue;
        }

        // Documentation.
        // Documentation.
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
        // Documentation.
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
            // Documentation.
            let wcss = compute_wcss(flat_data, n, p, &labels, &centroids, k);
            return KmeansResult {
                labels,
                centroids,
                wcss,
                iterations: iter + 1,
            };
        }

        // Documentation.
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
                // Documentation.
                new_centroids[c] = centroids[c].clone();
            }
        }
        centroids = new_centroids;
    }

    // Documentation.
    let wcss = compute_wcss(flat_data, n, p, &labels, &centroids, k);
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
    k: usize,
) -> f64 {
    let _ = k;
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

    // Documentation.
    let recommended_k = if wcss_per_k.len() < 3 {
        // Documentation.
        wcss_per_k.len() + 1
    } else {
        // Documentation.
        let second_diffs: Vec<f64> = (0..wcss_per_k.len() - 2)
            .map(|i| wcss_per_k[i] - 2.0 * wcss_per_k[i + 1] + wcss_per_k[i + 2])
            .collect();

        let best_idx = second_diffs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Documentation.
        // Documentation.
        best_idx + 3 // Documentation.
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
///
/// Documentation.
/// Documentation.
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

    // Documentation.
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

    // Documentation.
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

            // Documentation.
            let mut centroid = vec![0.0f64; p];
            for &i in &indices {
                for j in 0..p {
                    centroid[j] += flat_data[i * p + j];
                }
            }
            for m in &mut centroid {
                *m /= nc as f64;
            }

            // Documentation.
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

            // Documentation.
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
                    // Documentation.
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
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
pub fn run_pca(n_components: usize, space: PcaSpace) -> Option<PcaResult> {
    crate::dataframe::with_active_df(|df| {
        // Documentation.
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

        // Documentation.
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

/// Documentation.
///
/// Documentation.
pub fn compute_cluster_stats(labels: &[usize]) -> Vec<ClusterStat> {
    let Some(result) = crate::dataframe::with_active_df(|df| {
        let mut all_names = df.param_col_names().to_vec();
        all_names.extend_from_slice(df.objective_col_names());
        let n = df.row_count();
        let p = all_names.len();

        if n == 0 || p == 0 || labels.len() != n {
            return vec![];
        }

        // Documentation.
        let k = labels.iter().copied().max().map(|m| m + 1).unwrap_or(0);
        let flat_data: Vec<f64> = (0..n)
            .flat_map(|i| {
                all_names.iter().map(move |name| {
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
// Documentation.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    /// Documentation.
    fn make_dominant_axis_data(n: usize) -> Vec<Vec<f64>> {
        (0..n)
            .map(|i| {
                let x1 = i as f64 / n as f64 * 10.0; // Documentation.
                let x2 = 0.01 * (i as f64 / n as f64); // Documentation.
                vec![x1, x2]
            })
            .collect()
    }

    /// Documentation.
    fn make_clustered_data(n_per_cluster: usize, k: usize) -> Vec<f64> {
        let mut data = Vec::with_capacity(n_per_cluster * k * 2);
        for c in 0..k {
            let center = (c as f64) * 100.0; // Documentation.
            for i in 0..n_per_cluster {
                let x = center + (i as f64) * 0.01; // Documentation.
                let y = center + (i as f64) * 0.01;
                data.push(x);
                data.push(y);
            }
        }
        data
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_01_pca_dominant_axis() {
        // Documentation.
        let data = make_dominant_axis_data(200);
        let result = run_pca_on_matrix(&data, 2);

        // Documentation.
        assert_eq!(result.loadings.len(), 2, "translated 2 translated");
        assert_eq!(
            result.explained_variance.len(),
            2,
            "translated 2 translated"
        );

        // Documentation.
        assert!(
            result.explained_variance[0] > result.explained_variance[1],
            "translated1translated {} translated2translated {} translated",
            result.explained_variance[0],
            result.explained_variance[1]
        );

        // Documentation.
        let loading0 = result.loadings[0][0].abs();
        let loading1 = result.loadings[0][1].abs();
        assert!(
            loading0 > loading1,
            "translated1translated x1 translated {} translated x2 translated {} translated",
            loading0,
            loading1
        );
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_02_pca_projection_shape() {
        // Documentation.
        let n = 100;
        let data = make_dominant_axis_data(n);
        let result = run_pca_on_matrix(&data, 2);

        // Documentation.
        assert_eq!(result.projections.len(), n, "translated n translated");
        assert_eq!(result.projections[0].len(), 2, "translated 2 translated");
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_03_pca_empty_data() {
        // Documentation.
        let result = run_pca_on_matrix(&[vec![1.0, 2.0]], 2);
        assert!(result.projections.is_empty(), "n<2 translated");
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_04_kmeans_convergence() {
        // Documentation.
        let k = 3;
        let n_per_cluster = 50;
        let data = make_clustered_data(n_per_cluster, k);
        let n = n_per_cluster * k;
        let p = 2;

        let result = run_kmeans_on_data(&data, n, p, k);

        // Documentation.
        assert_eq!(result.labels.len(), n, "translated n translated");
        assert_eq!(result.centroids.len(), k, "translated k translated");

        // Documentation.
        let mut counts = vec![0usize; k];
        for &l in &result.labels {
            counts[l] += 1;
        }
        for (c, &cnt) in counts.iter().enumerate() {
            assert_eq!(
                cnt, n_per_cluster,
                "translated {} translated {} translated",
                c, n_per_cluster
            );
        }
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_05_kmeans_wcss_decreases_with_k() {
        // Documentation.
        let data = make_clustered_data(30, 4);
        let n = 120;
        let p = 2;

        let wcss_k2 = run_kmeans_on_data(&data, n, p, 2).wcss;
        let wcss_k4 = run_kmeans_on_data(&data, n, p, 4).wcss;

        // Documentation.
        assert!(
            wcss_k4 < wcss_k2,
            "k=4 translated WCSS {} translated k=2 translated WCSS {} translated",
            wcss_k4,
            wcss_k2
        );
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_06_elbow_recommended_k_valid() {
        // Documentation.
        // Documentation.
        let k_true = 3;
        let data = make_clustered_data(50, k_true);
        let n = 50 * k_true;
        let p = 2;

        let result = estimate_k_elbow_on_data(&data, n, p, 8);

        // Documentation.
        assert_eq!(
            result.wcss_per_k.len(),
            7,
            "WCSS translated 7 translated (k=2..8) translated"
        );

        // Documentation.
        assert!(
            result.recommended_k >= 2 && result.recommended_k <= 8,
            "translated k={} translatedrange [2, 8] translated",
            result.recommended_k
        );
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_07_cluster_stats_centroid() {
        // Documentation.
        // Documentation.
        let flat_data = vec![
            0.0, 1.0, // Documentation.
            2.0, 3.0, // Documentation.
            10.0, 11.0, // Documentation.
            12.0, 13.0, // Documentation.
        ];
        let labels = vec![0, 0, 1, 1];
        let stats = compute_cluster_stats_on_data(&flat_data, 4, 2, &labels, 2);

        // Documentation.
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

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_08_cluster_stats_significant() {
        // Documentation.
        // Documentation.
        let n_per = 50;
        let mut flat_data = Vec::new();
        let mut labels = Vec::new();
        for i in 0..n_per {
            flat_data.push(i as f64 / n_per as f64); // x ≈ 0.5 (mean)
            flat_data.push(-1000.0 + i as f64 * 0.01); // Documentation.
            labels.push(0usize);
        }
        for i in 0..n_per {
            flat_data.push(i as f64 / n_per as f64);
            flat_data.push(1000.0 + i as f64 * 0.01); // Documentation.
            labels.push(1usize);
        }

        let stats = compute_cluster_stats_on_data(&flat_data, 2 * n_per, 2, &labels, 2);

        // Documentation.
        let stat0 = stats.iter().find(|s| s.cluster_id == 0).unwrap();
        assert!(stat0.significant_features[1], "translated y translated");
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_p01_pca_performance() {
        // Documentation.
        // Documentation.

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

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_p02_kmeans_performance() {
        // Documentation.
        // Documentation.

        #[cfg(debug_assertions)]
        let (n, p) = (2_000, 4);
        #[cfg(not(debug_assertions))]
        let (n, p) = (50_000, 4);

        let flat_data: Vec<f64> = (0..n * p).map(|i| i as f64 / (n * p) as f64).collect();

        let start = std::time::Instant::now();
        let result = run_kmeans_on_data(&flat_data, n, p, 4);
        let elapsed = start.elapsed();

        assert_eq!(result.labels.len(), n, "translated n translated");
        assert!(
            elapsed.as_millis() < 200,
            "k-means translated 200ms translated: translated {}ms",
            elapsed.as_millis()
        );
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_901_p03_cluster_stats_performance() {
        // Documentation.

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
}
