/// Cholesky decomposition: A = L · L^T.
pub(super) fn cholesky(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    crate::core::math::linear_algebra::cholesky(a)
}

/// Forward substitution: solve L · x = b.
pub(super) fn forward_sub(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    crate::core::math::linear_algebra::forward_sub(l, b)
}

/// Backward substitution: solve L^T · x = b.
pub(super) fn backward_sub(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    crate::core::math::linear_algebra::backward_sub(l, b)
}

/// Compute alpha = K^{-1} y via Cholesky factor L.
pub(super) fn compute_alpha(l: &[Vec<f64>], y: &[f64]) -> Vec<f64> {
    let v = forward_sub(l, y);
    backward_sub(l, &v)
}
