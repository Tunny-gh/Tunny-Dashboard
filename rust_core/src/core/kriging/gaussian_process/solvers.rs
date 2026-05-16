/// Cholesky decomposition: A = L · L^T.
/// Returns the lower triangular factor, or None if not positive definite.
/// Adds a jitter of 1e-6 to diagonal elements for numerical stability.
pub(super) fn cholesky(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    if n == 0 {
        return Some(vec![]);
    }
    let mat = faer::Mat::<f64>::from_fn(n, n, |i, j| if i == j { a[i][j] + 1e-6 } else { a[i][j] });
    let chol = mat.llt(faer::Side::Lower).ok()?;
    let l_ref = chol.L();
    let mut l = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in 0..=i {
            l[i][j] = l_ref[(i, j)];
        }
    }
    Some(l)
}

/// Forward substitution: solve L · x = b (L is lower triangular).
pub(super) fn forward_sub(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    if n == 0 {
        return vec![];
    }
    let l_mat = faer::Mat::<f64>::from_fn(n, n, |i, j| l[i][j]);
    let mut x = faer::Mat::<f64>::from_fn(n, 1, |i, _| b[i]);
    faer::linalg::triangular_solve::solve_lower_triangular_in_place(
        l_mat.as_ref(),
        x.as_mut(),
        faer::Par::Seq,
    );
    (0..n).map(|i| x[(i, 0)]).collect()
}

/// Backward substitution: solve L^T · x = b (L^T is upper triangular).
pub(super) fn backward_sub(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    if n == 0 {
        return vec![];
    }
    let l_mat = faer::Mat::<f64>::from_fn(n, n, |i, j| l[i][j]);
    let mut x = faer::Mat::<f64>::from_fn(n, 1, |i, _| b[i]);
    faer::linalg::triangular_solve::solve_upper_triangular_in_place(
        l_mat.transpose(),
        x.as_mut(),
        faer::Par::Seq,
    );
    (0..n).map(|i| x[(i, 0)]).collect()
}

/// Compute alpha = K^{-1} y via Cholesky factor L.
pub(super) fn compute_alpha(l: &[Vec<f64>], y: &[f64]) -> Vec<f64> {
    let v = forward_sub(l, y);
    backward_sub(l, &v)
}
