/// Armijo backtracking line search.
///
/// Returns step size α satisfying: f(x + α·d) ≤ f(x) + c₁·α·(grad^T · d).
pub(crate) fn armijo_line_search(
    f_x: f64,
    grad: &[f64],
    d: &[f64],
    f: impl Fn(&[f64]) -> f64,
    x: &[f64],
    c1: f64,
    max_iter: usize,
) -> f64 {
    let slope: f64 = grad.iter().zip(d.iter()).map(|(g, di)| g * di).sum();
    let mut alpha = 1.0;
    for _ in 0..max_iter {
        let x_new: Vec<f64> = x
            .iter()
            .zip(d.iter())
            .map(|(xi, di)| xi + alpha * di)
            .collect();
        if f(&x_new) <= f_x + c1 * alpha * slope {
            return alpha;
        }
        alpha *= 0.5;
    }
    alpha
}
