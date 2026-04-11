use super::types::{PcaResult, PcaSpace};

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
    for mean in &mut means {
        *mean /= n as f64;
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
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
fn jacobi_eigensystem(mut a: Vec<Vec<f64>>, p: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    let mut eigvec: Vec<Vec<f64>> = (0..p)
        .map(|i| (0..p).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
        .collect();

    let max_sweeps = 100 * p * p;

    for _ in 0..max_sweeps {
        let mut max_off = 0.0f64;
        let mut pi = 0usize;
        let mut qi = 1usize;
        for i in 0..p {
            for j in (i + 1)..p {
                let value = a[i][j].abs();
                if value > max_off {
                    max_off = value;
                    pi = i;
                    qi = j;
                }
            }
        }

        if max_off < 1e-12 {
            break;
        }

        let a_pp = a[pi][pi];
        let a_qq = a[qi][qi];
        let a_pq = a[pi][qi];

        let theta = if a_pq.abs() < f64::EPSILON {
            0.0
        } else {
            (a_qq - a_pp) / (2.0 * a_pq)
        };

        let t = if theta >= 0.0 {
            1.0 / (theta + (1.0 + theta * theta).sqrt())
        } else {
            -1.0 / (-theta + (1.0 + theta * theta).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;

        a[pi][pi] = c * c * a_pp - 2.0 * s * c * a_pq + s * s * a_qq;
        a[qi][qi] = s * s * a_pp + 2.0 * s * c * a_pq + c * c * a_qq;
        a[pi][qi] = 0.0;
        a[qi][pi] = 0.0;

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

        for r in 0..p {
            let v_rp = eigvec[r][pi];
            let v_rq = eigvec[r][qi];
            eigvec[r][pi] = c * v_rp - s * v_rq;
            eigvec[r][qi] = s * v_rp + c * v_rq;
        }
    }

    let mut eigenvalues: Vec<f64> = (0..p).map(|i| a[i][i].max(0.0)).collect();

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

    eigenvalues.clear();

    (sorted_eigenvalues, sorted_eigvec)
}

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

    let means = col_means(data);
    let x_c = center_data(data, &means);

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
            let value: f64 = col_i
                .iter()
                .zip(col_j.iter())
                .map(|(a, b)| a * b)
                .sum::<f64>()
                / nf;
            cov[i][j] = value;
            cov[j][i] = value;
        }
    }

    let (eigenvalues, eigvec) = jacobi_eigensystem(cov, p);

    let loadings: Vec<Vec<f64>> = (0..k)
        .map(|comp| (0..p).map(|feat| eigvec[feat][comp]).collect())
        .collect();

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
pub fn run_pca(n_components: usize, space: PcaSpace) -> Option<PcaResult> {
    crate::dataframe::with_active_df(|df| {
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
