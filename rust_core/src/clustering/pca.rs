use super::types::{PcaResult, PcaSpace};

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

fn center_data(data: &[Vec<f64>], means: &[f64]) -> Vec<Vec<f64>> {
    data.iter()
        .map(|row| row.iter().zip(means.iter()).map(|(&v, &m)| v - m).collect())
        .collect()
}

/// PCA with centering only (no standardization). Compatibility wrapper for existing tests.
#[cfg(test)]
pub(crate) fn run_pca_on_matrix(data: &[Vec<f64>], n_components: usize) -> PcaResult {
    run_pca_on_matrix_opts(data, n_components, false)
}

/// The core PCA implementation. When `standardize` is true, each column is divided by its
/// standard deviation (correlation-matrix PCA), so variables with different units still
/// contribute equal weight. A zero-variance column becomes 0 after standardization and
/// contributes nothing to the components (loadings alignment is preserved).
pub(crate) fn run_pca_on_matrix_opts(
    data: &[Vec<f64>],
    n_components: usize,
    standardize: bool,
) -> PcaResult {
    let empty = PcaResult {
        projections: vec![],
        loadings: vec![],
        explained_variance: vec![],
        explained_ratio: vec![],
        feature_names: vec![],
    };

    let n = data.len();
    if n < 2 || data[0].is_empty() || n_components == 0 {
        return empty;
    }
    let p = data[0].len();
    // Reject explicitly if row lengths are inconsistent, since column access and flattening
    // would otherwise panic with an out-of-range index.
    if data.iter().any(|row| row.len() != p) {
        return empty;
    }
    let k = n_components.min(p);

    let x_c = if standardize {
        // Correlation-matrix PCA: standardization (unbiased variance with n-1, via the
        // shared clustering helper) also performs the centering.
        let mut x = data.to_vec();
        super::standardize::standardize_columns(&mut x, 1);
        x
    } else {
        center_data(data, &col_means(data))
    };

    let mut x_cols = vec![0.0f64; n * p];
    for (i, row) in x_c.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            x_cols[j * n + i] = v;
        }
    }

    let nf = (n as f64 - 1.0).max(1.0);
    let mut cov = faer::Mat::<f64>::zeros(p, p);
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
            cov[(i, j)] = value;
            cov[(j, i)] = value;
        }
    }

    // faer returns eigenvalues in nondecreasing (ascending) order
    let eig = match cov.self_adjoint_eigen(faer::Side::Lower) {
        Ok(e) => e,
        Err(_) => return empty,
    };

    let eigenvalues_raw: Vec<f64> = eig.S().column_vector().iter().copied().collect();
    let u = eig.U();

    // Sort indices descending by eigenvalue for PCA (largest variance first)
    let mut idx: Vec<usize> = (0..p).collect();
    idx.sort_by(|&i, &j| {
        eigenvalues_raw[j]
            .partial_cmp(&eigenvalues_raw[i])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let eigenvalues: Vec<f64> = idx.iter().map(|&i| eigenvalues_raw[i].max(0.0)).collect();
    let total_variance: f64 = eigenvalues.iter().sum();

    let loadings: Vec<Vec<f64>> = (0..k)
        .map(|comp| {
            let col_idx = idx[comp];
            (0..p).map(|feat| u[(feat, col_idx)]).collect()
        })
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

    let explained_ratio: Vec<f64> = eigenvalues[..k]
        .iter()
        .map(|&ev| {
            if total_variance > 0.0 {
                ev / total_variance
            } else {
                0.0
            }
        })
        .collect();

    PcaResult {
        projections,
        loadings,
        explained_variance: eigenvalues[..k].to_vec(),
        explained_ratio,
        feature_names: vec![],
    }
}

/// Standardized PCA for biplots (correlation-matrix PCA).
/// Standardization is required when mixing parameters and objectives with different units,
/// so this is offered as a separate public entry point from `run_pca` (centering only).
pub fn run_pca_standardized(n_components: usize, space: PcaSpace) -> Option<PcaResult> {
    run_pca_impl(n_components, space, true)
}

pub fn run_pca(n_components: usize, space: PcaSpace) -> Option<PcaResult> {
    run_pca_impl(n_components, space, false)
}

fn run_pca_impl(n_components: usize, space: PcaSpace, standardize: bool) -> Option<PcaResult> {
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

        let mut result = run_pca_on_matrix_opts(&data, n_components, standardize);
        result.feature_names = feature_names;
        Some(result)
    })
    .flatten()
}
