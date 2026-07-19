//! Self-Organizing Map (SOM, batch learning).
//!
//! Trains a batch SOM on a rectangular grid over the standardized feature
//! space, providing the U-matrix, hit counts, and component planes (in
//! original units). Initialization is a deterministic linear initialization
//! along the first and second PCA principal-component planes, giving a
//! reproducible map independent of any seed. See theory/{en,ja}/clustering/som.md
//! for the theoretical background.

use super::pca::run_pca_on_matrix_opts;

/// Training spec for the SOM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SomSpec {
    /// Grid width (number of node columns).
    pub grid_w: usize,
    /// Grid height (number of node rows).
    pub grid_h: usize,
    /// Number of batch epochs.
    pub n_epochs: usize,
}

impl Default for SomSpec {
    fn default() -> Self {
        Self {
            grid_w: 8,
            grid_h: 8,
            n_epochs: 20,
        }
    }
}

/// Training result of the SOM. Nodes are stored in row-major order
/// (node = y * grid_w + x).
#[derive(Debug, Clone)]
pub struct SomResult {
    pub grid_w: usize,
    pub grid_h: usize,
    /// Node weights (standardized space). `weights[node][feature]`.
    pub weights: Vec<Vec<f64>>,
    /// U-matrix: average distance of each node to its neighboring nodes
    /// (up/down/left/right), in standardized space.
    pub u_matrix: Vec<f64>,
    /// Number of rows for which each node became the BMU.
    pub hits: Vec<usize>,
    /// BMU node index for each data row.
    pub bmu: Vec<usize>,
    /// Column means used for standardization (for inverse-transforming component planes).
    pub feature_means: Vec<f64>,
    /// Column standard deviations used for standardization (0 for zero-variance columns).
    pub feature_stds: Vec<f64>,
}

impl SomResult {
    /// Returns the component plane for feature `j` in original units
    /// (inverse-standardizes `weights`).
    pub fn component_plane(&self, feature: usize) -> Vec<f64> {
        self.weights
            .iter()
            .map(|w| w[feature] * self.feature_stds[feature] + self.feature_means[feature])
            .collect()
    }
}

/// Maximum number of rows used within a batch-learning epoch. When exceeded,
/// weights are updated using only an evenly-spaced subsample of rows (the
/// same convention as the hierarchical clustering row cap, deterministic).
/// BMU, hits, and U-matrix are still computed over all rows, so the output
/// shape is unaffected.
pub const MAX_SOM_TRAINING_ROWS: usize = 800;

