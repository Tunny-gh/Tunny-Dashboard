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
