/// L-BFGS Two-loop recursion: compute search direction d = −H^{-1} · grad.
///
/// `s_hist[k]` = x_{k+1} − x_k
/// `y_hist[k]` = grad_{k+1} − grad_k
pub(crate) fn lbfgs_direction(grad: &[f64], s_hist: &[Vec<f64>], y_hist: &[Vec<f64>]) -> Vec<f64> {
    let m = s_hist.len();
    let mut q = grad.to_vec();
    let mut rho = vec![0.0; m];
    let mut alpha = vec![0.0; m];

    for i in (0..m).rev() {
        let sy: f64 = s_hist[i]
            .iter()
            .zip(y_hist[i].iter())
            .map(|(s, y)| s * y)
            .sum();
        if sy.abs() < 1e-15 {
            continue;
        }
        rho[i] = 1.0 / sy;
        alpha[i] = rho[i]
            * s_hist[i]
                .iter()
                .zip(q.iter())
                .map(|(s, qi)| s * qi)
                .sum::<f64>();
        for (qi, yi) in q.iter_mut().zip(y_hist[i].iter()) {
            *qi -= alpha[i] * yi;
        }
    }

    let gamma = if m > 0 {
        let sy: f64 = s_hist[m - 1]
            .iter()
            .zip(y_hist[m - 1].iter())
            .map(|(s, y)| s * y)
            .sum();
        let yy: f64 = y_hist[m - 1].iter().map(|y| y * y).sum();
        if yy > 1e-15 {
            sy / yy
        } else {
            1.0
        }
    } else {
        1.0
    };
    let mut r: Vec<f64> = q.iter().map(|qi| gamma * qi).collect();

    for i in 0..m {
        let yr: f64 = y_hist[i].iter().zip(r.iter()).map(|(y, ri)| y * ri).sum();
        let beta = rho[i] * yr;
        for (ri, si) in r.iter_mut().zip(s_hist[i].iter()) {
            *ri += (alpha[i] - beta) * si;
        }
    }

    r.iter_mut().for_each(|v| *v = -*v);
    r
}