/// Trains a batch SOM. Returns `None` if there are fewer than 3 rows, 0
/// features, or the grid is smaller than 2x2.
///
/// Data is standardized internally (zero-variance columns map to 0 and do
/// not contribute to the map). The neighborhood width σ decays
/// exponentially from `max(grid_w, grid_h)/2` to 0.5 over the epochs. When
/// the row count exceeds [`MAX_SOM_TRAINING_ROWS`], only the weight update
/// within each epoch uses an evenly-spaced subsample (the result remains
/// deterministic regardless of row count).
pub fn train_som(data: &[Vec<f64>], spec: &SomSpec) -> Option<SomResult> {
    let n = data.len();
    if n < 3 || data[0].is_empty() || spec.grid_w < 2 || spec.grid_h < 2 || spec.n_epochs == 0 {
        return None;
    }
    let p = data[0].len();
    // `standardize_columns` indexes every row up to `p` (its documented
    // rectangularity precondition), so a ragged row would panic. The sibling
    // clustering entry points (`run_pca_on_matrix_opts`, `hierarchical`) reject
    // non-rectangular input here; SOM must do the same rather than crash.
    if data.iter().any(|row| row.len() != p) {
        return None;
    }
    let n_nodes = spec.grid_w * spec.grid_h;

    // ── Standardization (shared clustering helper, population variance n) ──
    let mut x: Vec<Vec<f64>> = data.to_vec();
    let (means, stds) = super::standardize::standardize_columns(&mut x, 0);

    // ── Deterministic linear initialization along the PCA plane ────────────
    // Lays out the grid over ±2√λ along the top-2 principal-component
    // directions of the standardized data. If a component is degenerate,
    // falls back to zero-vector initialization (batch updates move it right away).
    let pca = run_pca_on_matrix_opts(&x, 2, false);
    let axis = |comp: usize| -> (Vec<f64>, f64) {
        let dir = pca
            .loadings
            .get(comp)
            .cloned()
            .unwrap_or_else(|| vec![0.0; p]);
        let scale = pca
            .explained_variance
            .get(comp)
            .copied()
            .unwrap_or(0.0)
            .max(0.0)
            .sqrt();
        (dir, scale)
    };
    let (dir1, s1) = axis(0);
    let (dir2, s2) = axis(1);
    let mut weights: Vec<Vec<f64>> = Vec::with_capacity(n_nodes);
    for gy in 0..spec.grid_h {
        for gx in 0..spec.grid_w {
            let a = if spec.grid_w > 1 {
                (gx as f64 / (spec.grid_w - 1) as f64) * 4.0 - 2.0
            } else {
                0.0
            };
            let b = if spec.grid_h > 1 {
                (gy as f64 / (spec.grid_h - 1) as f64) * 4.0 - 2.0
            } else {
                0.0
            };
            weights.push(
                (0..p)
                    .map(|j| a * s1 * dir1[j] + b * s2 * dir2[j])
                    .collect(),
            );
        }
    }

    let d2 =
        |a: &[f64], b: &[f64]| -> f64 { a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum() };
    let node_xy =
        |node: usize| -> (f64, f64) { ((node % spec.grid_w) as f64, (node / spec.grid_w) as f64) };
    let find_bmu = |weights: &[Vec<f64>], row: &[f64]| -> usize {
        let mut best = 0;
        let mut best_d = f64::INFINITY;
        for (i, w) in weights.iter().enumerate() {
            let d = d2(w, row);
            if d < best_d {
                best_d = d;
                best = i;
            }
        }
        best
    };

    // ── Subsample of training rows (evenly-spaced, deterministic) ──────────
    // Uses all rows if within the row cap (same behavior as before).
    let train_indices: Vec<usize> = if n > MAX_SOM_TRAINING_ROWS {
        let step = n as f64 / MAX_SOM_TRAINING_ROWS as f64;
        (0..MAX_SOM_TRAINING_ROWS)
            .map(|i| ((i as f64 * step) as usize).min(n - 1))
            .collect()
    } else {
        (0..n).collect()
    };

    // ── Batch learning ──────────────────────────────────────────────────
    let sigma0 = (spec.grid_w.max(spec.grid_h)) as f64 / 2.0;
    let sigma_end = 0.5f64;
    for epoch in 0..spec.n_epochs {
        let t = epoch as f64 / (spec.n_epochs.max(2) - 1) as f64;
        let sigma = sigma0 * (sigma_end / sigma0).powf(t);
        let two_sigma2 = 2.0 * sigma * sigma;

        // Determine the BMU for each row and take a weighted average using the neighborhood kernel.
        let mut num = vec![vec![0.0f64; p]; n_nodes];
        let mut den = vec![0.0f64; n_nodes];
        for row in train_indices.iter().map(|&ri| &x[ri]) {
            let bmu = find_bmu(&weights, row);
            let (bx, by) = node_xy(bmu);
            for node in 0..n_nodes {
                let (nx, ny) = node_xy(node);
                let g2 = (nx - bx).powi(2) + (ny - by).powi(2);
                let h = (-g2 / two_sigma2).exp();
                if h < 1e-6 {
                    continue;
                }
                for j in 0..p {
                    num[node][j] += h * row[j];
                }
                den[node] += h;
            }
        }
        for node in 0..n_nodes {
            if den[node] > 1e-12 {
                for j in 0..p {
                    weights[node][j] = num[node][j] / den[node];
                }
            }
        }
    }

    // ── BMU, hits, U-matrix ──────────────────────────────────────────────
    let bmu: Vec<usize> = x.iter().map(|row| find_bmu(&weights, row)).collect();
    let mut hits = vec![0usize; n_nodes];
    for &b in &bmu {
        hits[b] += 1;
    }
    let mut u_matrix = vec![0.0f64; n_nodes];
    for node in 0..n_nodes {
        let (gx, gy) = (node % spec.grid_w, node / spec.grid_w);
        let mut sum = 0.0;
        let mut count = 0usize;
        let mut push = |ox: isize, oy: isize| {
            let (nx, ny) = (gx as isize + ox, gy as isize + oy);
            if nx >= 0 && ny >= 0 && (nx as usize) < spec.grid_w && (ny as usize) < spec.grid_h {
                let neighbor = ny as usize * spec.grid_w + nx as usize;
                sum += d2(&weights[node], &weights[neighbor]).sqrt();
                count += 1;
            }
        };
        push(-1, 0);
        push(1, 0);
        push(0, -1);
        push(0, 1);
        u_matrix[node] = if count > 0 { sum / count as f64 } else { 0.0 };
    }

    Some(SomResult {
        grid_w: spec.grid_w,
        grid_h: spec.grid_h,
        weights,
        u_matrix,
        hits,
        bmu,
        feature_means: means,
        feature_stds: stds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blobs() -> Vec<Vec<f64>> {
        let mut data = Vec::new();
        for i in 0..15 {
            data.push(vec![i as f64 * 0.01, 0.0]);
            data.push(vec![10.0 + i as f64 * 0.01, 5.0]);
        }
        data
    }

    #[test]
    fn shapes_are_consistent() {
        let r = train_som(&blobs(), &SomSpec::default()).unwrap();
        assert_eq!(r.weights.len(), 64);
        assert_eq!(r.u_matrix.len(), 64);
        assert_eq!(r.hits.len(), 64);
        assert_eq!(r.bmu.len(), 30);
        assert_eq!(r.hits.iter().sum::<usize>(), 30);
        assert_eq!(r.component_plane(0).len(), 64);
    }

    #[test]
    fn deterministic_without_seed() {
        // PCA initialization + batch updates are fully deterministic.
        let a = train_som(&blobs(), &SomSpec::default()).unwrap();
        let b = train_som(&blobs(), &SomSpec::default()).unwrap();
        assert_eq!(a.bmu, b.bmu);
        assert_eq!(a.u_matrix, b.u_matrix);
    }

    #[test]
    fn separated_blobs_map_to_different_nodes() {
        let r = train_som(&blobs(), &SomSpec::default()).unwrap();
        // The BMU sets of the two clusters do not overlap (wiring check only; map quality is not assessed).
        let set_a: std::collections::HashSet<usize> = r.bmu.iter().step_by(2).copied().collect();
        let set_b: std::collections::HashSet<usize> =
            r.bmu.iter().skip(1).step_by(2).copied().collect();
        assert!(set_a.is_disjoint(&set_b), "{set_a:?} vs {set_b:?}");
    }

    #[test]
    fn component_plane_is_in_original_units() {
        let r = train_som(&blobs(), &SomSpec::default()).unwrap();
        let plane = r.component_plane(0);
        // Original units: x0 has two groups near 0 and near 10 → the plane's range should reach that scale.
        let max = plane.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(max > 1.0, "destandardized plane should reach data scale");
    }

    #[test]
    fn caps_training_rows_but_outputs_full_shapes() {
        // Even beyond the training cap, BMU/hits are returned for all rows and determinism is preserved.
        let n = MAX_SOM_TRAINING_ROWS + 50;
        let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (i % 7) as f64]).collect();
        let spec = SomSpec {
            grid_w: 4,
            grid_h: 4,
            n_epochs: 3,
        };
        let a = train_som(&data, &spec).unwrap();
        assert_eq!(a.bmu.len(), n);
        assert_eq!(a.hits.iter().sum::<usize>(), n);
        let b = train_som(&data, &spec).unwrap();
        assert_eq!(a.bmu, b.bmu);
    }

    #[test]
    fn rejects_degenerate_input() {
        assert!(train_som(&[vec![1.0], vec![2.0]], &SomSpec::default()).is_none());
        let spec = SomSpec {
            grid_w: 1,
            ..Default::default()
        };
        assert!(train_som(&blobs(), &spec).is_none());
    }
}
