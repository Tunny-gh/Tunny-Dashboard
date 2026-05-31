use super::types::ClusterStat;

/// Returns `(global_mean, global_std)` for each column.
/// `global_std` is the sample standard deviation (Bessel's correction, denominator = n-1).
/// Returns zero-filled vecs for n == 0 or p == 0.
pub fn compute_global_stats(flat_data: &[f64], n: usize, p: usize) -> (Vec<f64>, Vec<f64>) {
    if n == 0 || p == 0 {
        return (vec![0.0; p], vec![0.0; p]);
    }
    let nf = n as f64;
    let mut global_mean = vec![0.0f64; p];
    for i in 0..n {
        for j in 0..p {
            global_mean[j] += flat_data[i * p + j];
        }
    }
    for m in &mut global_mean {
        *m /= nf;
    }
    let mut global_var = vec![0.0f64; p];
    for i in 0..n {
        for j in 0..p {
            global_var[j] += (flat_data[i * p + j] - global_mean[j]).powi(2);
        }
    }
    for v in &mut global_var {
        *v /= (nf - 1.0).max(1.0);
    }
    let global_std: Vec<f64> = global_var.iter().map(|&v| v.sqrt()).collect();
    (global_mean, global_std)
}

/// Returns per-cluster centroid and std_dev (without significance flags).
/// Empty clusters get `global_mean` as centroid, zero std_dev, and false significance.
pub fn compute_cluster_centroid_std(
    flat_data: &[f64],
    labels: &[usize],
    n: usize,
    p: usize,
    k: usize,
    global_mean: &[f64],
) -> Vec<ClusterStat> {
    (0..k)
        .map(|cluster_id| {
            let indices: Vec<usize> = (0..n).filter(|&i| labels[i] == cluster_id).collect();
            let nc = indices.len();
            if nc == 0 {
                return ClusterStat {
                    cluster_id,
                    size: 0,
                    centroid: global_mean.to_vec(),
                    std_dev: vec![0.0; p],
                    significant_features: vec![false; p],
                };
            }

            let nc_f = nc as f64;
            let mut centroid = vec![0.0f64; p];
            for &i in &indices {
                for j in 0..p {
                    centroid[j] += flat_data[i * p + j];
                }
            }
            for m in &mut centroid {
                *m /= nc_f;
            }

            let mut var_c = vec![0.0f64; p];
            for &i in &indices {
                for j in 0..p {
                    var_c[j] += (flat_data[i * p + j] - centroid[j]).powi(2);
                }
            }
            for v in &mut var_c {
                *v /= (nc_f - 1.0).max(1.0);
            }
            let std_dev: Vec<f64> = var_c.iter().map(|&v| v.sqrt()).collect();

            ClusterStat {
                cluster_id,
                size: nc,
                centroid,
                std_dev,
                significant_features: vec![false; p],
            }
        })
        .collect()
}

/// Fills in `significant_features` for each ClusterStat using a two-sample t-test.
/// A feature is significant when `|centroid[j] - global_mean[j]| / SE > 3.0`.
/// `SE = sqrt(var_c/nc + var_g/n)` where `var_g = global_std[j]²`.
pub fn compute_significant_features(
    mut cluster_stats: Vec<ClusterStat>,
    global_mean: &[f64],
    global_std: &[f64],
    n: usize,
) -> Vec<ClusterStat> {
    let n_f = n as f64;
    let p = global_mean.len();
    for stat in &mut cluster_stats {
        if stat.size == 0 {
            continue;
        }
        let nc_f = stat.size as f64;
        stat.significant_features = (0..p)
            .map(|j| {
                let var_c = stat.std_dev[j] * stat.std_dev[j];
                let var_g = global_std[j] * global_std[j];
                let diff = (stat.centroid[j] - global_mean[j]).abs();
                let se = (var_c / nc_f + var_g / n_f).sqrt();
                if se < f64::EPSILON {
                    return false;
                }
                diff / se > 3.0
            })
            .collect();
    }
    cluster_stats
}

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
    let (global_mean, global_std) = compute_global_stats(flat_data, n, p);
    let stats = compute_cluster_centroid_std(flat_data, labels, n, p, k, &global_mean);
    compute_significant_features(stats, &global_mean, &global_std, n)
}

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
