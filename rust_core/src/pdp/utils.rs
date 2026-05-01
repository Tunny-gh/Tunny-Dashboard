/// Documentation.
///
/// Documentation.
/// Documentation.
pub(super) fn col_mean_std(data: &[f64]) -> (f64, f64) {
    let n = data.len();
    if n == 0 {
        return (0.0, 1.0);
    }
    let mean = data.iter().sum::<f64>() / n as f64;
    let var = data.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = var.sqrt();
    let std_dev = if std_dev < f64::EPSILON { 1.0 } else { std_dev };
    (mean, std_dev)
}
