use super::types::ClusterStat;

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

    let mut global_mean = vec![0.0f64; p];
    let mut global_var = vec![0.0f64; p];
    for i in 0..n {
        for j in 0..p {
            global_mean[j] += flat_data[i * p + j];
        }
    }
    for mean in &mut global_mean {
        *mean /= n as f64;
    }
    for i in 0..n {
        for j in 0..p {
            global_var[j] += (flat_data[i * p + j] - global_mean[j]).powi(2);
        }
    }
    for var in &mut global_var {
        *var /= (n as f64 - 1.0).max(1.0);
    }

    (0..k)
        .map(|cluster_id| {
            let indices: Vec<usize> = (0..n).filter(|&i| labels[i] == cluster_id).collect();
            let nc = indices.len();
            if nc == 0 {
                return ClusterStat {
                    cluster_id,
                    size: 0,
                    centroid: global_mean.clone(),
                    std_dev: vec![0.0; p],
                    significant_features: vec![false; p],
                };
            }

            let mut centroid = vec![0.0f64; p];
            for &i in &indices {
                for j in 0..p {
                    centroid[j] += flat_data[i * p + j];
                }
            }
            for mean in &mut centroid {
                *mean /= nc as f64;
            }

            let mut var_c = vec![0.0f64; p];
            for &i in &indices {
                for j in 0..p {
                    var_c[j] += (flat_data[i * p + j] - centroid[j]).powi(2);
                }
            }
            let nc_f = nc as f64;
            for var in &mut var_c {
                *var /= (nc_f - 1.0).max(1.0);
            }
            let std_dev: Vec<f64> = var_c.iter().map(|&v| v.sqrt()).collect();

            let n_f = n as f64;
            let significant_features: Vec<bool> = (0..p)
                .map(|j| {
                    let diff = (centroid[j] - global_mean[j]).abs();
                    let se = (var_c[j] / nc_f + global_var[j] / n_f).sqrt();
                    if se < f64::EPSILON {
                        return false;
                    }
                    let t = diff / se;
                    t > 3.0
                })
                .collect();

            ClusterStat {
                cluster_id,
                size: nc,
                centroid,
                std_dev,
                significant_features,
            }
        })
        .collect()
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
