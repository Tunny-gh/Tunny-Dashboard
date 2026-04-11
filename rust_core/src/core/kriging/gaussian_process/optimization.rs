use super::likelihood::{log_marginal_likelihood, log_ml_with_gradient};

/// L-BFGS Two-loop recursion wrapper.
pub(super) fn lbfgs_direction(grad: &[f64], s_hist: &[Vec<f64>], y_hist: &[Vec<f64>]) -> Vec<f64> {
    crate::core::optimization::lbfgs_direction(grad, s_hist, y_hist)
}

/// Armijo backtracking line search wrapper.
pub(super) fn armijo_line_search(
    f_x: f64,
    grad: &[f64],
    d: &[f64],
    f: impl Fn(&[f64]) -> f64,
    x: &[f64],
    c1: f64,
    max_iter: usize,
) -> f64 {
    crate::core::optimization::armijo_line_search(f_x, grad, d, f, x, c1, max_iter)
}

/// Optimize GP hyperparameters via L-BFGS.
pub(super) fn optimize_hyperparams(
    x: &[Vec<f64>],
    y: &[f64],
    n_iter: usize,
    m_history: usize,
) -> (Vec<f64>, usize) {
    if x.is_empty() {
        return (vec![], 0);
    }
    let ndim = x[0].len();
    let mut params = vec![0.0; ndim + 2];
    params[ndim + 1] = -2.0;

    let mut s_hist: Vec<Vec<f64>> = Vec::new();
    let mut y_hist: Vec<Vec<f64>> = Vec::new();
    let mut lml_history: std::collections::VecDeque<f64> =
        std::collections::VecDeque::with_capacity(6);

    let neg_lml = |p: &[f64]| -log_marginal_likelihood(x, y, &p[..ndim], p[ndim], p[ndim + 1]);

    let mut actual_iter = 0;
    for _ in 0..n_iter {
        let (lml, grad_raw) = log_ml_with_gradient(x, y, &params);
        actual_iter += 1;

        lml_history.push_back(lml);
        if lml_history.len() > 5 {
            lml_history.pop_front();
        }
        if lml_history.len() == 5 {
            let span = lml_history.back().unwrap() - lml_history.front().unwrap();
            if span.abs() < 1e-3 {
                break;
            }
        }

        let grad_neg: Vec<f64> = grad_raw.iter().map(|grad| -grad).collect();
        let grad_norm: f64 = grad_neg.iter().map(|grad| grad * grad).sum::<f64>().sqrt();
        if grad_norm < 1e-5 {
            break;
        }

        let direction = lbfgs_direction(&grad_neg, &s_hist, &y_hist);
        let f_x = -lml;
        let alpha = armijo_line_search(f_x, &grad_neg, &direction, neg_lml, &params, 1e-4, 20);

        let x_new: Vec<f64> = params
            .iter()
            .zip(direction.iter())
            .map(|(param, step)| param + alpha * step)
            .collect();

        let (_, grad_new_raw) = log_ml_with_gradient(x, y, &x_new);
        let grad_new: Vec<f64> = grad_new_raw.iter().map(|grad| -grad).collect();

        let s: Vec<f64> = x_new
            .iter()
            .zip(params.iter())
            .map(|(new_param, old_param)| new_param - old_param)
            .collect();
        let yv: Vec<f64> = grad_new
            .iter()
            .zip(grad_neg.iter())
            .map(|(new_grad, old_grad)| new_grad - old_grad)
            .collect();

        params = x_new;

        if s_hist.len() >= m_history {
            s_hist.remove(0);
            y_hist.remove(0);
        }
        s_hist.push(s);
        y_hist.push(yv);
    }

    (params, actual_iter)
}
